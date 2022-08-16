mod util;

use crate::bot::{Command, CommandContext};
use anyhow::anyhow;
use anyhow::bail;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Deserializer};
use serde_json::json;
use util::format_play_eval_stderr;
use util::post_gist;
use util::StrChunks;

#[derive(Debug)]
pub struct PlayResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

// this deserialize impl is taken from the rustbot source
impl<'de> Deserialize<'de> for PlayResult {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // The playground occasionally just sends a single "error" field, so we need to handle that
        // case.

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum RawResp {
            Err {
                error: String,
            },
            Ok {
                success: bool,
                stdout: String,
                stderr: String,
            },
        }

        Ok(match RawResp::deserialize(deserializer)? {
            RawResp::Ok {
                success,
                stdout,
                stderr,
            } => PlayResult {
                success,
                stdout,
                stderr,
            },
            RawResp::Err { error } => PlayResult {
                success: false,
                stdout: String::new(),
                stderr: error,
            },
        })
    }
}

async fn play_eval_base(client: &Client, ctx: CommandContext, dbg: bool) -> anyhow::Result<String> {
    let count = ctx
        .content
        .ok_or_else(|| anyhow!("No count specified"))?
        .parse::<usize>()?;

    let messages = ctx
        .history
        .last_msgs(&ctx.author, count)
        .await
        .ok_or_else(|| anyhow!("No code to run!"))?;

    let mut code = messages.join("\n");
    if dbg {
        if code.contains("fn main()") {
            bail!("fn main is not allowed in dbg");
        }

        code = format!("fn main() {{ println!(\"{{:?}}\", {{{}}}) }}", code);
        dbg!(&code);
    }

    let mut result: PlayResult = client
        .post("https://play.rust-lang.org/execute")
        .json(&json! ({
            "code": &code,
            "channel": "stable",
            "crateType": "bin",
            "edition": "2021",
            "mode": "debug",
            "tests": false
        }))
        .send()
        .await?
        .json()
        .await?;

    result.stderr = format_play_eval_stderr(&result.stderr, false);

    let mut to_send = if result.stderr.is_empty() {
        result.stdout
    } else if result.stdout.is_empty() {
        result.stderr
    } else {
        format!("{}\n{}", result.stderr, result.stdout)
    };

    to_send = to_send.replace('\n', "\r\n");

    let lines = to_send
        .lines()
        .flat_map(|s| StrChunks::new(s, 400))
        .collect::<Vec<_>>();

    if lines.len() > 10 {
        to_send = format!(
            "Output too large.\r\nPlayground link: {}",
            post_gist(client, &code).await?
        );
    }

    to_send = format!("{}:\r\n{}", ctx.author, to_send);

    Ok(to_send)
}

#[derive(Default)]
pub struct Play {
    client: Client,
}

#[async_trait]
impl Command for Play {
    async fn execute(&self, ctx: CommandContext) -> anyhow::Result<String> {
        play_eval_base(&self.client, ctx, false).await
    }
}

#[derive(Default)]
pub struct Dbg {
    client: Client,
}

#[async_trait]
impl Command for Dbg {
    async fn execute(&self, ctx: CommandContext) -> anyhow::Result<String> {
        play_eval_base(&self.client, ctx, true).await
    }
}
