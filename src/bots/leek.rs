use arrayvec::{ArrayString, CapacityError};
use rand::Rng;
use std::{error::Error, fmt::{Debug, Display}};

#[derive(Debug)]
pub struct LeekCapacityError(CapacityError);

impl Display for LeekCapacityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Error for LeekCapacityError {}

impl<T> From<CapacityError<T>> for LeekCapacityError {
    fn from(e: CapacityError<T>) -> Self {
        Self { 0: e.simplify() }
    }
}


type LeekResult = Result<ArrayString<512>, LeekCapacityError>;

pub fn mock(input: &str) -> LeekResult {
    let mut builder = ArrayString::<512>::new();

    for ch in input.chars() {
        if rand::random() {
            builder.try_push(ch.to_ascii_uppercase())?;
        } else {
            builder.try_push(ch.to_ascii_lowercase())?;
        }
    }

    Ok(builder)
}

pub fn leetify(input: &str) -> LeekResult {
    let mut builder = ArrayString::<512>::new();

    for ch in input.chars() {
        builder.try_push(match ch {
            'a' => '4',
            'e' => '3',
            'i' => '1',
            'o' => '0',
            'g' => '6',
            's' => '5',
            't' => '7',
            'b' => '8',
            _ => ch,
        })?;
    }

    Ok(builder)
}

pub fn owoify(input: &str) -> LeekResult {
    let mut builder: ArrayString<512> = ArrayString::from(input)?;
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
                    2 => " :3",
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
    Ok(builder)
}
