mod bots;
mod database;

use crate::database::{DbExecutor, ExecutorConnection};
use arrayvec::ArrayString;
use async_circe::{commands::Command, Client, Config};
use bots::title::Titlebot;
use bots::{misc, misc::LeekCommand, sed};
use rspotify::Credentials;
use serde::Deserialize;
use std::fmt::Write;
use std::fs::File;
use std::io::Read;
use std::thread;
use std::{collections::HashMap, env};
use tokio::select;
use tracing_subscriber::EnvFilter;

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
    titlebot: Titlebot,
    db: ExecutorConnection,
}

#[derive(Deserialize)]
struct ClientConf {
    channels: Vec<String>,
    host: String,
    mode: Option<String>,
    nickname: Option<String>,
    port: u16,
    username: String,
    spotify_client_id: String,
    spotify_client_secret: String,
    prefix: String,
    db_path: Option<String>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
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

    let config = Config::runtime_config(
        client_config.channels,
        client_config.host,
        client_config.mode,
        client_config.nickname,
        client_config.port,
        client_config.username,
    );

    let mut client = Client::new(config).await?;
    client.identify().await?;

    let state = AppState {
        prefix: client_config.prefix,
        client,
        last_msgs: HashMap::new(),
        last_eval: HashMap::new(),
        titlebot: Titlebot::create(spotify_creds).await?,
        db: db_conn,
    };

    if let Err(e) = executor(state).await {
        tracing::error!("Error in message loop: {}", e);
    }

    if let Err(e) = exec_thread.join() {
        tracing::error!("Error while shutting down the database: {:?}", e);
    }
    tracing::info!("Shutting down");

    Ok(())
}

async fn executor(mut state: AppState) -> anyhow::Result<()> {
    select! {
        r = message_loop(&mut state) => r?,
        _ = terminate_signal() => {
            tracing::info!("Sending QUIT message");
            state.client.quit(Some("überbot shutting down")).await?;
        }
    }
    Ok(())
}

async fn message_loop(state: &mut AppState) -> anyhow::Result<()> {
    while let Some(cmd) = state.client.read().await? {
        if let Command::PRIVMSG(nick, channel, message) = cmd {
            if let Err(e) = handle_privmsg(state, nick, &channel, message).await {
                state
                    .client
                    .privmsg(&channel, &format!("Error: {}", e))
                    .await?;
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
    nick: String,
    channel: &str,
    message: String,
) -> anyhow::Result<()> {
    if !message.starts_with(state.prefix.as_str()) {
        if let Some(titlebot_msg) = state.titlebot.resolve(&message).await? {
            state.client.privmsg(&channel, &titlebot_msg).await?;
        }

        if let Some(prev_msg) = state.last_msgs.get(&nick) {
            if let Some(formatted) = sed::resolve(prev_msg, &message)? {
                let mut result = ArrayString::<512>::new();
                write!(result, "<{}> {}", nick, formatted)?;
                state.client.privmsg(&channel, &result).await?;
                state.last_msgs.insert(nick, formatted.to_string());
                return Ok(());
            }
        }

        state.last_msgs.insert(nick, message);
        return Ok(());
    }
    let (command, remainder) = separate_to_space(&message, state.prefix.len());
    tracing::debug!("Command received ({:?}; {:?})", command, remainder);

    match command {
        "help" => {
            for help_line in HELP {
                state.client.privmsg(&channel, help_line).await?;
            }
        }
        "waifu" => {
            let category = remainder.unwrap_or("waifu");
            let url = misc::get_waifu_pic(category).await?;
            let response = url
                .as_ref()
                .map(|v| v.as_str())
                .unwrap_or("Invalid category. Valid categories: https://waifu.pics/docs");
            state.client.privmsg(&channel, response).await?;
        }
        "mock" => {
            misc::execute_leek(
                state,
                LeekCommand::Mock,
                channel,
                remainder.unwrap_or(&nick),
            )
            .await?;
        }
        "leet" => {
            misc::execute_leek(
                state,
                LeekCommand::Leet,
                channel,
                remainder.unwrap_or(&nick),
            )
            .await?;
        }
        "owo" => {
            misc::execute_leek(state, LeekCommand::Owo, channel, remainder.unwrap_or(&nick))
                .await?;
        }
        "ev" => {
            let result = misc::mathbot(nick, remainder, &mut state.last_eval)?;
            state.client.privmsg(&channel, &result).await?;
        }
        "grab" => {
            if let Some(target) = remainder {
                if target == nick {
                    state
                        .client
                        .privmsg(&channel, "You can't grab yourself")
                        .await?;
                    return Ok(());
                }
                if let Some(prev_msg) = state.last_msgs.get(target) {
                    if state.db.add_quote(prev_msg.clone(), target.into()).await {
                        state.client.privmsg(&channel, "Quote added").await?;
                    } else {
                        state
                            .client
                            .privmsg(&channel, "A database error has occurred")
                            .await?;
                    }
                } else {
                    state
                        .client
                        .privmsg(&channel, "No previous messages to grab")
                        .await?;
                }
            } else {
                state
                    .client
                    .privmsg(&channel, "No nickname to grab")
                    .await?;
            }
        }
        "quot" => {
            if let Some(quote) = state.db.get_quote(remainder.map(|v| v.to_string())).await {
                let mut resp = ArrayString::<512>::new();
                write!(resp, "\"{}\" ~{}", quote.0, quote.1)?;
                state.client.privmsg(&channel, &resp).await?;
            } else {
                state.client.privmsg(&channel, "No quotes found").await?;
            }
        }
        _ => {
            state.client.privmsg(&channel, "Unknown command").await?;
        }
    }
    Ok(())
}
