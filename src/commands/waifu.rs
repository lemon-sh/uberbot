use crate::bot::{Message, Command};
use async_trait::async_trait;
use serde_json::Value;

pub struct Waifu;

#[async_trait]
impl Command for Waifu {
    //noinspection RsNeedlessLifetimes
    async fn execute<'a>(&mut self, msg: Message<'a>) -> anyhow::Result<String> {
        let category = msg.content.unwrap_or("waifu");
        let api_resp = reqwest::get(format!("https://api.waifu.pics/sfw/{}", category))
            .await?
            .text()
            .await?;
        let api_resp = api_resp.trim();
        let value: Value = serde_json::from_str(api_resp)?;
        let url = value["url"]
            .as_str()
            .unwrap_or("Invalid API Response.")
            .to_string();
        Ok(url)
    }
}
