use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Platform {
    Twitch,
    YouTube,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Priority {
    Gift,
    Paid,
    Moderator,
    Mention,
    Normal,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Normal
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageContent {
    Text(String),
    System(String),
}

impl MessageContent {
    pub fn as_text(&self) -> Option<&str> {
        match self {
            MessageContent::Text(ref s) => Some(s),
            MessageContent::System(_) => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NormalizedMessage {
    pub id: Uuid,
    pub platform: Platform,
    pub channel: String,
    pub user_id: Option<String>,
    pub username: String,
    pub priority: Priority,
    pub content: MessageContent,
    pub metadata: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl NormalizedMessage {
    pub fn new_text(
        platform: Platform,
        channel: impl Into<String>,
        user_id: Option<String>,
        username: impl Into<String>,
        priority: Priority,
        text: impl Into<String>,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            platform,
            channel: channel.into(),
            user_id,
            username: username.into(),
            priority,
            content: MessageContent::Text(text.into()),
            metadata,
            timestamp: chrono::Utc::now(),
        }
    }
}

impl fmt::Display for NormalizedMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{platform:?}]#{channel} {user}: {content}",
            platform = self.platform,
            channel = self.channel,
            user = self.username,
            content = match &self.content {
                MessageContent::Text(s) => s,
                MessageContent::System(s) => s,
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalized_message_display() {
        let msg = NormalizedMessage::new_text(
            Platform::Twitch,
            "test",
            Some("123".into()),
            "user",
            Priority::Normal,
            "hello",
            serde_json::Value::Null,
        );
        assert!(format!("{msg}").contains("hello"));
    }
}
