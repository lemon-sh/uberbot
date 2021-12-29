use async_circe::{commands::Command, Client, Config};
use bots::title::Titlebot;
use bots::weeb;
use rspotify::Credentials;
use serde::Deserialize;
use std::fs::File;
use std::io::Read;
use std::{collections::HashMap, env};
use tokio::select;
use tracing_subscriber::EnvFilter;

mod bots;

const HELP: &str = concat!(
    "=- \x1d\x02Ü\x02berbot\x0f ",
    env!("CARGO_PKG_VERSION"),
    " -="
);

#[cfg(unix)]
async fn terminate_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    let mut sigint = signal(SignalKind::interrupt()).unwrap();
    select! {
        _ = sigterm.recv() => return,
        _ = sigint.recv() => return
    }
}

#[cfg(windows)]
async fn terminate_signal() {
    use tokio::signal::windows::ctrl_c;
    let mut ctrlc = ctrl_c().unwrap();
    let _ = ctrlc.recv().await;
}

struct AppState {
    prefix: String,
    client: Client,
    last_msgs: HashMap<String, String>,
    titlebot: Titlebot,
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
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_env("UBERBOT_LOG"))
        .init();

    let mut file = File::open("uberbot.toml")?;
    let mut client_conf = String::new();
    file.read_to_string(&mut client_conf)?;

    let client_config: ClientConf = toml::from_str(&client_conf)?;

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
        titlebot: Titlebot::create(spotify_creds).await?,
    };

    if let Err(e) = message_loop(state).await {
        tracing::error!("Error in message loop: {}", e);
    }

    tracing::info!("Shutting down");

    Ok(())
}

async fn message_loop(mut state: AppState) -> anyhow::Result<()> {
    loop {
        select! {
            r = state.client.read() => {
                if let Ok(command) = r {
                    handle_message(&mut state, command).await?;
                }
            },
            _ = terminate_signal() => {
                tracing::info!("Sending QUIT message");
                state.client.quit(Some("überbot shutting down")).await?;
                break;
            }
        }
    }
    Ok(())
}

async fn handle_message(state: &mut AppState, command: Command) -> anyhow::Result<()> {
    // change this to a match when more commands are handled
    if let Command::PRIVMSG(nick, channel, message) = command {
        if let Err(e) = handle_privmsg(state, nick, &channel, message).await {
            state
                .client
                .privmsg(&channel, &format!("Error: {}", e))
                .await?;
        }
    }
    Ok(())
}

async fn handle_privmsg(
    state: &mut AppState,
    nick: String,
    channel: &str,
    message: String,
) -> anyhow::Result<()> {
    if !message.starts_with(state.prefix.as_str()) {
        state.last_msgs.insert(nick, message.clone());

        if let Some(titlebot_msg) = state.titlebot.resolve(&message).await? {
            state.client.privmsg(&channel, &titlebot_msg).await?;
        }
        return Ok(());
    }
    let space_index = message.find(' ');
    let (command, remainder) = if let Some(o) = space_index {
        (&message[state.prefix.len()..o], Some(&message[o + 1..]))
    } else {
        (&message[state.prefix.len()..], None)
    };
    tracing::debug!("Command received ({}; {:?})", command, remainder);

    match command {
        "help" => {
            state.client.privmsg(&channel, HELP).await?;
        }
        "waifu" => {
            let category = remainder.unwrap_or("waifu");
            let url = weeb::get_waifu_pic(category).await?;
            let response = url
                .as_ref()
                .map(|v| v.as_str())
                .unwrap_or("Invalid category. Valid categories: https://waifu.pics/docs");
            state.client.privmsg(&channel, response).await?;
        }
        "leet" => {
            let user = match remainder {
                Some(u) => match u {
                    "" => &nick,
                    _ => u
                },
                None => &nick
            }.trim();
            tracing::info!(user);
            if let Some(prev_msg) = state.last_msgs.get(user) {
                let resp = bots::leek::leetify(prev_msg);
                state.client.privmsg(&channel, &resp).await?;
            } else {
                state.client.privmsg(&channel, "No previous messages to leetify!").await?;
            }
        }
        _ => {
            state.client.privmsg(&channel, "Unknown command").await?;
        }
    }
    Ok(())
}
