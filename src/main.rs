use std::env;

use irc::client::{Client, prelude::Config, ClientStream};
use futures::stream::StreamExt;
use tokio::select;
use tracing::{debug, info, error};
use tracing_subscriber::EnvFilter;

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
    tracing_subscriber::fmt().with_env_filter(EnvFilter::from_env("UBERBOT_LOG")).init();
    let config = Config::load(env::var("UBERBOT_CONFIG").unwrap_or_else(|_| "uberbot.toml".to_owned()))?;

    let mut client = Client::from_config(config).await?;
    client.identify()?;
    let stream = client.stream()?;

    select! {
        r = message_loop(stream) => {
            if let Err(e) = r {
                error!("Error in message loop: {}", e);
            }
        }
        _ = terminate_signal() => {}
    }

    info!("Shutting down...");

    Ok(())
}

async fn message_loop(mut stream: ClientStream) -> anyhow::Result<()> {
    while let Some(message) = stream.next().await.transpose()? {
        debug!("Received message: {}", message);
    }
    Ok(())
}
