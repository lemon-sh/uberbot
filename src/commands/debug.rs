use async_trait::async_trait;
use crate::bot::{Command, Message};

pub struct LastMsg;

#[async_trait]
impl Command for LastMsg {
    //noinspection RsNeedlessLifetimes
    async fn execute<'a>(&mut self, msg: Message<'a>) -> anyhow::Result<String> {
        let nick = msg.content.unwrap_or(msg.author);
        let lastmsg = msg.last_msg.read().await;
        Ok(format!("{}: {:?}", nick, lastmsg.get(nick)))
    }
}