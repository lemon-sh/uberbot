use std::fmt::Write;
use std::fs::File;
use std::io::Read;
use std::net::SocketAddr;
use std::thread;
use std::{collections::HashMap, env};

use arrayvec::ArrayString;
use futures_util::stream::StreamExt;
use irc::client::prelude::Config;
use irc::client::Client;
use irc::proto::{ChannelExt, Command, Prefix};
use rspotify::Credentials;
use serde::Deserialize;
use tokio::select;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tracing_log::LogTracer;
use tracing_subscriber::EnvFilter;

use crate::bots::{leek, misc, sed, title};
use crate::database::{DbExecutor, ExecutorConnection};

mod bots;
mod database;
mod web_service;

// this will be displayed when the help command is used
const HELP: &[&str] = &[
    concat!("=- \x1d\x02Ü\x02berbot\x0f ", env!("CARGO_PKG_VERSION"), " -="),
    " * waifu <category>",
    " * owo/mock/leet [user]",
    " * ev <math expression>",
    " - This bot also provides titles of URLs and details for Spotify URIs/links. It can also resolve sed expressions."
];

#[cfg(unix)]
async fn terminate_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    let mut sigint = signal(SignalKind::interrupt()).unwrap();
    tracing::debug!("Installed ctrl+c handler");
    select! {
        _ = sigterm.recv() => return,
        _ = sigint.recv() => return
    }
}

#[cfg(windows)]
async fn terminate_signal() {
    use tokio::signal::windows::ctrl_c;
    let mut ctrlc = ctrl_c().unwrap();
    tracing::debug!("Installed ctrl+c handler");
    let _ = ctrlc.recv().await;
}

pub struct AppState {
    prefix: String,
    client: Client,
    last_msgs: HashMap<String, String>,
    last_eval: HashMap<String, f64>,
    titlebot: title::Titlebot,
    db: ExecutorConnection,
    git_channel: String,
}

#[derive(Deserialize)]
struct ClientConf {
    channels: Vec<String>,
    host: String,
    tls: bool,
    mode: Option<String>,
    nickname: Option<String>,
    port: u16,
    username: String,
    spotify_client_id: String,
    spotify_client_secret: String,
    prefix: String,
    db_path: Option<String>,
    http_listen: Option<SocketAddr>,
    git_channel: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    LogTracer::init()?;
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_env("UBERBOT_LOG"))
        .init();

    let mut file =
        File::open(env::var("UBERBOT_CONFIG").unwrap_or_else(|_| "uberbot.toml".to_string()))?;
    let mut client_conf = String::new();
    file.read_to_string(&mut client_conf)?;

    let client_config: ClientConf = toml::from_str(&client_conf)?;

    let (db_exec, db_conn) =
        DbExecutor::create(client_config.db_path.as_deref().unwrap_or("uberbot.db3"))?;
    let exec_thread = thread::spawn(move || {
        db_exec.run();
        tracing::info!("Database executor has been shut down");
    });

    let spotify_creds = Credentials::new(
        &client_config.spotify_client_id,
        &client_config.spotify_client_secret,
    );

    let http_listen = client_config
        .http_listen
        .unwrap_or_else(|| SocketAddr::from(([127, 0, 0, 1], 5000)));

    let uber_ver = concat!("Überbot ", env!("CARGO_PKG_VERSION"));
    let irc_config = Config {
        nickname: client_config.nickname,
        username: Some(client_config.username.clone()),
        realname: Some(client_config.username),
        server: Some(client_config.host),
        port: Some(client_config.port),
        use_tls: Some(client_config.tls),
        channels: client_config.channels,
        umodes: client_config.mode,
        user_info: Some(uber_ver.into()),
        version: Some(uber_ver.into()),
        ..Config::default()
    };
    let client = Client::from_config(irc_config).await?;
    client.identify()?;

    let state = AppState {
        prefix: client_config.prefix,
        client,
        last_msgs: HashMap::new(),
        last_eval: HashMap::new(),
        titlebot: title::Titlebot::create(spotify_creds).await?,
        db: db_conn,
        git_channel: client_config.git_channel,
    };

    let (git_tx, git_recv) = channel(512);

    if let Err(e) = executor(state, git_tx, git_recv, http_listen).await {
        tracing::error!("Error in message loop: {}", e);
    }

    if let Err(e) = exec_thread.join() {
        tracing::error!("Error while shutting down the database: {:?}", e);
    }
    tracing::info!("Shutting down");

    Ok(())
}

