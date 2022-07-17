use async_trait::async_trait;
use fancy_regex::Captures;
use crate::bot::{Context, Trigger};

pub struct Sed;

#[async_trait]
impl Trigger for Sed {
    async fn execute<'a>(&mut self, msg: Context<'a>, captures: Captures<'a>) -> anyhow::Result<String> {
        let foreign_author;
        let author = if let Some(author) = captures.name("u").map(|m| m.as_str()) {
            foreign_author = true;
            author
        } else {
            foreign_author = false;
            msg.author
        };
        let lastmsg = msg.last_msg.read().await;
        let message = if let Some(msg) = lastmsg.get(author) {
            msg
        } else {
            return Ok("No previous messages found.".into());
        };
        if let (Some(find), Some(replace)) = (captures.name("r"), captures.name("w")) {
            // TODO: karx plz add flags
            //let flags = matches.name("f").map(|m| m.as_str());
            let result = message.replace(find.as_str(), replace.as_str());
            drop(lastmsg);
            if foreign_author {
                Ok(format!("(edited by {}) <{}> {}", msg.author, author, result))
            } else {
                msg.last_msg.write().await.insert(author.into(), result.to_string());
                Ok(format!("<{}> {}", author, result))
            }
        } else {
            Ok("Invalid usage.".into())
        }
    }
}
