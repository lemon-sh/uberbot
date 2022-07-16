use crate::bot::{Message, Command};
use async_trait::async_trait;

const HELP: &str = concat!(
    "=- \x1d\x02Überbot\x0f ", env!("CARGO_PKG_VERSION"), " -=\r\n",
    " * waifu <category>\r\n",
    " * owo/mock/leet [user]\r\n",
    " * ev <math expression>\r\n",
    " - This bot also provides titles of URLs and details for Spotify URIs/links. It can also resolve sed expressions."
);

pub struct Help;

#[async_trait]
impl Command for Help {
    //noinspection RsNeedlessLifetimes
    async fn execute<'a>(&mut self, _msg: Message<'a>) -> anyhow::Result<String> {
        Ok(HELP.into())
    }
}