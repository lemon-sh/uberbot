use crate::{
    history::MessageHistory,
    regex_util::{FancyRegexExt, OwnedCaptures},
    ExecutorConnection,
};
use async_trait::async_trait;
use fancy_regex::Regex;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::mpsc;

#[async_trait]
pub trait Trigger {
    async fn execute(&self, ctx: TriggerContext) -> anyhow::Result<String>;
}

#[async_trait]
pub trait Command {
    async fn execute(&self, ctx: CommandContext) -> anyhow::Result<String>;
}

pub struct CommandContext {
    pub history: Arc<MessageHistory>,
    pub author: String,
    pub content: Option<String>,
    pub db: ExecutorConnection,
}

pub struct TriggerContext {
    pub history: Arc<MessageHistory>,
    pub author: String,
    // we can omit content because it's the same as captures.get(0).unwrap()
    pub captures: OwnedCaptures,
    pub db: ExecutorConnection,
}

pub struct TriggerEntry {
    name: String,
    regex: Regex,
    handler: Arc<dyn Trigger + Send + Sync>,
}

pub struct Bot<SF: Fn(String, String) -> anyhow::Result<()>> {
    history: Arc<MessageHistory>,
    prefixes: Vec<String>,
    db: ExecutorConnection,
    commands: HashMap<String, Arc<dyn Command + Send + Sync>>,
    triggers: Vec<TriggerEntry>,
    ignored_triggers: Option<HashMap<String, Vec<String>>>,
    sendmsg: Arc<SF>,
}

/// Extracts the command and argument (remainder) from the message
fn dissect<'a>(prefixes: &[String], str: &'a str) -> Option<(&'a str, Option<&'a str>)> {
    for prefix in prefixes {
        if let Some(str) = str.strip_prefix(prefix) {
            return if let Some(o) = str.find(' ') {
                Some((&str[..o], Some(&str[o + 1..])))
            } else {
                Some((str, None))
            };
        }
    }
    None
}

impl<SF> Bot<SF>
where
    SF: Fn(String, String) -> anyhow::Result<()> + Send + Sync + 'static,
{
    pub fn new(
        prefixes: Vec<String>,
        db: ExecutorConnection,
        hdepth: usize,
        sendmsg: SF,
        ignored_triggers: Option<HashMap<String, Vec<String>>>,
    ) -> Self {
        Bot {
            history: Arc::new(MessageHistory::new(hdepth)),
            commands: HashMap::new(),
            triggers: Vec::new(),
            prefixes,
            db,
            sendmsg: Arc::new(sendmsg),
            ignored_triggers,
        }
    }

    pub fn add_command<C: Command + Send + Sync + 'static>(&mut self, name: String, cmd: C) {
        self.commands.insert(name, Arc::new(cmd));
    }

    pub fn add_trigger<C: Trigger + Send + Sync + 'static>(
        &mut self,
        name: String,
        regex: Regex,
        trig: C,
    ) {
        if let Some(ign) = self.ignored_triggers.as_ref().and_then(|v| v.get(&name)) {
            if ign.iter().any(|v| *v == name) {
                tracing::debug!(
                    "Not installing trigger {}, because it's globally ignored",
                    name
                );
                return;
            }
        }
        self.triggers.push(TriggerEntry {
            name,
            regex,
            handler: Arc::new(trig),
        });
    }

    pub(crate) async fn handle_message(
        &self,
        origin: String,
        author: String,
        content: String,
        cancel: mpsc::Sender<()>,
    ) {
        let content = content.trim();
        // first we check if the message is a command
        if let Some((command, remainder)) = dissect(&self.prefixes, content) {
            tracing::debug!("Got command: {:?} -> {:?}", command, remainder);
            if command.is_empty() {
                return;
            }
            // now we need to find a handler for this command
            if let Some(handler) = self.commands.get(command) {
                // we found a command, we can now spawn its handler
                let ctx = CommandContext {
                    author,
                    content: remainder.map(ToString::to_string),
                    db: self.db.clone(),
                    history: self.history.clone(),
                };
                let sendmsg = self.sendmsg.clone();
                let handler = handler.clone();
                tokio::spawn(async move {
                    #[allow(clippy::no_effect_underscore_binding)]
                    let _cancel = cancel;
                    let result = handler
                        .execute(ctx)
                        .await
                        .unwrap_or_else(|e| format!("Error: {e}"));
                    (sendmsg)(origin, result)
                });
                return;
            }
            // no handler found :c
            let _res = (self.sendmsg)(origin, "Unknown command.".into());
            return;
        }
        // at this point we need to make the message owned
        let content = content.to_string();
        // the message is not a command, maybe it's a trigger?
        for trigger in &self.triggers {
            let captures = trigger.regex.owned_captures(&content).unwrap();
            // we need to find a regex that matches this message
            if let Some(captures) = captures {
                // check if it's not ignored
                if let Some(ign) = self
                    .ignored_triggers
                    .as_ref()
                    .and_then(|v| v.get(&trigger.name))
                {
                    if ign.contains(&origin) {
                        tracing::debug!(
                            "Skipping ignored trigger {} for channel {}",
                            trigger.name,
                            origin
                        );
                        break;
                    }
                }
                // and spawn the trigger handler
                let ctx = TriggerContext {
                    author,
                    captures,
                    db: self.db.clone(),
                    history: self.history.clone(),
                };
                let sendmsg = self.sendmsg.clone();
                let handler = trigger.handler.clone();
                tokio::spawn(async move {
                    #[allow(clippy::no_effect_underscore_binding)]
                    let _cancel = cancel;
                    let result = handler
                        .execute(ctx)
                        .await
                        .unwrap_or_else(|e| format!("Error: {e}"));
                    (sendmsg)(origin, result)
                });
                return;
            }
        }
        // no regex matched the message, so it's neither a command nor a trigger
        // it's a regular message, so we add it to the message history
        self.history.add_message(&author, content).await;
    }
}
