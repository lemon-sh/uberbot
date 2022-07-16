use crate::ExecutorConnection;
use async_trait::async_trait;
use fancy_regex::{Captures, Regex};
use std::collections::HashMap;
use tokio::sync::{Mutex, RwLock};

fn separate_to_space(str: &str, prefix_len: usize) -> (&str, Option<&str>) {
    if let Some(o) = str.find(' ') {
        (&str[prefix_len..o], Some(&str[o + 1..]))
    } else {
        (&str[prefix_len..], None)
    }
}

#[async_trait]
pub trait Trigger {
    async fn execute<'a>(&mut self, msg: Message<'a>, captures: Captures<'a>) -> anyhow::Result<String>;
}

#[async_trait]
pub trait Command {
    //noinspection RsNeedlessLifetimes
    async fn execute<'a>(&mut self, msg: Message<'a>) -> anyhow::Result<String>;
}

pub struct Message<'a> {
    pub last_msg: &'a RwLock<HashMap<String, String>>,
    pub author: &'a str,
    // in case of triggers, this is always Some(...)
    pub content: Option<&'a str>,
}

pub struct Bot<SF: Fn(String, String) -> anyhow::Result<()>> {
    last_msg: RwLock<HashMap<String, String>>,
    prefix: String,
    db: ExecutorConnection,
    commands: HashMap<String, Box<Mutex<dyn Command + Send>>>,
    triggers: Vec<(Regex, Box<Mutex<dyn Trigger + Send>>)>,
    sendmsg: SF,
}

impl<SF: Fn(String, String) -> anyhow::Result<()>> Bot<SF> {
    pub fn new(prefix: String, db: ExecutorConnection, sendmsg: SF) -> Self {
        Bot {
            last_msg: RwLock::new(HashMap::new()),
            commands: HashMap::new(),
            triggers: Vec::new(),
            prefix,
            db,
            sendmsg,
        }
    }

    pub fn add_command<C: Command + Send + 'static>(&mut self, name: String, cmd: C) {
        self.commands.insert(name, Box::new(Mutex::new(cmd)));
    }

    pub fn add_trigger<C: Trigger + Send + 'static>(&mut self, regex: Regex, cmd: C) {
        self.triggers.push((regex, Box::new(Mutex::new(cmd))));
    }

    async fn handle_message_inner(
        &self,
        origin: &str,
        author: &str,
        content: &str,
    ) -> anyhow::Result<()> {
        if content.starts_with(&self.prefix) {
            let (command, remainder) = separate_to_space(content, self.prefix.len());
            if let Some(handler) = self.commands.get(command) {
                let msg = Message {
                    last_msg: &self.last_msg,
                    author,
                    content: remainder
                };
                return (self.sendmsg)(origin.into(), handler.lock().await.execute(msg).await?)
            }
            return (self.sendmsg)(origin.into(), "Unknown command.".into())
        } else {
            for trigger in &self.triggers {
                let captures = trigger.0.captures(content)?;
                if let Some(captures) = captures {
                    let msg = Message {
                        last_msg: &self.last_msg,
                        author,
                        content: Some(content)
                    };
                    return (self.sendmsg)(origin.into(), trigger.1.lock().await.execute(msg, captures).await?)
                }
            }
            self.last_msg.write().await.insert(author.to_string(), content.to_string());
        }
        Ok(())
    }

    pub async fn handle_message(
        &self,
        origin: &str,
        author: &str,
        content: &str,
    ) {
        if let Err(e) = self.handle_message_inner(origin, author, content).await {
            let _ = (self.sendmsg)(origin.into(), format!("Error: {}", e));
        }
    }
}
