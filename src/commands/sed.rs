use async_trait::async_trait;
use regex::RegexBuilder;
use crate::bot::{Trigger, TriggerContext};

pub struct Sed;

#[async_trait]
impl Trigger for Sed {
    async fn execute(&self, ctx: TriggerContext) -> anyhow::Result<String> {
        let foreign_author;
        let author = if let Some(author) = ctx.captures.name("u") {
            foreign_author = true;
            author
        } else {
            foreign_author = false;
            &ctx.author
        };
        let message = if let Some(msg) = ctx.history.last_msg(author).await {
            msg
        } else {
            return Ok("No previous messages found.".into());
        };
        if let (Some(find), Some(replace)) = (ctx.captures.name("r"), ctx.captures.name("w")) {
            let (global, ignore_case) = ctx.captures
                .name("f")
                .map(|s| (s.contains('g'), s.contains('i')))
                .unwrap_or_default();

            let re = RegexBuilder::new(find)
                .case_insensitive(ignore_case)
                .build()?;
            let result = if global {
                re.replace_all(&message, replace)
            } else {
                re.replace(&message, replace)
            };
            if foreign_author {
                Ok(format!(
                    "(edited by {}) <{}> {}",
                    ctx.author, author, result
                ))
            } else {
                ctx.history
                    .edit_message(author, 0, result.to_string())
                    .await;
                Ok(format!("<{}> {}", author, result))
            }
        } else {
            Ok("Invalid usage.".into())
        }
    }
}
