use arrayvec::ArrayString;
use meval::Context;
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::Write;
use crate::bot::NormalCommand;
use async_trait::async_trait;

pub struct Waifu;

#[async_trait]
impl NormalCommand for Waifu {
    async fn execute(&mut self, _last_msg: &HashMap<String, String>, message: String) -> anyhow::Result<String> {
        let api_resp = reqwest::get(format!("https://api.waifu.pics/sfw/{}", message))
            .await?
            .text()
            .await?;
        let api_resp = api_resp.trim();
        let value: Value = serde_json::from_str(api_resp)?;
        let url = value["url"].as_str().unwrap_or("Invalid API Response.").to_string();
        Ok(url)
    }
}

pub fn mathbot(
    author: String,
    expr: Option<&str>,
    last_evals: &mut HashMap<String, f64>,
) -> anyhow::Result<ArrayString<256>> {
    if let Some(expr) = expr {
        let last_eval = last_evals.entry(author).or_insert(0.0);
        let mut meval_ctx = Context::new();
        let mut result = ArrayString::new();
        let value = meval::eval_str_with_context(expr, meval_ctx.var("x", *last_eval))?;
        *last_eval = value;
        tracing::debug!("{} = {}", expr, value);
        write!(result, "{} = {}", expr, value)?;
        Ok(result)
    } else {
        Ok(ArrayString::from("No expression to evaluate")?)
    }
}
