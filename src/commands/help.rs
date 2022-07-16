use crate::bot::{Message, Command};
use async_trait::async_trait;

const HELP: &str = concat!(
    "=- \x1d\x02Überbot\x0f ", env!("CARGO_PKG_VERSION"), " -=\n",
    " * waifu <category>\n",
    " * owo/mock/leet [user]\n",
    " * ev <math expression>\n",
    " - This bot also provides titles of URLs and details for Spotify URIs/links. It can also resolve sed expressions.\n"
);

pub struct Help;

#[async_trait]
impl Command for Help {
    //noinspection RsNeedlessLifetimes
    async fn execute<'a>(&mut self, _msg: Message<'a>) -> anyhow::Result<String> {
        Ok(HELP.into())
    }
}
