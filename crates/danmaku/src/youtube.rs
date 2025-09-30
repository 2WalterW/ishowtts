use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::message::{NormalizedMessage, Platform, Priority};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct LiveChatMessagesResponse {
    pub items: Vec<LiveChatMessageItem>,
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
    #[serde(rename = "pollingIntervalMillis")]
    pub polling_interval_millis: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct LiveChatMessageItem {
    pub id: String,
    pub snippet: LiveChatSnippet,
    #[serde(rename = "authorDetails")]
    pub author_details: AuthorDetails,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct LiveChatSnippet {
    #[serde(rename = "liveChatId")]
    pub live_chat_id: String,
    #[serde(rename = "publishedAt")]
    pub published_at: chrono::DateTime<chrono::Utc>,
    #[serde(rename = "displayMessage")]
    pub display_message: String,
    #[serde(default)]
    #[serde(rename = "superChatDetails")]
    pub super_chat_details: Option<SuperChatDetails>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct SuperChatDetails {
    #[serde(rename = "amountDisplayString")]
    pub amount_display_string: String,
    #[serde(rename = "amountMicros")]
    pub amount_micros: Option<u64>,
    #[serde(rename = "currency")]
    pub currency: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct AuthorDetails {
    #[serde(rename = "channelId")]
    pub channel_id: Option<String>,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "isChatModerator")]
    pub is_chat_moderator: Option<bool>,
    #[serde(rename = "isChatOwner")]
    pub is_chat_owner: Option<bool>,
    #[serde(rename = "isChatSponsor")]
    pub is_chat_sponsor: Option<bool>,
}

impl LiveChatMessageItem {
    pub fn to_normalized(&self) -> NormalizedMessage {
        let mut priority = Priority::Normal;
        if let Some(details) = &self.snippet.super_chat_details {
            if details.amount_micros.unwrap_or_default() > 0 {
                priority = Priority::Paid;
            }
        }
        if self.author_details.is_chat_moderator.unwrap_or(false)
            || self.author_details.is_chat_owner.unwrap_or(false)
        {
            priority = Priority::Moderator;
        }
        let metadata = serde_json::json!({
            "super_chat": self.snippet.super_chat_details,
        });
        NormalizedMessage {
            id: uuid::Uuid::new_v4(),
            platform: Platform::YouTube,
            channel: self.snippet.live_chat_id.clone(),
            user_id: self.author_details.channel_id.clone(),
            username: self.author_details.display_name.clone(),
            priority,
            content: crate::message::MessageContent::Text(self.snippet.display_message.clone()),
            metadata,
            timestamp: self.snippet.published_at,
        }
    }
}

pub fn parse_live_chat_messages(json: &str) -> Result<LiveChatMessagesResponse> {
    let resp: LiveChatMessagesResponse = serde_json::from_str(json)
        .with_context(|| "failed to deserialize liveChatMessages response")?;
    Ok(resp)
}

pub fn extract_messages(json: &str) -> Result<Vec<NormalizedMessage>> {
    let resp = parse_live_chat_messages(json)?;
    Ok(resp
        .items
        .into_iter()
        .map(|item| item.to_normalized())
        .collect())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn parse_super_chat() {
        let data = json!({
            "items": [
                {
                    "id": "abc",
                    "snippet": {
                        "liveChatId": "chat123",
                        "publishedAt": "2024-08-01T00:00:00Z",
                        "displayMessage": "Hello stream",
                        "superChatDetails": {
                            "amountDisplayString": "$5.00",
                            "amountMicros": 5_000_000,
                            "currency": "USD"
                        }
                    },
                    "authorDetails": {
                        "channelId": "user123",
                        "displayName": "Viewer",
                        "isChatModerator": false,
                        "isChatOwner": false,
                        "isChatSponsor": true
                    }
                }
            ],
            "nextPageToken": "token",
            "pollingIntervalMillis": 2000
        });
        let json = serde_json::to_string(&data).unwrap();
        let messages = extract_messages(&json).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content.as_text().unwrap(), "Hello stream");
        matches!(messages[0].priority, Priority::Paid);
    }
}
