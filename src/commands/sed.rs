use crate::bot::{Context, Trigger};
use async_trait::async_trait;
use fancy_regex::escape;
use fancy_regex::Captures;
use regex::RegexBuilder;

pub struct Sed;

#[async_trait]
impl Trigger for Sed {
    async fn execute<'a>(
        &mut self,
        msg: Context<'a>,
        captures: Captures<'a>,
    ) -> anyhow::Result<String> {
        let foreign_author;
        let author = if let Some(author) = captures.name("u").map(|m| m.as_str()) {
            foreign_author = true;
            author
        } else {
            foreign_author = false;
            msg.author
        };
        let message = if let Some(msg) = msg.history.last_msg(author).await {
            msg
        } else {
            return Ok("No previous messages found.".into());
        };
        if let (Some(find), Some(replace)) = (captures.name("r"), captures.name("w")) {
            let (global, ignore_case) = captures
                .name("f")
                .map(|m| m.as_str())
                .map(|s| (s.contains('g'), s.contains('i')))
                .unwrap_or_default();

            let escaped = escape(find.as_str());
            let re = RegexBuilder::new(&escaped)
                .case_insensitive(ignore_case)
                .build()
                .unwrap(); // Of course it's valid, we just escaped special chars
            let result = if global {
                re.replace_all(&message, replace.as_str())
            } else {
                re.replace(&message, replace.as_str())
            };
            if foreign_author {
                Ok(format!(
                    "(edited by {}) <{}> {}",
                    msg.author, author, result
                ))
            } else {
                msg.history
                    .edit_message(author, 0, result.to_string())
                    .await;
                Ok(format!("<{}> {}", author, result))
            }
        } else {
            Ok("Invalid usage.".into())
        }
    }
}
