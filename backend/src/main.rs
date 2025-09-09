use std::{env, error::Error};

use lambda_runtime::{LambdaEvent, service_fn};
use serde::Deserialize;

use crate::{
    discord::DiscordClient,
    dynamodb::{DynamoDBClient, DynamoDBClientTrait},
    spotify::{SpotifyClient, SpotifyClientTrait, SpotifyPlaylistItem, SpotifyPlaylistResponse},
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
    let dynamodb_client = DynamoDBClient::new().await;
    let spotify_refresh_token = if let Some(spotify_refresh_token) =
        dynamodb_client.extract_spotify_refresh_token().await?
    {
        spotify_refresh_token
    } else {
        return Err("no spotify_refresh_token".into());
    };
    let spotify_client = SpotifyClient::init(&spotify_refresh_token).await?;
    let processer =
        SpotifyPlaylistNotificationProcesser::init(dynamodb_client, spotify_client).await?;
    processer.execute().await?;
    Ok(())
}

struct SpotifyPlaylistNotificationProcesser<D: DynamoDBClientTrait, S: SpotifyClientTrait> {
    playlist_id: String,
    discord_channel_id: String,
    dynamodb_client: D,
    user_master: UserMaster,
    spotify_client: S,
    discord_client: DiscordClient,
}

