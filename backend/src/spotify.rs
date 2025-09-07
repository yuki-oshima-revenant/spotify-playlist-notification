use std::{collections::HashMap, env};

use reqwest::header::{CONTENT_TYPE, HeaderMap};
use serde::Deserialize;

use crate::OpaqueError;

#[derive(Deserialize, Debug)]
struct SpotifyTokenResponse {
    access_token: String,
    token_type: String,
    expires_in: i64,
    scope: String,
    refresh_token: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct SpotifyUser {
    pub id: String,
}

#[derive(Deserialize, Debug)]
pub struct SpotifyExternalUrls {
    pub spotify: String,
}

#[derive(Deserialize, Debug)]
pub struct SpotifyTrack {
    pub id: String,
    name: String,
    pub external_urls: SpotifyExternalUrls,
}

#[derive(Deserialize, Debug)]
pub struct SpotifyPlaylistItem {
    added_at: String,
    pub added_by: SpotifyUser,
    pub track: SpotifyTrack,
}

#[derive(Deserialize, Debug)]
pub struct SpotifyPlaylistTracksResponse {
    next: Option<String>,
    items: Vec<SpotifyPlaylistItem>,
}

impl SpotifyPlaylistTracksResponse {
    pub fn get_latest_track(&self) -> Option<&SpotifyPlaylistItem> {
        self.items.last()
    }

    pub fn get_not_notified_tracks(
        &self,
        last_notified_track_id: &str,
    ) -> Vec<&SpotifyPlaylistItem> {
        let mut not_notified_tracks = Vec::new();
        for item in self.items.iter().rev() {
            if item.track.id == last_notified_track_id {
                break;
            }
            not_notified_tracks.push(item);
        }
        not_notified_tracks.reverse();
        not_notified_tracks
    }
}

pub struct SpotifyClient {
    token_response: SpotifyTokenResponse,
}

impl SpotifyClient {
    async fn refresh_spotify_access_token(
        refresh_token: &str,
    ) -> Result<SpotifyTokenResponse, OpaqueError> {
        let client = reqwest::Client::new();
        let mut headers = HeaderMap::new();
        headers.append(CONTENT_TYPE, "application/x-www-form-urlencoded".parse()?);
        let mut params = HashMap::new();
        params.insert("grant_type", "refresh_token".to_string());
        // params.insert("refresh_token", env::var("SPOTIFY_REFRESH_TOKEN")?);
        params.insert("refresh_token", refresh_token.to_string());
        let res = client
            .post("https://accounts.spotify.com/api/token")
            .basic_auth(
                env::var("SPOTIFY_CLIENT_ID")?,
                env::var("SPOTIFY_CLIENT_SECRET").ok(),
            )
            .form(&params)
            .send()
            .await?;
        let res_body: SpotifyTokenResponse = res.json().await?;
        Ok(res_body)
    }

    pub async fn init(refresh_token: &str) -> Result<Self, OpaqueError> {
        let token_response = Self::refresh_spotify_access_token(refresh_token).await?;
        Ok(Self { token_response })
    }

    pub async fn get_spotify_playlist_tracks(
        &self,
        playlist_id: &str,
        url: Option<String>,
    ) -> Result<SpotifyPlaylistTracksResponse, OpaqueError> {
        let client = reqwest::Client::new();
        let url = if let Some(url) = url {
            url.to_string()
        } else {
            format!("https://api.spotify.com/v1/playlists/{playlist_id}/tracks")
        };
        let res = client
            .get(url)
            .bearer_auth(&self.get_access_token())
            .send()
            .await?;
        let res_body: SpotifyPlaylistTracksResponse = res.json().await?;
        Ok(res_body)
    }

    pub async fn list_all_spotify_playlist_tracks(
        &self,
        playlist_id: &str,
    ) -> Result<SpotifyPlaylistTracksResponse, OpaqueError> {
        let mut all_items = Vec::new();
        let mut next_url: Option<String> = None;
        loop {
            let res_body = self
                .get_spotify_playlist_tracks(playlist_id, next_url)
                .await?;
            all_items.extend(res_body.items);
            next_url = res_body.next.clone();
            if next_url.is_none() {
                break;
            }
        }
        Ok(SpotifyPlaylistTracksResponse {
            next: None,
            items: all_items,
        })
    }

    fn get_access_token(&self) -> &str {
        &self.token_response.access_token
    }

    pub fn get_next_spotify_refresh_token(&self) -> &Option<String> {
        &self.token_response.refresh_token
    }
}

#[cfg(test)]
mod tests {
    use crate::dynamodb::DynamoDBClient;

    use super::*;

    #[tokio::test]
    async fn test_get_spotify_playlist_tracks() {
        dotenvy::dotenv().ok();
        let playlist_id = env::var("SPOTIFY_PLAYLIST_ID").unwrap();
        let dynamodb_client = DynamoDBClient::new().await;
        let spotify_refresh_token = dynamodb_client
            .extract_spotify_refresh_token()
            .await
            .unwrap()
            .unwrap();
        let client = SpotifyClient::init(&spotify_refresh_token).await.unwrap();
        let res = client
            .get_spotify_playlist_tracks(&playlist_id, None)
            .await
            .unwrap();
        println!("{:?}", res);
    }
}
