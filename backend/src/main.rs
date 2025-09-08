use std::{env, error::Error};

use lambda_runtime::{LambdaEvent, service_fn};
use serde::Deserialize;

use crate::{
    discord::DiscordClient,
    dynamodb::DynamoDBClient,
    spotify::{SpotifyClient, SpotifyPlaylistItem, SpotifyPlaylistResponse},
    user::UserMaster,
};

mod discord;
mod dynamodb;
mod spotify;
mod user;

pub type OpaqueError = Box<dyn Error + Send + Sync + 'static>;

#[derive(Deserialize)]
struct LambdaPayload {}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    lambda_runtime::run(service_fn(lambda_handler)).await?;
    Ok(())
}

async fn lambda_handler(_event: LambdaEvent<LambdaPayload>) -> Result<(), lambda_runtime::Error> {
    match execute_process().await {
        Ok(_) => Ok(()),
        Err(e) => {
            println!("{:}", e);
            Err(e)
        }
    }
}

async fn execute_process() -> Result<(), OpaqueError> {
    let processer = SpotifyPlaylistNotificationProcesser::init().await?;
    processer.execute().await?;
    Ok(())
}

struct SpotifyPlaylistNotificationProcesser {
    playlist_id: String,
    discord_channel_id: String,
    dynamodb_client: DynamoDBClient,
    user_master: UserMaster,
    spotify_client: SpotifyClient,
    discord_client: DiscordClient,
}

impl SpotifyPlaylistNotificationProcesser {
    async fn init() -> Result<Self, OpaqueError> {
        let playlist_id = env::var("SPOTIFY_PLAYLIST_ID")?;
        let discord_channel_id = env::var("DISCORD_CHANNEL_ID")?;
        let dynamodb_client = DynamoDBClient::new().await;
        let user_master = dynamodb_client.extract_user_master().await?;
        let spotify_refresh_token = if let Some(spotify_refresh_token) =
            dynamodb_client.extract_spotify_refresh_token().await?
        {
            spotify_refresh_token
        } else {
            return Err("no spotify_refresh_token".into());
        };
        let spotify_client = SpotifyClient::init(&spotify_refresh_token).await?;
        let discord_client = DiscordClient::init()?;

        Ok(Self {
            playlist_id,
            discord_channel_id,
            dynamodb_client,
            user_master,
            spotify_client,
            discord_client,
        })
    }

    async fn execute(&self) -> Result<(), OpaqueError> {
        let last_notified_track_id = if let Some(last_notified_track_id) = self
            .dynamodb_client
            .extract_last_notified_track_id()
            .await?
        {
            last_notified_track_id
        } else {
            return Err("no last_notified_track_id".into());
        };
        let spotify_playlist = self
            .spotify_client
            .get_spotify_playlist(&self.playlist_id)
            .await?;
        let spotify_playlist_tracks = self
            .spotify_client
            .list_all_spotify_playlist_tracks(&self.playlist_id)
            .await?;
        let last_track = if let Some(last_track) = spotify_playlist_tracks.get_latest_track() {
            last_track
        } else {
            return Err("no last_track".into());
        };
        let target_tracks =
            spotify_playlist_tracks.get_not_notified_tracks(&last_notified_track_id);
        let last_notified_track_id = self
            .notify(&spotify_playlist, &last_track, &target_tracks)
            .await?;
        if let Some(last_notified_track_id) = last_notified_track_id {
            self.dynamodb_client
                .update_last_notified_track_id(&last_notified_track_id)
                .await?;
        }
        if let Some(new_refresh_token) = &self.spotify_client.get_next_spotify_refresh_token() {
            self.dynamodb_client
                .update_spotify_refresh_token(new_refresh_token)
                .await?;
        }
        Ok(())
    }

    async fn notify(
        &self,
        spotify_playlist: &SpotifyPlaylistResponse,
        last_track: &SpotifyPlaylistItem,
        target_tracks: &[&SpotifyPlaylistItem],
    ) -> Result<Option<String>, OpaqueError> {
        if target_tracks.is_empty() {
            return Ok(None);
        }
        let next_user = if let Some(next_user) = self
            .user_master
            .get_next_user_by_spotify_id(&last_track.added_by.id)
        {
            next_user
        } else {
            return Err("no next_user".into());
        };
        self.discord_client
            .send_latest_tracks_and_next_user_message(
                &self.discord_channel_id,
                &spotify_playlist.name,
                &spotify_playlist.external_urls.spotify,
                &target_tracks
                    .iter()
                    .map(|t| t.track.external_urls.spotify.as_str())
                    .collect::<Vec<&str>>(),
                &next_user.discord_user_id,
            )
            .await?;
        Ok(Some(last_track.track.id.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_execute_process() {
        dotenvy::dotenv().ok();
        execute_process().await.unwrap();
    }

    #[tokio::test]
    async fn test_notify() {
        dotenvy::dotenv().ok();
        let processer = SpotifyPlaylistNotificationProcesser::init().await.unwrap();
        let spotify_playlist = processer
            .spotify_client
            .get_spotify_playlist(&processer.playlist_id)
            .await
            .unwrap();
        let spotify_playlist_tracks = processer
            .spotify_client
            .list_all_spotify_playlist_tracks(&processer.playlist_id)
            .await
            .unwrap();
        let last_track = spotify_playlist_tracks.get_latest_track().unwrap();
        let target_tracks: Vec<&SpotifyPlaylistItem> = spotify_playlist_tracks
            .items
            .iter()
            .rev()
            .take(3)
            .rev()
            .collect();
        processer
            .notify(&spotify_playlist, last_track, &target_tracks)
            .await
            .unwrap();
        if let Some(new_refresh_token) = processer.spotify_client.get_next_spotify_refresh_token() {
            processer
                .dynamodb_client
                .update_spotify_refresh_token(new_refresh_token)
                .await
                .unwrap();
        }
    }
}