impl<D: DynamoDBClientTrait, S: SpotifyClientTrait> SpotifyPlaylistNotificationProcesser<D, S> {
    async fn init(dynamodb_client: D, spotify_client: S) -> Result<Self, OpaqueError> {
        let playlist_id = env::var("SPOTIFY_PLAYLIST_ID")?;
        let discord_channel_id = env::var("DISCORD_CHANNEL_ID")?;
        let user_master = dynamodb_client.extract_user_master().await?;
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
        let last_notified_track_id = if let Some(last_notified_track_id) = self
            .dynamodb_client
            .extract_last_notified_track_id()
            .await?
        {
            last_notified_track_id
        } else {
            // last_notified_track_idが存在しない場合は最新の曲までを通知済みとみなす
            last_track.track.id.clone()
        };
        let target_tracks = if let Some(target_tracks) =
            spotify_playlist_tracks.get_not_notified_tracks(&last_notified_track_id)
        {
            target_tracks
        } else {
            // last_notified_track_idに該当するトラックが見つからなかった場合は最新の一曲を追加分とみなす
            vec![last_track]
        };
        self.notify(&spotify_playlist, &last_track, &target_tracks)
            .await?;
        // last_notified_track_idが存在しなかった場合は最新の曲までを通知済みとして更新する
        self.dynamodb_client
            .update_last_notified_track_id(&last_track.track.id.clone())
            .await?;
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
    ) -> Result<(), OpaqueError> {
        if target_tracks.is_empty() {
            return Ok(());
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
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use mockall::predicate::eq;

    use crate::{
        dynamodb::MockDynamoDBClientTrait,
        spotify::{
            MockSpotifyClientTrait, SpotifyPlaylistTracksResponse, SpotifyTrack, SpotifyUser,
        },
        user::User,
    };

    use super::*;

    #[tokio::test]
    async fn test_execute_process() {
        dotenvy::dotenv().ok();
        execute_process().await.unwrap();
    }

    impl UserMaster {
        fn new_test_data() -> Self {
            UserMaster {
                users: vec![
                    User {
                        name: "User 1".to_string(),
                        spotify_user_id: "spotify_user_1".to_string(),
                        discord_user_id: "discord_user_1".to_string(),
                        order: 1,
                    },
                    User {
                        name: "User 2".to_string(),
                        spotify_user_id: "spotify_user_2".to_string(),
                        discord_user_id: "discord_user_2".to_string(),
                        order: 2,
                    },
                ],
            }
        }
    }

    impl SpotifyPlaylistResponse {
        fn new_test_data() -> Self {
            SpotifyPlaylistResponse {
                name: "Test Playlist".to_string(),
                external_urls: spotify::SpotifyExternalUrls {
                    spotify: "https://open.spotify.com/playlist/test".to_string(),
                },
            }
        }
    }

    impl SpotifyPlaylistTracksResponse {
        fn new_test_data() -> Self {
            SpotifyPlaylistTracksResponse {
                next: None,
                items: vec![
                    SpotifyPlaylistItem {
                        added_by: SpotifyUser {
                            id: "spotify_user_1".to_string(),
                        },
                        track: SpotifyTrack {
                            id: "track_1".to_string(),
                            name: "Track 1".to_string(),
                            external_urls: spotify::SpotifyExternalUrls {
                                spotify: "https://open.spotify.com/track/track_1".to_string(),
                            },
                        },
                        added_at: "2023-01-01T00:00:00Z".to_string(),
                    },
                    SpotifyPlaylistItem {
                        added_by: SpotifyUser {
                            id: "spotify_user_2".to_string(),
                        },
                        track: SpotifyTrack {
                            id: "track_2".to_string(),
                            name: "Track 2".to_string(),
                            external_urls: spotify::SpotifyExternalUrls {
                                spotify: "https://open.spotify.com/track/track_2".to_string(),
                            },
                        },
                        added_at: "2023-01-02T00:00:00Z".to_string(),
                    },
                ],
            }
        }
    }

    #[tokio::test]
    async fn test_last_notified_track_id_not_found() {
        dotenvy::dotenv().ok();
        let mut mock_dynamodb_client = MockDynamoDBClientTrait::new();
        mock_dynamodb_client
            .expect_extract_last_notified_track_id()
            .returning(|| Ok(None));
        mock_dynamodb_client
            .expect_extract_spotify_refresh_token()
            .returning(|| Ok(Some(env::var("SPOTIFY_REFRESH_TOKEN").unwrap())));
        mock_dynamodb_client
            .expect_extract_user_master()
            .returning(|| Ok(UserMaster::new_test_data()));
        mock_dynamodb_client
            .expect_update_last_notified_track_id()
            .with(eq("track_2"))
            .returning(|_| Ok(()));
        mock_dynamodb_client
            .expect_update_spotify_refresh_token()
            .returning(|_| Ok(()));
        let mut mock_spotify_client = MockSpotifyClientTrait::new();
        mock_spotify_client
            .expect_get_spotify_playlist()
            .returning(|_| Ok(SpotifyPlaylistResponse::new_test_data()));
        mock_spotify_client
            .expect_list_all_spotify_playlist_tracks()
            .returning(|_| Ok(SpotifyPlaylistTracksResponse::new_test_data()));
        mock_spotify_client
            .expect_get_next_spotify_refresh_token()
            .return_const(None);
        let processer =
            SpotifyPlaylistNotificationProcesser::init(mock_dynamodb_client, mock_spotify_client)
                .await
                .unwrap();
        processer.execute().await.unwrap();
    }

    #[tokio::test]
    async fn test_invalid_last_notified_track_id() {
        dotenvy::dotenv().ok();
        let mut mock_dynamodb_client = MockDynamoDBClientTrait::new();
        mock_dynamodb_client
            .expect_extract_last_notified_track_id()
            .returning(|| Ok("invalid_track_id".to_string().into()));
        mock_dynamodb_client
            .expect_extract_spotify_refresh_token()
            .returning(|| Ok(Some(env::var("SPOTIFY_REFRESH_TOKEN").unwrap())));
        mock_dynamodb_client
            .expect_extract_user_master()
            .returning(|| Ok(UserMaster::new_test_data()));
        mock_dynamodb_client
            .expect_update_last_notified_track_id()
            .with(eq("track_2"))
            .returning(|_| Ok(()));
        mock_dynamodb_client
            .expect_update_spotify_refresh_token()
            .returning(|_| Ok(()));
        let mut mock_spotify_client = MockSpotifyClientTrait::new();
        mock_spotify_client
            .expect_get_spotify_playlist()
            .returning(|_| Ok(SpotifyPlaylistResponse::new_test_data()));
        mock_spotify_client
            .expect_list_all_spotify_playlist_tracks()
            .returning(|_| Ok(SpotifyPlaylistTracksResponse::new_test_data()));
        mock_spotify_client
            .expect_get_next_spotify_refresh_token()
            .return_const(None);
        let processer =
            SpotifyPlaylistNotificationProcesser::init(mock_dynamodb_client, mock_spotify_client)
                .await
                .unwrap();
        processer.execute().await.unwrap();
    }

    #[tokio::test]
    async fn test_valid_last_notified_track_id() {
        dotenvy::dotenv().ok();
        let mut mock_dynamodb_client = MockDynamoDBClientTrait::new();
        mock_dynamodb_client
            .expect_extract_last_notified_track_id()
            .returning(|| Ok("track_1".to_string().into()));
        mock_dynamodb_client
            .expect_extract_spotify_refresh_token()
            .returning(|| Ok(Some(env::var("SPOTIFY_REFRESH_TOKEN").unwrap())));
        mock_dynamodb_client
            .expect_extract_user_master()
            .returning(|| Ok(UserMaster::new_test_data()));
        mock_dynamodb_client
            .expect_update_last_notified_track_id()
            .with(eq("track_2"))
            .returning(|_| Ok(()));
        mock_dynamodb_client
            .expect_update_spotify_refresh_token()
            .returning(|_| Ok(()));
        let mut mock_spotify_client = MockSpotifyClientTrait::new();
        mock_spotify_client
            .expect_get_spotify_playlist()
            .returning(|_| Ok(SpotifyPlaylistResponse::new_test_data()));
        mock_spotify_client
            .expect_list_all_spotify_playlist_tracks()
            .returning(|_| Ok(SpotifyPlaylistTracksResponse::new_test_data()));
        mock_spotify_client
            .expect_get_next_spotify_refresh_token()
            .return_const(None);
        let processer =
            SpotifyPlaylistNotificationProcesser::init(mock_dynamodb_client, mock_spotify_client)
                .await
                .unwrap();
        processer.execute().await.unwrap();
    }
}
