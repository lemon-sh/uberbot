use serde::Deserialize;

#[derive(Deserialize)]
pub struct UberConfig {
    pub log_level: Option<String>,
    pub irc: IrcConfig,
    pub spotify: Option<SpotifyConfig>,
    pub bot: BotConfig
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
    pub port: u16,
    pub username: String,
    pub prefix: String,
}

#[derive(Deserialize)]
pub struct BotConfig {
    pub db_path: Option<String>,
    pub history_depth: usize,
    pub search_limit: Option<usize>
}