use std::env;

use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap};
use serde::Serialize;

use crate::OpaqueError;

#[derive(Serialize)]
struct DiscordCreateMessageRequest {
    content: String,
}

pub struct DiscordClient {
    bot_token: String,
}

impl DiscordClient {
    pub fn init() -> Result<Self, OpaqueError> {
        let bot_token = env::var("DISCORD_BOT_TOKEN")?;
        Ok(Self { bot_token })
    }
}
impl DiscordClient {
    async fn create_discord_message(
        &self,
        channel_id: &str,
        request: &DiscordCreateMessageRequest,
    ) -> Result<reqwest::Response, OpaqueError> {
        let mut headers = HeaderMap::new();
        headers.append(AUTHORIZATION, format!("Bot {}", self.bot_token).parse()?);
        headers.append(CONTENT_TYPE, "application/json".parse()?);
        let reqwest_client = reqwest::Client::new();
        let response = reqwest_client
            .post(format!(
                "https://discord.com/api/v10/channels/{channel_id}/messages"
            ))
            .headers(headers)
            .body(serde_json::to_string(&request)?)
            .send()
            .await?;
        Ok(response)
    }

    pub async fn send_latest_tracks_and_next_user_message(
        &self,
        channel_id: &str,
        playlist_name: &str,
        playlist_url: &str,
        latest_track_urls: &[&str],
        next_user_id: &str,
    ) -> Result<reqwest::Response, OpaqueError> {
        let message_lines = vec![
            "## プレイリスト更新のお知らせ".to_string(),
            "\n".to_string(),
            format!("[{playlist_name}]({playlist_url})が更新されました！",),
            "### 追加された曲".to_string(),
            "\n".to_string(),
            latest_track_urls.join("\n"),
            "### 次の人".to_string(),
            "\n".to_string(),
            format!("<@{}>", next_user_id),
        ];
        let request = DiscordCreateMessageRequest {
            content: message_lines.join("\n"),
        };
        let response = self.create_discord_message(channel_id, &request).await?;
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_message() {
        dotenvy::dotenv().ok();
        let channel_id = env::var("DISCORD_CHANNEL_ID").unwrap();
        let playlist_name = "test";
        let playlist_url = "https://open.spotify.com/playlist/...";
        let latest_track_urls = vec![
            "https://open.spotify.com/track/1",
            "https://open.spotify.com/track/2",
        ];
        let next_user_id = "...";
        let client = DiscordClient::init().unwrap();
        let res = client
            .send_latest_tracks_and_next_user_message(
                &channel_id,
                playlist_name,
                playlist_url,
                &latest_track_urls,
                next_user_id,
            )
            .await
            .unwrap();
        println!("{:?}", res);
        println!("{:?}", res.text().await.unwrap());
    }
}
