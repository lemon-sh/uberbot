use serde::Deserialize;

#[derive(Deserialize)]
pub struct UberConfig {
    pub irc: IrcConfig,
    pub spotify: Option<SpotifyConfig>,
    pub db_path: Option<String>,
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
