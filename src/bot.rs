use crate::ExecutorConnection;
use async_trait::async_trait;
use fancy_regex::{Captures, Regex};
use std::collections::HashMap;

fn separate_to_space(str: &str, prefix_len: usize) -> (&str, Option<&str>) {
    if let Some(o) = str.find(' ') {
        (&str[prefix_len..o], Some(&str[o + 1..]))
    } else {
        (&str[prefix_len..], None)
    }
}

#[async_trait]
pub trait Trigger {
    async fn execute(&mut self, msg: Message, matches: Captures) -> anyhow::Result<String>;
}

#[async_trait]
pub trait Command {
    //noinspection RsNeedlessLifetimes
    async fn execute<'a>(&mut self, msg: Message<'a>) -> anyhow::Result<String>;
}

pub struct Message<'a> {
    pub last_msg: &'a HashMap<String, String>,
    pub author: &'a str,
    pub content: Option<&'a str>,
}

pub struct Bot<SF: FnMut(String, String) -> anyhow::Result<()>> {
    last_msg: HashMap<String, String>,
    prefix: String,
    db: ExecutorConnection,
    commands: HashMap<String, Box<dyn Command + Send>>,
    triggers: Vec<(Regex, Box<dyn Trigger + Send>)>,
    sendmsg: SF,
}

impl<SF: FnMut(String, String) -> anyhow::Result<()>> Bot<SF> {
    pub fn new(prefix: String, db: ExecutorConnection, sendmsg: SF) -> Self {
        Bot {
            last_msg: HashMap::new(),
            commands: HashMap::new(),
            triggers: Vec::new(),
            prefix,
            db,
            sendmsg,
        }
    }

    pub fn add_command<C: Command + Send + 'static>(&mut self, name: String, cmd: C) {
        self.commands.insert(name, Box::new(cmd));
    }

    pub fn add_regex_command<C: Trigger + Send + 'static>(&mut self, regex: Regex, cmd: C) {
        self.triggers.push((regex, Box::new(cmd)));
    }

    pub async fn handle_message(
        &mut self,
        origin: &str,
        author: &str,
        content: &str,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}
