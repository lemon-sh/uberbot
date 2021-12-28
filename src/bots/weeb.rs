use arrayvec::{ArrayString, CapacityError};
use rand::Rng;
use serde_json::Value;
use tracing::debug;
use std::result::Result;

pub async fn get_waifu_pic(category: &str) -> anyhow::Result<Option<String>> {
    let api_resp = reqwest::get(format!("https://api.waifu.pics/sfw/{}", category))
        .await?
        .text()
        .await?;
    let api_resp = api_resp.trim();
    debug!("API response: {}", api_resp);
    let value: Value = serde_json::from_str(&api_resp)?;
    let url = value["url"].as_str().map(|v| v.to_string());
    Ok(url)
}

pub struct OwoCapacityError(CapacityError);

impl<T> From<CapacityError<T>> for OwoCapacityError {
    fn from(e: CapacityError<T>) -> Self {
        Self { 0: e.simplify() }
    }
}

pub fn owoify_out_of_place(input: &str, output: &mut ArrayString<512>) -> Result<(), OwoCapacityError> {
    let input: ArrayString<512> = ArrayString::from(input)?;
    let mut rng = rand::thread_rng();
    let mut last_char = '\0';
    for byte in input.bytes() {
        let mut ch = char::from(byte);
        if !ch.is_ascii() {
            continue
        }
        // owoify character
        ch = match ch.to_ascii_lowercase() {
            'r' | 'l' => 'w',
            _ => ch
        };
        // stutter (e.g. "o-ohayou gozaimasu!")
        if last_char == ' ' && rng.gen_bool(0.2) {
            output.try_push(ch)?;
            output.try_push('-')?;
        }
        match ch {
            // nya-ify
            'a' | 'e' | 'i' | 'o' | 'u' if last_char == 'n' => {
                output.try_push('y')?;
            },
            // textmoji
            '.' => {
                output.try_push_str(match rng.gen_range(0..6) {
                    1 => " OwO",
                    2 => " :3",
                    3 => " >w<",
                    4 => " >_<",
                    5 => " ^•ﻌ•^",
                    _ => " ^^",
                })?;
            }
            _ => {}
        }
        output.try_push(ch)?;
        last_char = ch;
    }
    Ok(())
}
