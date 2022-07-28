use crate::history::MessageHistory;
use crate::ExecutorConnection;
use async_trait::async_trait;
use fancy_regex::{Captures, Regex};
use std::collections::HashMap;
use tokio::sync::{Mutex, mpsc};

fn dissect<'a>(prefixes: &[String], str: &'a str) -> Option<(&'a str, Option<&'a str>)> {
    for prefix in prefixes {
        if let Some(str) = str.strip_prefix(prefix) {
            return if let Some(o) = str.find(' ') {
                Some((&str[..o], Some(&str[o + 1..])))
            } else {
                Some((str, None))
            }
        }
    }
    None
}

#[async_trait]
pub trait Trigger {
    async fn execute<'a>(
        &mut self,
        msg: Context<'a>,
        captures: Captures<'a>,
    ) -> anyhow::Result<String>;
}

#[async_trait]
pub trait Command {
    async fn execute(&mut self, msg: Context<'_>) -> anyhow::Result<String>;
}

pub struct Context<'a> {
    pub history: &'a MessageHistory,
    pub author: &'a str,
    // in case of triggers, this is always Some(...)
    pub content: Option<&'a str>,
    pub db: &'a ExecutorConnection,
}

pub struct Bot<SF: Fn(String, String) -> anyhow::Result<()>> {
    history: MessageHistory,
    prefixes: Vec<String>,
    db: ExecutorConnection,
    commands: HashMap<String, Box<Mutex<dyn Command + Send>>>,
    triggers: Vec<(Regex, Box<Mutex<dyn Trigger + Send>>)>,
    sendmsg: SF,
}

impl<SF: Fn(String, String) -> anyhow::Result<()>> Bot<SF> {
    pub fn new(prefixes: Vec<String>, db: ExecutorConnection, hdepth: usize, sendmsg: SF) -> Self {
        Bot {
            history: MessageHistory::new(hdepth),
            commands: HashMap::new(),
            triggers: Vec::new(),
            prefixes,
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
        let content = content.trim();
        if let Some((command, remainder)) = dissect(&self.prefixes, content) {
            tracing::debug!("Got command: {:?} -> {:?}", command, remainder);
            if let Some(handler) = self.commands.get(command) {
                let msg = Context {
                    author,
                    content: remainder,
                    db: &self.db,
                    history: &self.history,
                };
                return (self.sendmsg)(origin.into(), handler.lock().await.execute(msg).await?);
            }
            return (self.sendmsg)(origin.into(), "Unknown command.".into());
        }
        for trigger in &self.triggers {
            let captures = trigger.0.captures(content)?;
            if let Some(captures) = captures {
                let msg = Context {
                    author,
                    content: Some(content),
                    db: &self.db,
                    history: &self.history,
                };
                return (self.sendmsg)(
                    origin.into(),
                    trigger.1.lock().await.execute(msg, captures).await?,
                );
            }
        }
        self.history.add_message(author, content).await;
        Ok(())
    }

    pub async fn handle_message(&self, origin: String, author: String, content: String, _cancel_handle: mpsc::Sender<()>) {
        if let Err(e) = self.handle_message_inner(&origin, &author, &content).await {
            let _err = (self.sendmsg)(origin.into(), format!("Error: {}", e));
        }
    }
}
