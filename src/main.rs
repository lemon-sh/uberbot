#![allow(clippy::match_wildcard_for_single_variants)]

use std::fs::File;
use std::io::Read;
use std::sync::Arc;
use std::thread;
use std::env;
use std::fmt::Display;

use futures_util::stream::StreamExt;
use irc::client::prelude::Config;
use irc::client::{Client, ClientStream};
use irc::proto::{ChannelExt, Command, Prefix};
use rspotify::Credentials;
use serde::Deserialize;
use tokio::select;
use tokio::sync::broadcast;
use tokio::sync::mpsc::unbounded_channel;
use tracing_subscriber::EnvFilter;
use crate::bot::Bot;
use crate::bots::misc::Waifu;

use crate::config::UberConfig;
use crate::database::{DbExecutor, ExecutorConnection};

mod bots;
mod database;
mod bot;
mod config;

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

pub struct AppState<SF: FnMut(String, String) -> anyhow::Result<()>> {
    client: Arc<Client>,
    stream: ClientStream,
    bot: Bot<SF>
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

    let (db_exec, db_conn) =
        DbExecutor::create(cfg.db_path.as_deref().unwrap_or("uberbot.db3"))?;
    let exec_thread = thread::spawn(move || {
        db_exec.run();
        tracing::info!("Database executor has been shut down");
    });

    let uber_ver = concat!("Überbot ", env!("CARGO_PKG_VERSION"));
    let irc_config = Config {
        nickname: Some(
            cfg
                .irc
                .nickname
                .unwrap_or_else(|| cfg.irc.username.clone()),
        ),
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

    let mut bot = Bot::new(cfg.irc.prefix, db_conn, {
        let client = client.clone();
        move |target, msg| Ok(client.send_privmsg(target, msg)?)
    });

    bot.add_command("waifu".into(), Waifu);

    let state = AppState {
        client: client.clone(),
        stream,
        bot
    };
    let message_loop_task = tokio::spawn(async move {
        if let Err(e) = message_loop(state).await {
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
    tracing::info!("Executor thread finished");
    tracing::info!("Shutdown complete!");

    Ok(())
}

async fn message_loop<SF: FnMut(String, String) -> anyhow::Result<()>>(mut state: AppState<SF>) -> anyhow::Result<()> {
    while let Some(message) = state.stream.next().await.transpose()? {
        if let Command::PRIVMSG(ref origin, content) = message.command {
            if origin.is_channel_name() {
                if let Some(author) = message.prefix.as_ref().and_then(|p| match p {
                    Prefix::Nickname(name, _, _) => Some(&name[..]),
                    _ => None,
                }) {
                    if let Err(e) = state.bot.handle_message(origin, author, &content).await {
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


