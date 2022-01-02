use arrayvec::ArrayString;
use meval::Context;
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::Write;

#[derive(Debug)]
pub enum LeekCommand {
    Owo,
    Leet,
    Mock,
}

pub async fn get_waifu_pic(category: &str) -> anyhow::Result<Option<String>> {
    let api_resp = reqwest::get(format!("https://api.waifu.pics/sfw/{}", category))
        .await?
        .text()
        .await?;
    let api_resp = api_resp.trim();
    let value: Value = serde_json::from_str(&api_resp)?;
    let url = value["url"].as_str().map(|v| v.to_string());
    Ok(url)
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

pub async fn execute_leek(
    state: &mut crate::AppState,
    cmd: LeekCommand,
    channel: &str,
    nick: &str,
) -> anyhow::Result<()> {
    match state.last_msgs.get(nick) {
        Some(msg) => {
            tracing::debug!("Executing {:?} on {:?}", cmd, msg);
            let output = match cmd {
                LeekCommand::Owo => super::leek::owoify(msg)?,
                LeekCommand::Leet => super::leek::leetify(msg)?,
                LeekCommand::Mock => super::leek::mock(msg)?,
            };
            state.client.privmsg(channel, &output).await?;
        }
        None => {
            state
                .client
                .privmsg(channel, "No last messages found.")
                .await?;
        }
    }
    Ok(())
}
