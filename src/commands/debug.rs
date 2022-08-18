use crate::bot::{Command, CommandContext};
use async_trait::async_trait;
use std::time::Duration;
use tokio::time::sleep;

pub struct LastMsg;

#[async_trait]
impl Command for LastMsg {
    async fn execute(&self, msg: CommandContext) -> anyhow::Result<String> {
        let nick = msg.content.unwrap_or(msg.author);
        Ok(format!(
            "{}: {:?}",
            nick,
            msg.history.last_msgs(&nick, usize::MAX).await
        ))
    }
}

pub struct Sleep;

#[async_trait]
impl Command for Sleep {
    async fn execute(&self, msg: CommandContext) -> anyhow::Result<String> {
        let duration = if let Some(o) = msg.content {
            o.parse()?
        } else {
            return Ok("Invalid usage.".to_string());
        };
        sleep(Duration::from_secs(duration)).await;
        return Ok(format!("Slept {} seconds", duration));
    }
}
