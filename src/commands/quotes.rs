use crate::bot::{Command, Context};
use crate::database::Quote;
use async_trait::async_trait;
use std::fmt::Write;

pub struct Grab;
pub struct Quot;

pub struct Search {
    limit: usize
}

impl Search {
    pub fn new(limit: usize) -> Self {
        Self { limit }
    }
}

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
        let messages = msg.history.last_msgs(author, count).await;
        if let Some(messages) = messages {
            let message = messages.join(" | ");
            msg.db
                .add_quote(Quote {
                    author: author.into(),
                    quote: message,
                })
                .await?;
            Ok(format!("Quote added ({} messages).", messages.len()))
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

#[async_trait]
impl Command for Search {
    async fn execute(&mut self, msg: Context<'_>) -> anyhow::Result<String> {
        let query = if let Some(c) = msg.content {
            c
        } else {
            return Ok("Invalid usage.".into());
        };
        let results = msg.db.search_quotes(query.into(), self.limit).await?;
        if results.is_empty() {
            return Ok("No results.".into());
        }
        let mut buf = format!("{}/{} results:\r\n", results.len(), self.limit);
        for q in results {
            write!(buf, "\"{}\" ~{}\r\n", q.quote, q.author)?;
        }
        Ok(buf)
    }
}