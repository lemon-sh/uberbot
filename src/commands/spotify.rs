use crate::bot::{Trigger, TriggerContext};
use async_trait::async_trait;
use rspotify::{
    clients::BaseClient,
    model::{AlbumId, ArtistId, PlayableItem, PlaylistId, TrackId},
    ClientCredsSpotify, Credentials,
};

pub struct Spotify {
    spotify: ClientCredsSpotify,
}

impl Spotify {
    pub async fn new(creds: Credentials) -> anyhow::Result<Self> {
        let spotify = ClientCredsSpotify::new(creds);
        spotify.request_token().await?;
        Ok(Self { spotify })
    }
}

#[async_trait]
impl Trigger for Spotify {
    async fn execute(&self, ctx: TriggerContext) -> anyhow::Result<String> {
        let tp = ctx.captures.get(1).unwrap();
        let id = ctx.captures.get(2).unwrap();
        resolve_spotify(&self.spotify, tp, id).await
    }
}

fn calculate_playtime(secs: u64) -> (u64, u64) {
    let mut dur_sec = secs;
    let dur_min = dur_sec / 60;
    dur_sec -= dur_min * 60;
    (dur_min, dur_sec)
}

async fn resolve_spotify(
    spotify: &ClientCredsSpotify,
    resource_type: &str,
    resource_id: &str,
) -> anyhow::Result<String> {
    if spotify
        .token
        .lock()
        .await
        .unwrap()
        .as_ref()
        .unwrap()
        .is_expired()
    {
        spotify.request_token().await?;
    }
    tracing::debug!(
        "Resolving Spotify resource '{}' with id '{}'",
        resource_type,
        resource_id
    );
    match resource_type {
        "track" => {
            let track = spotify.track(TrackId::from_id(resource_id)?).await?;
            let playtime = calculate_playtime(track.duration.as_secs() as u64);
            let artists: Vec<String> = track.artists.into_iter().map(|x| x.name).collect();
            Ok(format!("\x037[Spotify]\x03 Track: \x039\"{}\"\x03 - \x039\"{}\" \x0311|\x03 Album: \x039\"{}\" \x0311|\x03 Length:\x0315 {}:{:02} \x0311|", artists.join(", "), track.name, track.album.name, playtime.0, playtime.1))
        }
        "artist" => {
            let artist = spotify.artist(ArtistId::from_id(resource_id)?).await?;
            Ok(format!(
                "\x037[Spotify]\x03 Artist: \x039\"{}\" \x0311|\x03 Genres:\x039 {} \x0311|",
                artist.name,
                artist.genres.join(", ")
            ))
        }
        "album" => {
            let album = spotify.album(AlbumId::from_id(resource_id)?).await?;
            let playtime = calculate_playtime(
                album
                    .tracks
                    .items
                    .iter()
                    .fold(0, |acc, x| acc + x.duration.as_secs() as u64),
            );
            Ok(format!("\x037[Spotify]\x03 Album: \x039\"{}\" \x0311|\x03 Tracks:\x0315 {} \x0311|\x03 Release date:\x039 {} \x0311|\x03 Length:\x0315 {}:{:02} \x0311|", album.name, album.tracks.total, album.release_date, playtime.0, playtime.1))
        }
        "playlist" => {
            let playlist = spotify
                .playlist(PlaylistId::from_id(resource_id)?, None, None)
                .await?;
            let mut tracks = 0;
            let playtime = calculate_playtime(playlist.tracks.items.iter().fold(0, |acc, x| {
                x.track.as_ref().map_or(acc, |item| match item {
                    PlayableItem::Track(t) => {
                        tracks += 1;
                        acc + t.duration.as_secs() as u64
                    }
                    PlayableItem::Episode(e) => {
                        tracks += 1;
                        acc + e.duration.as_secs() as u64
                    }
                })
            }));
            Ok(format!("\x037[Spotify]\x03 Playlist: \x039\"{}\" \x0311|\x03 Tracks/Episodes:\x0315 {} \x0311|\x03 Length:\x0315 {}:{:02} \x0311|\x03 Description: \x039\"{}\" \x0311|", playlist.name, tracks, playtime.0, playtime.1, playlist.description.unwrap_or_else(|| "<empty>".into())))
        }
        _ => Ok("\x037[Spotify]\x03 Error: Invalid resource type".into()),
    }
}
