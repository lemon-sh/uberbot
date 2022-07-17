use crate::bot::{Command, Context};
use async_trait::async_trait;

pub struct LastMsg;

#[async_trait]
impl Command for LastMsg {
    async fn execute(&mut self, msg: Context<'_>) -> anyhow::Result<String> {
        let nick = msg.content.unwrap_or(msg.author);
        let lastmsg = msg.history.read().await;
        Ok(format!("{}: {:?}", nick, lastmsg.get(nick)))
    }
}
