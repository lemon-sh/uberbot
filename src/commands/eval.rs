use crate::bot::{Command, CommandContext};
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::Mutex;

#[derive(Default)]
pub struct Eval {
    last_eval: Mutex<HashMap<String, f64>>,
}

#[async_trait]
impl Command for Eval {
    async fn execute(&self, msg: CommandContext) -> anyhow::Result<String> {
        if let Some(expr) = msg.content {
            let mut last_eval = self.last_eval.lock().await;
            let last_eval = last_eval.entry(msg.author).or_insert(0.0);
            let mut meval_ctx = meval::Context::new();
            let value = meval::eval_str_with_context(&expr, meval_ctx.var("x", *last_eval))?;
            *last_eval = value;
            Ok(format!("{} = {:.10}", expr, value))
        } else {
            Ok("No expression to evaluate".into())
        }
    }
}
