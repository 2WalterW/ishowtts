use anyhow::Result;
use regex::Regex;
use serde::Serialize;
use tokio::time::{Duration, Instant};

use danmaku::message::{MessageContent, NormalizedMessage};

use crate::config::FilterConfig;

#[derive(Debug, Clone, Serialize)]
pub struct FilteredMessage {
    pub source: NormalizedMessage,
    pub sanitized_text: String,
    pub accepted_at: chrono::DateTime<chrono::Utc>,
}

pub struct MessageFilter {
    config: FilterConfig,
    banned_regex: Option<Regex>,
    link_regex: Regex,
}

impl MessageFilter {
    pub fn new(config: FilterConfig) -> Result<Self> {
        let banned_regex = if config.banned_keywords.is_empty() {
            None
        } else {
            let pattern = config
                .banned_keywords
                .iter()
                .map(|kw| regex::escape(kw))
                .collect::<Vec<_>>()
                .join("|");
            Some(Regex::new(&format!("(?i)({})", pattern))?)
        };
        let link_regex = Regex::new(r"https?://|www\.").expect("invalid default link regex");
        Ok(Self {
            config,
            banned_regex,
            link_regex,
        })
    }

    pub fn sanitize(&self, message: &NormalizedMessage) -> Option<FilteredMessage> {
        let text = match &message.content {
            MessageContent::Text(t) => t,
            MessageContent::System(_) => return None,
        };
        let mut sanitized = text.replace(['\r', '\n'], " ").trim().to_string();
        if sanitized.is_empty() {
            return None;
        }

        if !self.config.allow_links && self.link_regex.is_match(&sanitized) {
            return None;
        }

        if let Some(regex) = &self.banned_regex {
            if regex.is_match(&sanitized) {
                return None;
            }
        }

        let mut words: Vec<&str> = sanitized.split_whitespace().collect();
        if words.len() > self.config.max_words {
            words.truncate(self.config.max_words);
            sanitized = words.join(" ");
        }
        if sanitized.len() > self.config.max_chars {
            sanitized.truncate(self.config.max_chars);
        }

        Some(FilteredMessage {
            source: message.clone(),
            sanitized_text: sanitized,
            accepted_at: chrono::Utc::now(),
        })
    }
}

#[derive(Debug)]
pub struct RateLimiter {
    last_emit: Option<Instant>,
    interval: Duration,
}

impl RateLimiter {
    pub fn new(rate_per_sec: f32) -> Self {
        let interval = if rate_per_sec <= 0.0 {
            Duration::from_secs(1)
        } else {
            Duration::from_secs_f32(1.0 / rate_per_sec)
        };
        Self {
            last_emit: None,
            interval,
        }
    }

    pub async fn throttle(&mut self) {
        if let Some(last) = self.last_emit {
            let elapsed = last.elapsed();
            if elapsed < self.interval {
                tokio::time::sleep(self.interval - elapsed).await;
            }
        }
        self.last_emit = Some(Instant::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use danmaku::message::{NormalizedMessage, Platform, Priority};

    fn make_message(text: &str) -> NormalizedMessage {
        NormalizedMessage::new_text(
            Platform::Twitch,
            "channel",
            Some("u1".into()),
            "user",
            Priority::Normal,
            text,
            serde_json::Value::Null,
        )
    }

    #[test]
    fn filter_rejects_links_and_keywords() {
        let filter = MessageFilter::new(FilterConfig {
            max_words: 10,
            max_chars: 50,
            banned_keywords: vec!["spoiler".into()],
            allow_links: false,
        })
        .unwrap();
        assert!(filter
            .sanitize(&make_message("check http://example.com"))
            .is_none());
        assert!(filter
            .sanitize(&make_message("this is a spoiler message"))
            .is_none());
        assert!(filter.sanitize(&make_message("nice message")).is_some());
    }

    #[test]
    fn filter_truncates_words() {
        let filter = MessageFilter::new(FilterConfig {
            max_words: 3,
            max_chars: 100,
            banned_keywords: vec![],
            allow_links: true,
        })
        .unwrap();
        let msg = filter
            .sanitize(&make_message("one two three four five"))
            .unwrap();
        assert_eq!(msg.sanitized_text.split_whitespace().count(), 3);
    }
}
