use std::sync::Arc;

use anyhow::Result;
use tokio::sync::{mpsc, Mutex};

use danmaku::message::NormalizedMessage;

use crate::config::QueueConfig;
use crate::filter::{FilteredMessage, MessageFilter, RateLimiter};

pub struct MessageQueue {
    filter: MessageFilter,
    tx: mpsc::Sender<FilteredMessage>,
    limiter: Arc<Mutex<RateLimiter>>,
}

impl MessageQueue {
    pub fn new(
        filter: MessageFilter,
        config: QueueConfig,
    ) -> (Self, mpsc::Receiver<FilteredMessage>) {
        let (tx, rx) = mpsc::channel(config.capacity);
        let limiter = Arc::new(Mutex::new(RateLimiter::new(config.rate_limit_per_sec)));
        (
            Self {
                filter,
                tx,
                limiter,
            },
            rx,
        )
    }

    pub async fn enqueue(&self, message: &NormalizedMessage) -> Result<bool> {
        if let Some(filtered) = self.filter.sanitize(message) {
            let mut limiter = self.limiter.lock().await;
            limiter.throttle().await;
            drop(limiter);
            if self.tx.send(filtered.clone()).await.is_ok() {
                tracing::trace!(
                    target = "ishowtts::danmaku",
                    channel = %filtered.source.channel,
                    user = %filtered.source.username,
                    text = %filtered.sanitized_text,
                    "enqueued filtered message"
                );
                return Ok(true);
            }
        } else {
            tracing::trace!(
                target = "ishowtts::danmaku",
                channel = %message.channel,
                user = %message.username,
                "message dropped by filter"
            );
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::FilterConfig;
    use danmaku::message::{Platform, Priority};

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

    #[tokio::test]
    async fn enqueue_and_receive() {
        let filter = MessageFilter::new(FilterConfig::default()).unwrap();
        let (queue, mut rx) = MessageQueue::new(filter, QueueConfig::default());
        assert!(queue.enqueue(&make_message("hello world")).await.unwrap());
        let msg = rx.recv().await.unwrap();
        assert_eq!(msg.sanitized_text, "hello world");
    }
}
