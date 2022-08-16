#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions, clippy::too_many_lines)]

use fancy_regex::Regex;
use std::str::FromStr;
use std::sync::Arc;
use std::{env, fs};
use std::{process, thread};

use crate::bot::Bot;
use crate::commands::eval::Eval;
use crate::commands::help::Help;
use crate::commands::leek::{Leet, Mock, Owo};
use crate::commands::playground::Dbg;
use crate::commands::playground::Play;
use crate::commands::quotes::{Grab, Quot, Search, SearchNext};
use crate::commands::sed::Sed;
use crate::commands::spotify::Spotify;
use crate::commands::title::Title;
use crate::commands::waifu::Waifu;
use crate::web::HttpContext;
use futures_util::stream::StreamExt;
use irc::client::prelude::Config;
use irc::client::{Client, ClientStream};
use irc::proto::{ChannelExt, Command, Prefix};
use rspotify::Credentials;
use tokio::select;
use tokio::sync::mpsc::unbounded_channel;
use tokio::sync::{broadcast, mpsc};
use tracing::Level;

use crate::config::UberConfig;
use crate::database::{DbExecutor, ExecutorConnection};

mod bot;
mod commands;
mod config;
mod database;
mod history;
mod web;
mod util;

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
    let config_var = env::var("UBERBOT_CONFIG");
    let config_path = config_var.as_deref().unwrap_or("uberbot.toml");
    println!("Loading config from '{}'...", config_path);
    let config_str = fs::read_to_string(config_path)?;
    let cfg: UberConfig = toml::from_str(&config_str)?;

    tracing_subscriber::fmt::fmt()
        .with_max_level({
            if let Some(o) = cfg.log_level.as_deref() {
                Level::from_str(o)?
            } else {
                Level::INFO
            }
        })
        .init();

    if cfg.bot.prefixes.is_empty() {
        tracing::error!("You have to specify at least one prefix");
        process::exit(1);
    }

    let (db_exec, db_conn) =
        DbExecutor::create(cfg.bot.db_path.as_deref().unwrap_or("uberbot.db3"))?;
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

    let sf = {
        let client = client.clone();
        move |target, msg| Ok(client.send_privmsg(target, msg)?)
    };

    let http_task = cfg.web.map(|http| {
        let http_ctx = ctx.subscribe();
        let context = HttpContext {
            cfg: http,
            sendmsg: sf.clone(),
        };
        tokio::spawn(async move {
            if let Err(e) = web::run(context, http_ctx).await {
                tracing::error!("Fatal error in web service: {}", e);
            }
        })
    });
    let mut bot = Bot::new(cfg.bot.prefixes, db_conn, cfg.bot.history_depth, sf);

    bot.add_command("help".into(), Help);
    bot.add_command("waifu".into(), Waifu::default());
    bot.add_command("owo".into(), Owo);
    bot.add_command("leet".into(), Leet);
    bot.add_command("mock".into(), Mock);
    bot.add_command("ev".into(), Eval::default());
    bot.add_command("grab".into(), Grab);
    bot.add_command("quot".into(), Quot);
    let search_limit = cfg.bot.search_limit.unwrap_or(3);
    bot.add_command("qsearch".into(), Search::new(search_limit));
    bot.add_command("qnext".into(), SearchNext::new(search_limit));
    bot.add_command("play".into(), Play::default());
    bot.add_command("dbg".into(), Dbg::default());
    bot.add_trigger(
        Regex::new(r"^(?:(?<u>\S+):\s+)?s/(?<r>[^/]*)/(?<w>[^/]*)(?:/(?<f>[a-z]*))?\s*")?,
        Sed,
    );
    if let Some(spotcfg) = cfg.spotify {
        let creds = Credentials::new(&spotcfg.client_id, &spotcfg.client_secret);
        let spotify = Spotify::new(creds).await?;
        bot.add_trigger(Regex::new(r"(?:https?|spotify):(?://open\.spotify\.com/)?(track|artist|album|playlist)[/:]([a-zA-Z\d]*)")?, spotify);
    } else {
        tracing::warn!("Spotify module is disabled, because the config is missing");
    }
    bot.add_trigger(Regex::new(r"https?://[-a-zA-Z0-9@:%._+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b[-a-zA-Z0-9()@:%_+.~#?&/=]*")?, Title::new()?);
    #[cfg(feature = "debug")]
    {
        use commands::debug::*;
        bot.add_command("lastmsg".into(), LastMsg);
        bot.add_command("sleep".into(), Sleep)
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
    message_loop_task.await.unwrap();
    tracing::info!("Message loop finished");
    if let Some(t) = http_task {
        t.await.unwrap();
        tracing::info!("Web service finished");
    }
    exec_thread.join().unwrap();
    tracing::info!("DB Executor thread finished");
    tracing::info!("Shutdown complete!");

    Ok(())
}

async fn message_loop<SF>(mut stream: ClientStream, bot: Bot<SF>) -> anyhow::Result<()>
where
    SF: Fn(String, String) -> anyhow::Result<()> + Send + Sync + 'static,
{
    let (cancelled_send, mut cancelled_recv) = mpsc::channel::<()>(1);
    while let Some(message) = stream.next().await.transpose()? {
        if let Command::PRIVMSG(origin, content) = message.command {
            if origin.is_channel_name() {
                if let Some(author) = message.prefix.and_then(|p| match p {
                    Prefix::Nickname(name, _, _) => Some(name),
                    Prefix::ServerName(_) => None,
                }) {
                    let cancelled_send = cancelled_send.clone();
                    bot.handle_message(origin, author, content, cancelled_send)
                        .await;
                } else {
                    tracing::warn!("Couldn't get the author for a message");
                }
            }
        }
    }
    drop(cancelled_send);
    let _ = cancelled_recv.recv().await;
    Ok(())
}
