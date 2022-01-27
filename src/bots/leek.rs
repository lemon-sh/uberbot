use arrayvec::ArrayString;
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
        builder.push(match ch {
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
                    1 => " OwO",
                    2 => " (◕ᴗ◕✿)",
                    3 => " >w<",
                    4 => " >_<",
                    5 => " ^•ﻌ•^",
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
pub enum Command {
    Owo,
    Leet,
    Mock,
}

pub fn execute(
    state: &mut crate::AppState,
    cmd: Command,
    target: &str,
    nick: &str,
) -> anyhow::Result<()> {
    match state.last_msgs.get(nick) {
        Some(msg) => {
            tracing::debug!("Executing {:?} on {:?}", cmd, msg);
            let output = match cmd {
                Command::Owo => super::leek::owoify(msg)?,
                Command::Leet => super::leek::leetify(msg),
                Command::Mock => super::leek::mock(msg),
            };
            state.client.send_privmsg(target, &output)?;
        }
        None => {
            state
                .client
                .send_privmsg(target, "No last messages found.")?;
        }
    }
    Ok(())
}
