use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct GatewayConfig {
    #[serde(default)]
    pub queue: QueueConfig,
    #[serde(default)]
    pub filter: FilterConfig,
    #[serde(default)]
    pub tts: TtsConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct QueueConfig {
    #[serde(default = "default_queue_capacity")]
    pub capacity: usize,
    #[serde(default = "default_rate_limit_per_sec")]
    pub rate_limit_per_sec: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FilterConfig {
    #[serde(default = "default_max_words")]
    pub max_words: usize,
    #[serde(default = "default_max_chars")]
    pub max_chars: usize,
    #[serde(default)]
    pub banned_keywords: Vec<String>,
    #[serde(default)]
    pub allow_links: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TtsConfig {
    #[serde(default = "default_tts_endpoint")]
    pub endpoint: String,
    #[serde(default)]
    pub voice_id: Option<String>,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            queue: QueueConfig::default(),
            filter: FilterConfig::default(),
            tts: TtsConfig::default(),
        }
    }
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            capacity: default_queue_capacity(),
            rate_limit_per_sec: default_rate_limit_per_sec(),
        }
    }
}

impl Default for FilterConfig {
    fn default() -> Self {
        Self {
            max_words: default_max_words(),
            max_chars: default_max_chars(),
            banned_keywords: Vec::new(),
            allow_links: false,
        }
    }
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            endpoint: default_tts_endpoint(),
            voice_id: None,
            timeout_secs: Some(15),
        }
    }
}

fn default_queue_capacity() -> usize {
    512
}

fn default_rate_limit_per_sec() -> f32 {
    1.5
}

fn default_max_words() -> usize {
    77
}

fn default_max_chars() -> usize {
    280
}

fn default_tts_endpoint() -> String {
    "http://127.0.0.1:27121/api/tts".to_string()
}

impl GatewayConfig {
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref()).with_context(|| {
            format!("failed to read gateway config: {}", path.as_ref().display())
        })?;
        let config: GatewayConfig =
            toml::from_str(&content).with_context(|| "failed to parse gateway config")?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_gateway_config() {
        let toml = r#"
[queue]
capacity = 100
rate_limit_per_sec = 2.0

[filter]
max_words = 50
banned_keywords = ["bad"]

[tts]
endpoint = "http://localhost:8080/tts"
voice_id = "walter"
"#;
        let cfg: GatewayConfig = toml::from_str(toml).unwrap();
        assert_eq!(cfg.queue.capacity, 100);
        assert_eq!(cfg.filter.max_words, 50);
        assert_eq!(cfg.tts.voice_id.as_deref(), Some("walter"));
    }
}
