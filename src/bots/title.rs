use fancy_regex::Regex;
use htmlescape::decode_html;
use rspotify::clients::BaseClient;
use rspotify::model::PlayableItem;
use rspotify::{model::Id, ClientCredsSpotify, Credentials};
use tracing::debug;

fn calculate_playtime(secs: u64) -> (u64, u64) {
    let mut dur_sec = secs;
    let dur_min = dur_sec / 60;
    dur_sec -= dur_min * 60;
    (dur_min, dur_sec)
}

async fn resolve_spotify(
    spotify: &mut ClientCredsSpotify,
    resource_type: &str,
    resource_id: &str,
) -> anyhow::Result<String> {
    // uncomment this if titlebot commits suicide after exactly 30 minutes

    // if spotify.token.lock().await.unwrap().as_ref().unwrap().is_expired() {
    //     spotify.request_token().await?;
    // }
    debug!(
        "Resolving Spotify resource '{}' with id '{}'",
        resource_type, resource_id
    );
    match resource_type {
        "track" => {
            let track = spotify.track(&Id::from_id(resource_id)?).await?;
            let playtime = calculate_playtime(track.duration.as_secs());
            let artists: Vec<String> = track.artists.into_iter().map(|x| x.name).collect();
            Ok(format!("\x037[Spotify]\x03 Track: \x039\"{}\"\x03 - \x039\"{}\" \x0311|\x03 Album: \x039\"{}\" \x0311|\x03 Length:\x0315 {}:{:02} \x0311|", artists.join(", "), track.name, track.album.name, playtime.0, playtime.1))
        }
        "artist" => {
            let artist = spotify.artist(&Id::from_id(resource_id)?).await?;
            Ok(format!(
                "\x037[Spotify]\x03 Artist: \x039\"{}\" \x0311|\x03 Genres:\x039 {} \x0311|",
                artist.name,
                artist.genres.join(", ")
            ))
        }
        "album" => {
            let album = spotify.album(&Id::from_id(resource_id)?).await?;
            let playtime = calculate_playtime(
                album
                    .tracks
                    .items
                    .iter()
                    .fold(0, |acc, x| acc + x.duration.as_secs()),
            );
            Ok(format!("\x037[Spotify]\x03 Album: \x039\"{}\" \x0311|\x03 Tracks:\x0315 {} \x0311|\x03 Release date:\x039 {} \x0311|\x03 Length:\x0315 {}:{:02} \x0311|", album.name, album.tracks.total, album.release_date, playtime.0, playtime.1))
        }
        "playlist" => {
            let playlist = spotify
                .playlist(&Id::from_id(resource_id)?, None, None)
                .await?;
            let mut tracks = 0;
            let playtime = calculate_playtime(playlist.tracks.items.iter().fold(0, |acc, x| {
                x.track.as_ref().map_or(acc, |item| match item {
                    PlayableItem::Track(t) => {
                        tracks += 1;
                        acc + t.duration.as_secs()
                    }
                    PlayableItem::Episode(e) => {
                        tracks += 1;
                        acc + e.duration.as_secs()
                    }
                })
            }));
            Ok(format!("\x037[Spotify]\x03 Playlist: \x039\"{}\" \x0311|\x03 Tracks/Episodes:\x0315 {} \x0311|\x03 Length:\x0315 {}:{:02} \x0311|\x03 Description: \x039\"{}\" \x0311|", playlist.name, tracks, playtime.0, playtime.1, playlist.description.unwrap_or_else(|| "<empty>".into())))
        }
        _ => Ok("\x037[Spotify]\x03 Error: Invalid resource type".into()),
    }
}

pub struct Titlebot {
    url_regex: Regex,
    title_regex: Regex,
    spotify_regex: Regex,
    spotify: ClientCredsSpotify,
}

impl Titlebot {
    pub async fn create(spotify_creds: Credentials) -> anyhow::Result<Self> {
        let url_regex = Regex::new(r"https?://\w+\.\w+[/\S+]*")?;
        let title_regex = Regex::new(r"(?<=<title>)(.*)(?=</title>)")?;
        let spotify_regex = Regex::new(
            r"(?:https?|spotify):(?://open\.spotify\.com/)?(track|artist|album|playlist)[/:]([a-zA-Z0-9]*)",
        )?;
        let mut spotify = ClientCredsSpotify::new(spotify_creds);
        spotify.request_token().await?;
        Ok(Self {
            url_regex,
            title_regex,
            spotify_regex,
            spotify,
        })
    }

    pub async fn resolve(&mut self, message: &str) -> anyhow::Result<Option<String>> {
        if let Some(m) = self.spotify_regex.captures(&message)? {
            tracing::debug!("{}", message);
            let tp_group = m.get(1).unwrap();
            let id_group = m.get(2).unwrap();
            return Ok(Some(
                resolve_spotify(
                    &mut self.spotify,
                    &message[tp_group.start()..tp_group.end()],
                    &message[id_group.start()..id_group.end()],
                )
                .await?,
            ));
        } else if let Some(m) = self.url_regex.find(&message)? {
            let url = &message[m.start()..m.end()];
            tracing::debug!("url: {}", url);
            let response = reqwest::get(url).await?;
            if let Some(header) = response.headers().get("Content-Type") {
                tracing::debug!("response header: {}", header.to_str()?);
                if !(header.to_str()?.contains("text/html")) {
                    return Ok(None);
                }
            }
            let body = response.text().await?;
            if let Some(tm) = self.title_regex.find(&body)? {
                let title_match = &body[tm.start()..tm.end()];
                let result = decode_html(title_match).unwrap_or_else(|_| title_match.to_string());
                tracing::debug!("result: {}", result);
                return Ok(Some(format!("\x039[Title]\x0311 {}", result)));
            }
        }
        Ok(None)
    }
}
