use crate::bot::{Command, Context};
use crate::database::Quote;
use async_trait::async_trait;

pub struct Grab;
pub struct Quot;

#[async_trait]
impl Command for Grab {
    async fn execute(&mut self, msg: Context<'_>) -> anyhow::Result<String> {
        let content = if let Some(c) = msg.content {
            c
        } else {
            return Ok("Invalid usage.".into());
        };
        let mut split = content.splitn(2, ' ');
        let split = (split.next().unwrap(), split.next());
        let (author, count) = if let Some(author) = split.1 {
            (author, split.0.parse::<usize>()?)
        } else {
            (split.0, 1)
        };
        if count == 0 {
            return Ok("So are you going to grab anything?".into());
        }
        if author == msg.author {
            return Ok("You can't grab yourself.".into());
        }
        let message = msg
            .history
            .last_msgs(author, count)
            .await
            .map(|v| v.join(" | "));
        if let Some(message) = message {
            msg.db
                .add_quote(Quote {
                    author: author.into(),
                    quote: message,
                })
                .await?;
            Ok("Quote added ({} messages).".into())
        } else {
            Ok("No previous messages to grab.".into())
        }
    }
}

#[async_trait]
impl Command for Quot {
    async fn execute(&mut self, msg: Context<'_>) -> anyhow::Result<String> {
        let author = msg.content.map(ToString::to_string);
        if let Some(q) = msg.db.get_quote(author).await? {
            Ok(format!("\"{}\" ~{}", q.quote, q.author))
        } else {
            Ok("No quotes found from this user.".into())
        }
    }
}
