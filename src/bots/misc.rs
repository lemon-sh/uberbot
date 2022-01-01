use std::collections::HashMap;
use arrayvec::ArrayString;
use meval::Context;
use serde_json::Value;
use std::fmt::Write;

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

pub fn mathbot(author: String, expr: Option<&str>, last_evals: &mut HashMap<String, f64>) -> anyhow::Result<ArrayString<256>> {
    if let Some(expr) = expr {
        let last_eval = last_evals.entry(author).or_insert(0.0);
        let mut meval_ctx = Context::new();
        let mut result = ArrayString::new();
        let value = meval::eval_str_with_context(expr, meval_ctx.var("x", *last_eval))?;
        *last_eval = value;
        write!(result, "{} = {}", expr, value)?;
        Ok(result)
    } else {
        Ok(ArrayString::from("No expression to evaluate")?)
    }
}
