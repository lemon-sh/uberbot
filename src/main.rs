use fancy_regex::Regex;
use std::env;
use std::fs::File;
use std::io::Read;
use std::sync::Arc;
use std::thread;

use crate::bot::Bot;
use crate::commands::eval::Eval;
use crate::commands::help::Help;
use crate::commands::leek::Owo;
use crate::commands::quotes::{Grab, Quot};
use crate::commands::sed::Sed;
use crate::commands::spotify::Spotify;
use crate::commands::title::Title;
use crate::commands::waifu::Waifu;
use futures_util::stream::StreamExt;
use irc::client::prelude::Config;
use irc::client::{Client, ClientStream};
use irc::proto::{ChannelExt, Command, Prefix};
use rspotify::Credentials;
use tokio::select;
use tokio::sync::broadcast;
use tokio::sync::mpsc::unbounded_channel;
use tracing_subscriber::EnvFilter;

use crate::config::UberConfig;
use crate::database::{DbExecutor, ExecutorConnection};

mod bot;
mod commands;
mod config;
mod database;
mod history;

#[cfg(unix)]
async fn terminate_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut sigterm = signal(SignalKind::terminate()).unwrap();
    let mut sigint = signal(SignalKind::interrupt()).unwrap();
    tracing::debug!("Installed ctrl+c handler");
    select! {
        _ = sigterm.recv() => (),
        _ = sigint.recv() => ()
    }
}

#[cfg(windows)]
async fn terminate_signal() {
    use tokio::signal::windows::ctrl_c;
    let mut ctrlc = ctrl_c().unwrap();
    tracing::debug!("Installed ctrl+c handler");
    let _ = ctrlc.recv().await;
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_env("UBERBOT_LOG"))
        .init();

    let mut file =
        File::open(env::var("UBERBOT_CONFIG").unwrap_or_else(|_| "uberbot.toml".to_string()))?;
    let mut client_conf = String::new();
    file.read_to_string(&mut client_conf)?;

    let cfg: UberConfig = toml::from_str(&client_conf)?;

    let (db_exec, db_conn) = DbExecutor::create(cfg.db_path.as_deref().unwrap_or("uberbot.db3"))?;
    let exec_thread = thread::spawn(move || db_exec.run());

    let uber_ver = concat!("Überbot ", env!("CARGO_PKG_VERSION"));
    let irc_config = Config {
        nickname: Some(cfg.irc.nickname.unwrap_or_else(|| cfg.irc.username.clone())),
        username: Some(cfg.irc.username.clone()),
        realname: Some(cfg.irc.username),
        server: Some(cfg.irc.host),
        port: Some(cfg.irc.port),
        use_tls: Some(cfg.irc.tls),
        channels: cfg.irc.channels,
        umodes: cfg.irc.mode,
        user_info: Some(uber_ver.into()),
        version: Some(uber_ver.into()),
        ..Config::default()
    };
    let mut client = Client::from_config(irc_config).await?;
    let stream = client.stream()?;
    client.identify()?;
    let client = Arc::new(client);

    let (ctx, _) = broadcast::channel(1);
    let (etx, mut erx) = unbounded_channel();

    let mut bot = Bot::new(cfg.irc.prefix, db_conn, 3, {
        let client = client.clone();
        move |target, msg| Ok(client.send_privmsg(target, msg)?)
    });

    bot.add_command("help".into(), Help);
    bot.add_command("waifu".into(), Waifu::default());
    bot.add_command("owo".into(), Owo);
    bot.add_command("ev".into(), Eval::default());
    bot.add_command("grab".into(), Grab);
    bot.add_command("quot".into(), Quot);
    bot.add_trigger(
        Regex::new(r"^(?:(?<u>\S+):\s+)?s/(?<r>[^/]*)/(?<w>[^/]*)(?:/(?<f>[a-z]*))?\s*")?,
        Sed,
    );
    if let Some(spotcfg) = cfg.spotify {
        let creds = Credentials::new(&spotcfg.client_id, &spotcfg.client_secret);
        let spotify = Spotify::new(creds).await?;
        bot.add_trigger(Regex::new(r"(?:https?|spotify):(?://open\.spotify\.com/)?(track|artist|album|playlist)[/:]([a-zA-Z0-9]*)")?, spotify);
    } else {
        tracing::warn!("Spotify module is disabled, because the config is missing")
    }
    bot.add_trigger(Regex::new(r"https?://[^\s/$.?#].\S*")?, Title::new()?);
    #[cfg(feature = "debug")]
    {
        use commands::debug::*;
        bot.add_command("lastmsg".into(), LastMsg);
    }

    let message_loop_task = tokio::spawn(async move {
        if let Err(e) = message_loop(stream, bot).await {
            let _err = etx.send(e);
        }
    });

    select! {
        _ = terminate_signal() => {
            tracing::info!("Received shutdown signal, sending QUIT message");
            client.send_quit("überbot shutting down")?;
        }
        e = erx.recv() => {
            if let Some(e) = e {
                tracing::error!("An error has occurred, shutting down: {}", e);
            } else {
                tracing::error!("Error channel has been dropped due to an unknown error, shutting down");
            }
        }
    }

    tracing::info!("Closing services...");
    let _ = ctx.send(());
    message_loop_task
        .await
        .unwrap_or_else(|e| tracing::warn!("Couldn't join the web service: {:?}", e));
    tracing::info!("Message loop finished");
    exec_thread
        .join()
        .unwrap_or_else(|e| tracing::warn!("Couldn't join the database: {:?}", e));
    tracing::info!("DB Executor thread finished");
    tracing::info!("Shutdown complete!");

    Ok(())
}

async fn message_loop<SF: Fn(String, String) -> anyhow::Result<()>>(
    mut stream: ClientStream,
    bot: Bot<SF>,
) -> anyhow::Result<()> {
    while let Some(message) = stream.next().await.transpose()? {
        if let Command::PRIVMSG(ref origin, content) = message.command {
            if origin.is_channel_name() {
                if let Some(author) = message.prefix.as_ref().and_then(|p| match p {
                    Prefix::Nickname(name, _, _) => Some(&name[..]),
                    _ => None,
                }) {
                    bot.handle_message(origin, author, &content).await
                } else {
                    tracing::warn!("Couldn't get the author for a message");
                }
            }
        }
    }
    Ok(())
}
