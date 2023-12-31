use crate::bot::{Trigger, TriggerContext};
use async_trait::async_trait;
use fancy_regex::Regex;
use htmlescape::decode_html;
use hyper::header::HeaderValue;
use reqwest::Client;

pub struct Title {
    http: Client,
    title_regex: Regex,
    user_agent: String,
}

impl Title {
    pub fn new(user_agent: Option<String>) -> anyhow::Result<Self> {
        Ok(Title {
            http: Client::new(),
            title_regex: Regex::new(r"<title[^>]*>(.*?)</title>")?,
            user_agent: user_agent.unwrap_or_else(|| {
                format!("uberbot {} (reqwest)", env!("CARGO_PKG_VERSION")).to_string()
            }),
        })
    }
}

#[async_trait]
impl Trigger for Title {
    async fn execute(&self, ctx: TriggerContext) -> anyhow::Result<String> {
        let url = ctx.captures.get(0).unwrap();
        tracing::debug!("url: {}", url);

        let request = self
            .http
            .get(url)
            .header("User-Agent", &self.user_agent)
            .header("Accept", "text/html, */*")
            .build()?;
        let response = self.http.execute(request).await?;
        let headers = response.headers();

        let header = headers.get("Content-Type");
        let mime = header
            .map(HeaderValue::to_str)
            .transpose()?
            .unwrap_or("text/html");
        if mime.contains("text/html") {
            let body = response.text().await?;
            if let Some(tm) = self.title_regex.captures(&body)?.and_then(|c| c.get(1)) {
                let title_match = tm.as_str();
                let result = decode_html(title_match).unwrap_or_else(|_| title_match.to_string());
                Ok(format!("\x039[Title]\x0311 {result}"))
            } else {
                Ok("\x039[Title]\x0311 No title".into())
            }
        } else {
            let content_length = response.content_length().map(|l| (l / 1024).to_string());
            let size = content_length.as_deref().unwrap_or("unknown");
            Ok(format!("\x039[Title]\x0311 File: {mime}; {size}kb"))
        }
    }
}
