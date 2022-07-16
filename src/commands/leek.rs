use crate::bot::{Command, Message};
use arrayvec::ArrayString;
use async_trait::async_trait;
use rand::Rng;
use std::{
    error::Error,
    fmt::{Debug, Display},
};

#[derive(Debug)]
pub struct CapacityError(arrayvec::CapacityError);

impl Display for CapacityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Error for CapacityError {}

impl<T> From<arrayvec::CapacityError<T>> for CapacityError {
    fn from(e: arrayvec::CapacityError<T>) -> Self {
        Self(e.simplify())
    }
}

type LeekResult = Result<ArrayString<512>, CapacityError>;

fn mock(input: &str) -> ArrayString<512> {
    let mut builder = ArrayString::<512>::new();

    for ch in input.chars() {
        if rand::random() {
            builder.push(ch.to_ascii_uppercase());
        } else {
            builder.push(ch.to_ascii_lowercase());
        }
    }

    builder
}

fn leetify(input: &str) -> ArrayString<512> {
    let mut builder = ArrayString::<512>::new();

    for ch in input.chars() {
        builder.push(match ch.to_ascii_lowercase() {
            'a' => '4',
            'e' => '3',
            'i' => '1',
            'o' => '0',
            'g' => '6',
            's' => '5',
            't' => '7',
            'b' => '8',
            _ => ch,
        });
    }

    builder
}

fn owoify(input: &str) -> LeekResult {
    let mut builder: ArrayString<512> = ArrayString::from("\x1d")?;
    let mut rng = rand::thread_rng();
    let mut last_char = '\0';
    for byte in input.bytes() {
        let mut ch = char::from(byte);
        if !ch.is_ascii() {
            continue;
        }
        // owoify character
        ch = match ch.to_ascii_lowercase() {
            'r' | 'l' => 'w',
            _ => ch,
        };
        // stutter (e.g. "o-ohayou gozaimasu!")
        if last_char == ' ' && rng.gen_bool(0.2) {
            builder.try_push(ch)?;
            builder.try_push('-')?;
        }
        match ch {
            // nya-ify
            'a' | 'e' | 'i' | 'o' | 'u' if last_char == 'n' => {
                builder.try_push('y')?;
            }
            // textmoji
            '.' => {
                builder.try_push_str(match rng.gen_range(0..6) {
                    1 => " >~<",
                    2 => " (◕ᴗ◕✿)",
                    3 => " >w<",
                    4 => " >_<",
                    5 => " OwO",
                    _ => " ^^",
                })?;
            }
            _ => {}
        }
        builder.try_push(ch)?;
        last_char = ch;
    }
    builder.try_push_str("~~")?;
    Ok(builder)
}

#[derive(Debug, Clone, Copy)]
enum LeekCommand {
    Owo,
    Leet,
    Mock,
}

async fn execute_leek(cmd: LeekCommand, msg: &Message<'_>) -> anyhow::Result<String> {
    let nick = msg.content.unwrap_or(msg.author);
    match msg.last_msg.read().await.get(nick) {
        Some(msg) => Ok(match cmd {
            LeekCommand::Owo => owoify(msg)?,
            LeekCommand::Leet => leetify(msg),
            LeekCommand::Mock => mock(msg),
        }
        .to_string()),
        None => Ok("No previous messages found.".into()),
    }
}

pub struct Owo;
pub struct Leet;
pub struct Mock;

#[async_trait]
impl Command for Owo {
    //noinspection RsNeedlessLifetimes
    async fn execute<'a>(&mut self, msg: Message<'a>) -> anyhow::Result<String> {
        execute_leek(LeekCommand::Owo, &msg).await
    }
}

#[async_trait]
impl Command for Leet {
    //noinspection RsNeedlessLifetimes
    async fn execute<'a>(&mut self, msg: Message<'a>) -> anyhow::Result<String> {
        execute_leek(LeekCommand::Leet, &msg).await
    }
}

#[async_trait]
impl Command for Mock {
    //noinspection RsNeedlessLifetimes
    async fn execute<'a>(&mut self, msg: Message<'a>) -> anyhow::Result<String> {
        execute_leek(LeekCommand::Mock, &msg).await
    }
}
