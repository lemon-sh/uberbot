use crate::bot::{Command, Context};
use async_trait::async_trait;
use std::collections::HashMap;

#[derive(Default)]
pub struct Eval {
    last_eval: HashMap<String, f64>,
}

#[async_trait]
impl Command for Eval {
    async fn execute(&mut self, msg: Context<'_>) -> anyhow::Result<String> {
        if let Some(expr) = msg.content {
            let last_eval = self.last_eval.entry(msg.author.into()).or_insert(0.0);
            let mut meval_ctx = meval::Context::new();
            let value = meval::eval_str_with_context(expr, meval_ctx.var("x", *last_eval))?;
            *last_eval = value;
            Ok(format!("{} = {:.10}", expr, value))
        } else {
            Ok("No expression to evaluate".into())
        }
    }
}
