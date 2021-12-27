use std::env;

use futures::stream::StreamExt;
use irc::{
    client::{prelude::Config, Client, ClientStream},
    proto::{Command, Message},
};
use tokio::select;
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;

mod waifu;

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

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_env("UBERBOT_LOG"))
        .init();
    let mut config =
        Config::load(env::var("UBERBOT_CONFIG").unwrap_or_else(|_| "uberbot.toml".to_owned()))?;
    let prefix = config.options.remove("prefix").unwrap_or("!".into());
    let prefix = prefix.as_str();

    let mut client = Client::from_config(config).await?;
    client.identify()?;
    let stream = client.stream()?;

    if let Err(e) = message_loop(stream, prefix, client).await {
        error!("Error in message loop: {}", e);
    }

    info!("Shutting down");

    Ok(())
}

async fn message_loop(
    mut stream: ClientStream,
    prefix: &str,
    client: Client,
) -> anyhow::Result<()> {
    loop {
        select! {
            r = stream.next() => {
                if let Some(message) = r.transpose()? {
                    debug!("{}", message.to_string().trim_end());

                    if let Err(e) = handle_message(message, prefix, &client).await {
                        warn!("Error in message handler: {}", e);
                    }
                } else {
                    break
                }
            },
            _ = terminate_signal() => {
                info!("Sending QUIT message");
                client.send_quit("überbot shutting down")?;
            }
        }
    }
    Ok(())
}

async fn handle_message(msg: Message, prefix: &str, client: &Client) -> anyhow::Result<()> {
    if let Command::PRIVMSG(target, content) = &msg.command {
        let target = msg.response_target().unwrap_or(target);
        if let Err(e) = handle_privmsg(target, content, prefix, client).await {
            client.send_privmsg(target, format!("Error: {}", e))?;
        }
    }
    Ok(())
}

async fn handle_privmsg(
    target: &str,
    content: &String,
    prefix: &str,
    client: &Client,
) -> anyhow::Result<()> {
    if !content.starts_with(prefix) {
        return Ok(());
    }
    let content = content.trim();
    let space_index = content.find(' ');
    let (command, remainder) = if let Some(o) = space_index {
        (&content[prefix.len()..o], Some(&content[o + 1..]))
    } else {
        (&content[prefix.len()..], None)
    };
    debug!("Command received ({}; {:?})", command, remainder);

    match command {
        "waifu" => {
            let category = remainder.unwrap_or("waifu");
            let url = waifu::get_waifu_pic(category).await?;
            let response = url.as_ref().map(|v| v.as_str())
                .unwrap_or("Invalid category. Valid categories: https://waifu.pics/docs");
            client.send_privmsg(target, response)?;
        }
        _ => {
            client.send_privmsg(target, "Unknown command")?;
        }
    }
    Ok(())
}
