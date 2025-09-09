use aws_sdk_dynamodb::types::AttributeValue;
use mockall::automock;

use crate::{
    OpaqueError,
    user::{User, UserMaster},
};

const USER_TABLE_NAME: &str = "spotify-playlist-notification_user";
const LAST_NOTIFIED_TRACK_TABLE_NAME: &str = "spotify-playlist-notification_last_notified_track";
const SPOTIFY_REFRESH_TOKEN_TABLE_NAME: &str =
    "spotify-playlist-notification_spotify_refresh_token";

#[automock]
pub trait DynamoDBClientTrait {
    async fn extract_user_master(&self) -> Result<UserMaster, OpaqueError>;
    async fn extract_last_notified_track_id(&self) -> Result<Option<String>, OpaqueError>;
    async fn update_last_notified_track_id(&self, new_track_id: &str) -> Result<(), OpaqueError>;
    async fn extract_spotify_refresh_token(&self) -> Result<Option<String>, OpaqueError>;
    async fn update_spotify_refresh_token(
        &self,
        new_refresh_token: &str,
    ) -> Result<(), OpaqueError>;
}

pub struct DynamoDBClient {
    client: aws_sdk_dynamodb::Client,
}

impl DynamoDBClient {
    pub async fn new() -> Self {
        let config = aws_config::load_from_env().await;
        let client = aws_sdk_dynamodb::Client::new(&config);
        DynamoDBClient { client }
    }
}

impl DynamoDBClientTrait for DynamoDBClient {
    async fn extract_user_master(&self) -> Result<UserMaster, OpaqueError> {
        let mut users = Vec::new();
        let request = self.client.scan().table_name(USER_TABLE_NAME);
        let response = request.send().await?;
        if let Some(items) = response.items {
            for item in items {
                let name = item
                    .get("name")
                    .and_then(|v| v.as_s().ok())
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                let spotify_user_id = item
                    .get("spotify_user_id")
                    .and_then(|v| v.as_s().ok())
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                let discord_user_id = item
                    .get("discord_user_id")
                    .and_then(|v| v.as_s().ok())
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                let order = item
                    .get("order")
                    .and_then(|v| v.as_n().ok())
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(0);
                users.push(User {
                    name,
                    spotify_user_id,
                    discord_user_id,
                    order,
                });
            }
        }
        users.sort_by_key(|user| user.order);
        Ok(UserMaster { users })
    }

    async fn extract_last_notified_track_id(&self) -> Result<Option<String>, OpaqueError> {
        let request = self
            .client
            .get_item()
            .table_name(LAST_NOTIFIED_TRACK_TABLE_NAME)
            .key(
                "singleton_key",
                AttributeValue::S("last_notified_track_id".to_string()),
            );
        let response = request.send().await?;
        if let Some(item) = response.item {
            if let Some(track_id) = item
                .get("id")
                .and_then(|v| v.as_s().ok())
                .map(|s| s.to_string())
            {
                return Ok(Some(track_id));
            }
        }
        Ok(None)
    }

    async fn update_last_notified_track_id(&self, new_track_id: &str) -> Result<(), OpaqueError> {
        let request = self
            .client
            .update_item()
            .table_name(LAST_NOTIFIED_TRACK_TABLE_NAME)
            .key(
                "singleton_key",
                AttributeValue::S("last_notified_track_id".to_string()),
            )
            .update_expression("SET id = :new_id")
            .expression_attribute_values(":new_id", AttributeValue::S(new_track_id.to_string()));
        request.send().await?;
        Ok(())
    }

    async fn extract_spotify_refresh_token(&self) -> Result<Option<String>, OpaqueError> {
        let request = self
            .client
            .get_item()
            .table_name(SPOTIFY_REFRESH_TOKEN_TABLE_NAME)
            .key(
                "singleton_key",
                AttributeValue::S("spotify_refresh_token".to_string()),
            );
        let response = request.send().await?;
        if let Some(item) = response.item {
            if let Some(refresh_token) = item
                .get("refresh_token")
                .and_then(|v| v.as_s().ok())
                .map(|s| s.to_string())
            {
                return Ok(Some(refresh_token));
            }
        }
        Ok(None)
    }

    async fn update_spotify_refresh_token(
        &self,
        new_refresh_token: &str,
    ) -> Result<(), OpaqueError> {
        let request = self
            .client
            .update_item()
            .table_name(SPOTIFY_REFRESH_TOKEN_TABLE_NAME)
            .key(
                "singleton_key",
                AttributeValue::S("spotify_refresh_token".to_string()),
            )
            .update_expression("SET refresh_token = :new_refresh_token")
            .expression_attribute_values(
                ":new_refresh_token",
                aws_sdk_dynamodb::types::AttributeValue::S(new_refresh_token.to_string()),
            );
        request.send().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dotenvy::dotenv;

    #[tokio::test]
    async fn test_extract_user_master() {
        dotenv().ok();
        let dynamodb_client = DynamoDBClient::new().await;
        let user_master = dynamodb_client.extract_user_master().await.unwrap();
        for user in user_master.users {
            println!("{:?}", user);
        }
    }

    #[tokio::test]
    async fn test_extract_last_notified_track_id() {
        dotenv().ok();
        let dynamodb_client = DynamoDBClient::new().await;
        let track_id = dynamodb_client
            .extract_last_notified_track_id()
            .await
            .unwrap();
        println!("{:?}", track_id);
    }

    #[tokio::test]
    async fn test_extract_spotify_refresh_token() {
        dotenv().ok();
        let dynamodb_client = DynamoDBClient::new().await;
        let refresh_token = dynamodb_client
            .extract_spotify_refresh_token()
            .await
            .unwrap();
        println!("{:?}", refresh_token);
    }
}
