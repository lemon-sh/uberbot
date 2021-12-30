use serde_json::Value;

pub async fn get_waifu_pic(category: &str) -> anyhow::Result<Option<String>> {
    let api_resp = reqwest::get(format!("https://api.waifu.pics/sfw/{}", category))
        .await?
        .text()
        .await?;
    let api_resp = api_resp.trim();
    tracing::debug!("API response: {}", api_resp);
    let value: Value = serde_json::from_str(&api_resp)?;
    let url = value["url"].as_str().map(|v| v.to_string());
    Ok(url)
}

