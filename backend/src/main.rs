use std::{env, error::Error};

use crate::{discord::DiscordClient, dynamodb::DynamoDBClient, spotify::SpotifyClient};

mod discord;
mod dynamodb;
mod spotify;
mod user;

pub type OpaqueError = Box<dyn Error + Send + Sync + 'static>;

#[tokio::main]
async fn main() {
    execute().await.unwrap();
}

async fn execute() -> Result<(), OpaqueError> {
    let playlist_id = env::var("SPOTIFY_PLAYLIST_ID")?;
    let discord_channel_id = env::var("DISCORD_CHANNEL_ID")?;
    let dynamodb_client = DynamoDBClient::new().await;
    let user_master = dynamodb_client.extract_user_master().await?;
    let last_notified_track_id = if let Some(last_notified_track_id) =
        dynamodb_client.extract_last_notified_track_id().await?
    {
        last_notified_track_id
    } else {
        return Err("no last_notified_track_id".into());
    };
    let spotify_refresh_token = if let Some(spotify_refresh_token) =
        dynamodb_client.extract_spotify_refresh_token().await?
    {
        spotify_refresh_token
    } else {
        return Err("no spotify_refresh_token".into());
    };
    let spotify_client = SpotifyClient::init(&spotify_refresh_token).await?;
    let discord_client = DiscordClient::init()?;
    let spotify_playlist_tracks = spotify_client
        .list_all_spotify_playlist_tracks(&playlist_id)
        .await?;
    let last_track = if let Some(last_track) = spotify_playlist_tracks.get_latest_track() {
        last_track
    } else {
        return Err("no last_track".into());
    };
    let target_tracks = spotify_playlist_tracks.get_not_notified_tracks(&last_notified_track_id);
    if !target_tracks.is_empty() {
        let next_user = if let Some(next_user) =
            user_master.get_next_user_by_spotify_id(&last_track.added_by.id)
        {
            next_user
        } else {
            return Err("no next_user".into());
        };
        discord_client
            .send_latest_tracks_and_next_user_message(
                &target_tracks
                    .iter()
                    .map(|t| t.track.external_urls.spotify.as_str())
                    .collect::<Vec<&str>>(),
                &next_user.discord_user_id,
                &discord_channel_id,
            )
            .await?;
        dynamodb_client
            .update_last_notified_track_id(&last_track.track.id)
            .await?;
    }
    if let Some(new_refresh_token) = &spotify_client.get_next_spotify_refresh_token() {
        dynamodb_client
            .update_spotify_refresh_token(new_refresh_token)
            .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_execute() {
        dotenvy::dotenv().ok();
        execute().await.unwrap();
    }
}
