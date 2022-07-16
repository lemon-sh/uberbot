use std::collections::HashMap;
use fancy_regex::Regex;
use crate::ExecutorConnection;
use async_trait::async_trait;

fn separate_to_space(str: &str, prefix_len: usize) -> (&str, Option<&str>) {
    if let Some(o) = str.find(' ') {
        (&str[prefix_len..o], Some(&str[o + 1..]))
    } else {
        (&str[prefix_len..], None)
    }
}

pub trait RegexCommand {
    fn execute(&mut self, message: String) -> anyhow::Result<String>;
}

#[async_trait]
pub trait NormalCommand {
    async fn execute(&mut self, last_msg: &HashMap<String, String>, message: String) -> anyhow::Result<String>;
}

#[derive(Default)]
struct Commands {
    regex: Vec<(Regex, Box<dyn RegexCommand + Send>)>,
    normal: HashMap<String, Box<dyn NormalCommand + Send>>,
}

pub struct Bot<SF: FnMut(String, String) -> anyhow::Result<()>> {
    last_msg: HashMap<String, String>,
    prefix: String,
    db: ExecutorConnection,
    commands: Commands,
    sendmsg: SF
}

impl<SF: FnMut(String, String) -> anyhow::Result<()>> Bot<SF> {
    pub fn new(prefix: String, db: ExecutorConnection, sendmsg: SF) -> Self {
        Bot {
            last_msg: HashMap::new(),
            prefix,
            db,
            commands: Commands::default(),
            sendmsg
        }
    }

    pub fn add_command<C: NormalCommand + Send + 'static>(&mut self, name: String, cmd: C) {
        self.commands.normal.insert(name, Box::new(cmd));
    }

    pub fn add_regex_command<C: RegexCommand + Send + 'static>(&mut self, regex: Regex, cmd: C) {
        self.commands.regex.push((regex, Box::new(cmd)));
    }

    pub async fn handle_message(&mut self, origin: &str, author: &str, content: &str) -> anyhow::Result<()> {
        (self.sendmsg)(origin.into(), content.into()).unwrap();
        Ok(())
    }
}