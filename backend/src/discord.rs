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
        latest_track_urls: &[&str],
        next_user_id: &str,
        channel_id: &str,
    ) -> Result<(), OpaqueError> {
        let message_lines = vec![
            "プレイリストが更新されました！".to_string(),
            latest_track_urls.join("\n"),
            format!("next: <@{}>", next_user_id),
        ];
        let request = DiscordCreateMessageRequest {
            content: message_lines.join("\n"),
        };
        self.create_discord_message(channel_id, &request).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_message() {
        dotenvy::dotenv().ok();
        let channel_id = env::var("DISCORD_CHANNEL_ID").unwrap();
        let message_lines = vec![
            "プレイリストが更新されました！",
            "https://open.spotify.com/intl-ja/track/...",
            "https://open.spotify.com/intl-ja/track/...",
            "next: <@...>",
        ];
        let request = DiscordCreateMessageRequest {
            content: message_lines.join("\n"),
        };
        let client = DiscordClient::init().unwrap();
        let res = client
            .create_discord_message(&channel_id, &request)
            .await
            .unwrap();
        println!("{:?}", res);
        println!("{:?}", res.text().await.unwrap());
    }
}
