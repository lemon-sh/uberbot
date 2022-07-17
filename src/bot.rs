use crate::ExecutorConnection;
use async_trait::async_trait;
use fancy_regex::{Captures, Regex};
use std::collections::HashMap;
use tokio::sync::{Mutex, RwLock};

fn dissect<'a>(prefix: &str, str: &'a str) -> Option<(&'a str, Option<&'a str>)> {
    let str = str.strip_prefix(prefix)?;
    if let Some(o) = str.find(' ') {
        Some((&str[..o], Some(&str[o + 1..])))
    } else {
        Some((str, None))
    }
}

#[async_trait]
pub trait Trigger {
    async fn execute<'a>(&mut self, msg: Context<'a>, captures: Captures<'a>) -> anyhow::Result<String>;
}

#[async_trait]
pub trait Command {
    async fn execute(&mut self, msg: Context<'_>) -> anyhow::Result<String>;
}

pub struct Context<'a> {
    pub last_msg: &'a RwLock<HashMap<String, String>>,
    pub author: &'a str,
    // in case of triggers, this is always Some(...)
    pub content: Option<&'a str>,
    pub db: &'a ExecutorConnection
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
        if let Some((command, remainder)) = dissect(&self.prefix, content) {
            if let Some(handler) = self.commands.get(command) {
                let msg = Context {
                    last_msg: &self.last_msg,
                    author,
                    content: remainder,
                    db: &self.db
                };
                return (self.sendmsg)(origin.into(), handler.lock().await.execute(msg).await?)
            }
            return (self.sendmsg)(origin.into(), "Unknown command.".into())
        } else {
            for trigger in &self.triggers {
                let captures = trigger.0.captures(content)?;
                if let Some(captures) = captures {
                    let msg = Context {
                        last_msg: &self.last_msg,
                        author,
                        content: Some(content),
                        db: &self.db
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
