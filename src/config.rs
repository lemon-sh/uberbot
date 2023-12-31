use std::{collections::HashMap, net::SocketAddr};

use serde::Deserialize;

#[derive(Deserialize)]
pub struct UberConfig {
    pub log_level: Option<String>,
    pub irc: IrcConfig,
    pub spotify: Option<SpotifyConfig>,
    pub bot: BotConfig,
    pub web: Option<HttpConfig>,
}

#[derive(Deserialize)]
pub struct SpotifyConfig {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Deserialize)]
pub struct IrcConfig {
    pub channels: Vec<String>,
    pub host: String,
    pub tls: bool,
    pub mode: Option<String>,
    pub nickname: Option<String>,
    pub alt_nicks: Option<Vec<String>>,
    pub port: u16,
    pub username: String,
    pub sasl_pass: Option<String>,
}

#[derive(Deserialize)]
pub struct BotConfig {
    pub db_path: Option<String>,
    pub history_depth: usize,
    pub search_limit: Option<usize>,
    pub prefixes: Vec<String>,
    pub ignored_triggers: Option<HashMap<String, Vec<String>>>,
    pub user_agent: Option<String>,
}

#[derive(Deserialize)]
pub struct HttpConfig {
    pub listen: SocketAddr,
    pub webhooks: HashMap<String, String>,
}
