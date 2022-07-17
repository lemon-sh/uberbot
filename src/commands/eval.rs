use std::collections::HashMap;
use async_trait::async_trait;
use meval::Context;
use crate::bot::{Command, Message};

#[derive(Default)]
pub struct Eval {
    last_eval: HashMap<String, f64>
}

#[async_trait]
impl Command for Eval {
    async fn execute(&mut self, msg: Message<'_>) -> anyhow::Result<String> {
        if let Some(expr) = msg.content {
            let last_eval = self.last_eval.entry(msg.author.into()).or_insert(0.0);
            let mut meval_ctx = Context::new();
            let value = meval::eval_str_with_context(expr, meval_ctx.var("x", *last_eval))?;
            *last_eval = value;
            Ok(format!("{} = {}", expr, value))
        } else {
            Ok("No expression to evaluate".into())
        }
    }
}
