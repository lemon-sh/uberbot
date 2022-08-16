use crate::bot::{Command, CommandContext};
use async_trait::async_trait;

const HELP: &str = concat!(
    "=- \x1d\x02Überbot\x0f ",
    env!("CARGO_PKG_VERSION"),
    " -=\r\n",
    " * waifu <category>      * grab [count] <user>\r\n",
    " * owo/mock/leet [user]  * quot <user>\r\n",
    " * ev <math expression>  * qsearch <query>\r\n",
    " * play [count]          * dbg [count]\r\n",
    " - This bot can also resolve HTML titles, Spotify links and a subset of sed expressions."
);

pub struct Help;

#[async_trait]
impl Command for Help {
    async fn execute(&self, _msg: CommandContext) -> anyhow::Result<String> {
        Ok(HELP.into())
    }
}