async fn executor(
    mut state: AppState,
    git_tx: Sender<String>,
    mut git_recv: Receiver<String>,
    http_listen: SocketAddr,
) -> anyhow::Result<()> {
    let web_db = state.db.clone();
    select! {
        r = web_service::run(web_db, git_tx, http_listen) => r?,
        r = message_loop(&mut state) => r?,
        r = git_recv.recv() => {
            if let Some(message) = r {
                state.client.send_privmsg(&state.git_channel, &message)?;
            }
        }
        _ = terminate_signal() => {
            tracing::info!("Sending QUIT message");
            state.client.send_quit("überbot shutting down")?;
        }
    }
    Ok(())
}

async fn message_loop(state: &mut AppState) -> anyhow::Result<()> {
    let mut stream = state.client.stream()?;
    while let Some(message) = stream.next().await.transpose()? {
        if let Command::PRIVMSG(ref origin, content) = message.command {
            if origin.is_channel_name() {
                if let Some(author) = message.prefix.as_ref().and_then(|p| match p {
                    Prefix::Nickname(name, _, _) => Some(&name[..]),
                    _ => None,
                }) {
                    if let Err(e) = handle_privmsg(state, author, origin, content).await {
                        state
                            .client
                            .send_privmsg(origin, &format!("Error: {}", e))?;
                    }
                } else {
                    tracing::warn!("Couldn't get the author for a message");
                }
            }
        }
    }
    Ok(())
}

fn separate_to_space(str: &str, prefix_len: usize) -> (&str, Option<&str>) {
    if let Some(o) = str.find(' ') {
        (&str[prefix_len..o], Some(&str[o + 1..]))
    } else {
        (&str[prefix_len..], None)
    }
}

async fn handle_privmsg(
    state: &mut AppState,
    author: &str,
    origin: &str,
    content: String,
) -> anyhow::Result<()> {
    if !content.starts_with(state.prefix.as_str()) {
        if let Some(titlebot_msg) = state.titlebot.resolve(&content).await? {
            state.client.send_privmsg(origin, &titlebot_msg)?;
        }

        if let Some(prev_msg) = state.last_msgs.get(author) {
            if let Some(formatted) = sed::resolve(prev_msg, &content)? {
                let mut result = ArrayString::<512>::new();
                write!(result, "<{}> {}", author, formatted)?;
                state.client.send_privmsg(origin, &result)?;
                state.last_msgs.insert(author.into(), formatted.to_string());
                return Ok(());
            }
        }

        state.last_msgs.insert(author.into(), content);
        return Ok(());
    }
    let (command, remainder) = separate_to_space(&content, state.prefix.len());
    tracing::debug!("Command received ({:?}; {:?})", command, remainder);

    match command {
        "help" => {
            for help_line in HELP {
                state.client.send_privmsg(origin, help_line)?;
            }
        }
        "waifu" => {
            let category = remainder.unwrap_or("waifu");
            let url = misc::get_waifu_pic(category).await?;
            let response = url
                .as_ref()
                .map(|v| v.as_str())
                .unwrap_or("Invalid category. Valid categories: https://waifu.pics/docs");
            state.client.send_privmsg(origin, response)?;
        }
        "mock" => {
            leek::execute_leek(
                state,
                leek::LeekCommand::Mock,
                origin,
                remainder.unwrap_or(author),
            )?;
        }
        "leet" => {
            leek::execute_leek(
                state,
                leek::LeekCommand::Leet,
                origin,
                remainder.unwrap_or(author),
            )?;
        }
        "owo" => {
            leek::execute_leek(
                state,
                leek::LeekCommand::Owo,
                origin,
                remainder.unwrap_or(author),
            )?;
        }
        "ev" => {
            let result = misc::mathbot(author.into(), remainder, &mut state.last_eval)?;
            state.client.send_privmsg(origin, &result)?;
        }
        "grab" => {
            if let Some(target) = remainder {
                if target == author {
                    state
                        .client
                        .send_privmsg(target, "You can't grab yourself")?;
                    return Ok(());
                }
                if let Some(prev_msg) = state.last_msgs.get(target) {
                    if state.db.add_quote(prev_msg.clone(), target.into()).await {
                        state.client.send_privmsg(target, "Quote added")?;
                    } else {
                        state
                            .client
                            .send_privmsg(target, "A database error has occurred")?;
                    }
                } else {
                    state
                        .client
                        .send_privmsg(target, "No previous messages to grab")?;
                }
            } else {
                state.client.send_privmsg(origin, "No nickname to grab")?;
            }
        }
        "quot" => {
            if let Some(quote) = state.db.get_quote(remainder.map(|v| v.to_string())).await {
                let mut resp = ArrayString::<512>::new();
                write!(resp, "\"{}\" ~{}", quote.0, quote.1)?;
                state.client.send_privmsg(origin, &resp)?;
            } else {
                state.client.send_privmsg(origin, "No quotes found")?;
            }
        }
        _ => {
            state.client.send_privmsg(origin, "Unknown command")?;
        }
    }
    Ok(())
}
