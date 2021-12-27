use std::{env, collections::HashMap};

use futures::stream::StreamExt;
use irc::{
    client::{prelude::Config, Client, ClientStream},
    proto::{Command, Message, Prefix},
};
use rspotify::Credentials;
use titlebot::Titlebot;
use tokio::select;
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;

mod waifu;
mod titlebot;

const HELP: &str = concat!(
    "a",
    "b"
);

#[cfg(unix)]
async fn terminate_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    let mut sigint = signal(SignalKind::interrupt()).unwrap();
    select! {
        _ = sigterm.recv() => break,
        _ = sigint.recv() => break
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
    titlebot: Titlebot
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_env("UBERBOT_LOG"))
        .init();
    let mut config =
        Config::load(env::var("UBERBOT_CONFIG").unwrap_or_else(|_| "uberbot.toml".to_owned()))?;
    let prefix = config.options.remove("prefix").unwrap_or("!".into());
    let spotify_cred_options = (config.options.remove("spotify_client_id"), config.options.remove("spotify_client_secret"));
    let spotify_creds = if let (Some(id), Some(sec)) = spotify_cred_options {
        Credentials::new(id.as_str(), sec.as_str())
    } else {
        return Err(anyhow::anyhow!("Config doesn't contain Spotify credentials."))
    };

    let mut client = Client::from_config(config).await?;
    client.identify()?;
    let stream = client.stream()?;

    let state = AppState {
        prefix, client,
        last_msgs: HashMap::new(),
        titlebot: Titlebot::create(spotify_creds).await?
    };

    if let Err(e) = message_loop(stream, state).await {
        error!("Error in message loop: {}", e);
    }

    info!("Shutting down");

    Ok(())
}

async fn message_loop(
    mut stream: ClientStream,
    mut state: AppState
) -> anyhow::Result<()> {
    loop {
        select! {
            r = stream.next() => {
                if let Some(message) = r.transpose()? {
                    debug!("{}", message.to_string().trim_end());

                    if let Err(e) = handle_message(&mut state, message).await {
                        warn!("Error in message handler: {}", e);
                    }
                } else {
                    break
                }
            },
            _ = terminate_signal() => {
                info!("Sending QUIT message");
                state.client.send_quit("überbot shutting down")?;
            }
        }
    }
    Ok(())
}

async fn handle_message(state: &mut AppState, msg: Message) -> anyhow::Result<()> {
    // change this to a match when more commands are handled
    if let Command::PRIVMSG(target, content) = &msg.command {
        let target = msg.response_target().unwrap_or(target);
        let author = if let Some(Prefix::Nickname(ref nick, _, _)) = msg.prefix {
            Some(nick.as_str())
        } else {
            None
        };
        if let Err(e) = handle_privmsg(state, author, target, content).await {
            state.client.send_privmsg(target, format!("Error: {}", e))?;
        }
    }
    Ok(())
}

async fn handle_privmsg(
    state: &mut AppState,
    author: Option<&str>,
    target: &str,
    content: &String
) -> anyhow::Result<()> {
    if !content.starts_with(state.prefix.as_str()) {
        if let Some(author) = author {
            state.last_msgs.insert(author.to_string(), content.clone());
        }
        if let Some(titlebot_msg) = state.titlebot.resolve(content).await? {
            state.client.send_privmsg(target, titlebot_msg)?;
        }
        return Ok(());
    }
    let content = content.trim();
    let space_index = content.find(' ');
    let (command, remainder) = if let Some(o) = space_index {
        (&content[state.prefix.len()..o], Some(&content[o + 1..]))
    } else {
        (&content[state.prefix.len()..], None)
    };
    debug!("Command received ({}; {:?})", command, remainder);

    match command {
        "help" => {
            state.client.send_privmsg(target, HELP)?;
        }
        "waifu" => {
            let category = remainder.unwrap_or("waifu");
            let url = waifu::get_waifu_pic(category).await?;
            let response = url.as_ref().map(|v| v.as_str())
                .unwrap_or("Invalid category. Valid categories: https://waifu.pics/docs");
            state.client.send_privmsg(target, response)?;
        }
        _ => {
            state.client.send_privmsg(target, "Unknown command")?;
        }
    }
    Ok(())
}
