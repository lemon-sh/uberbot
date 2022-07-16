use crate::bot::{Message, Command};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

#[derive(Default)]
pub struct Waifu {
    http: Client
}

#[async_trait]
impl Command for Waifu {
    //noinspection RsNeedlessLifetimes
    async fn execute<'a>(&mut self, msg: Message<'a>) -> anyhow::Result<String> {
        let category = msg.content.unwrap_or("waifu");
        let request = self.http.get(format!("https://api.waifu.pics/sfw/{}", category)).build()?;
        let response = self.http.execute(request)
            .await?
            .text()
            .await?;
        let response = response.trim();
        let value: Value = serde_json::from_str(response)?;
        let url = value["url"]
            .as_str()
            .unwrap_or("Invalid API Response.")
            .to_string();
        Ok(url)
    }
}
