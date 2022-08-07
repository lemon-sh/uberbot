// This file is taken almost verbatim from other sources.
use anyhow::anyhow;
use reqwest::Client;
use serde_json::json;
use std::collections::HashMap;

fn extract_relevant_lines<'a>(
    mut stderr: &'a str,
    strip_start_tokens: &[&str],
    strip_end_tokens: &[&str],
) -> &'a str {
    // Find best matching start token
    if let Some(start_token_pos) = strip_start_tokens
        .iter()
        .filter_map(|t| stderr.rfind(t))
        .max()
    {
        // Keep only lines after that
        stderr = match stderr[start_token_pos..].find('\n') {
            Some(line_end) => &stderr[(line_end + start_token_pos + 1)..],
            None => "",
        };
    }

    // Find best matching end token
    if let Some(end_token_pos) = strip_end_tokens
        .iter()
        .filter_map(|t| stderr.rfind(t))
        .min()
    {
        // Keep only lines before that
        stderr = match stderr[..end_token_pos].rfind('\n') {
            Some(prev_line_end) => &stderr[..=prev_line_end],
            None => "",
        };
    }

    // Strip trailing or leading empty lines
    stderr = stderr.trim_start_matches('\n');
    while stderr.ends_with("\n\n") {
        stderr = &stderr[..(stderr.len() - 1)];
    }

    stderr
}

pub fn format_play_eval_stderr(stderr: &str, show_compiler_warnings: bool) -> String {
    let compiler_output = extract_relevant_lines(
        stderr,
        &["Compiling playground"],
        &[
            "warning emitted",
            "warnings emitted",
            "warning: `playground` (bin \"playground\") generated",
            "error: could not compile",
            "error: aborting",
            "Finished ",
        ],
    );

    if stderr.contains("Running `target") {
        // Program successfully compiled, so compiler output will be just warnings
        let program_stderr = extract_relevant_lines(stderr, &["Running `target"], &[]);

        if show_compiler_warnings {
            // Concatenate compiler output and program stderr with a newline
            match (compiler_output, program_stderr) {
                ("", "") => String::new(),
                (warnings, "") => warnings.to_owned(),
                ("", stderr) => stderr.to_owned(),
                (warnings, stderr) => format!("{}\n{}", warnings, stderr),
            }
        } else {
            program_stderr.to_owned()
        }
    } else {
        // Program didn't get to run, so there must be an error, so we yield the compiler output
        // regardless of whether warn is enabled or not
        compiler_output.to_owned()
    }
}

pub async fn post_gist(client: &Client, code: &str) -> anyhow::Result<String> {
    let mut resp: HashMap<String, String> = client
        .post("https://play.rust-lang.org/meta/gist")
        .json(&json!({ "code": code }))
        .send()
        .await?
        .json()
        .await?;

    let gist_id = resp.remove("id").ok_or_else(|| anyhow!("no gist found"))?;

    Ok(format!(
        "https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist={}",
        gist_id
    ))
}

// this part was written by missing
pub struct StrChunks<'a> {
    v: &'a str,
    size: usize,
}

impl<'a> Iterator for StrChunks<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<Self::Item> {
        if self.v.is_empty() {
            return None;
        }
        if self.v.len() < self.size {
            let res = self.v;
            self.v = &self.v[self.v.len()..];
            return Some(res);
        }
        let mut offset = self.size;
        let res = loop {
            match self.v.get(..offset) {
                Some(v) => break v,
                None => {
                    offset -= 1;
                }
            }
        };
        self.v = &self.v[offset..];
        Some(res)
    }
}

impl<'a> StrChunks<'a> {
    pub fn new(v: &'a str, size: usize) -> Self {
        Self { v, size }
    }
}
