use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct DanmakuConfig {
    #[serde(default)]
    pub twitch: Option<TwitchConfig>,
    #[serde(default)]
    pub youtube: Option<YouTubeConfig>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct TwitchConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub client_secret: Option<String>,
    #[serde(default)]
    pub bot_username: Option<String>,
    #[serde(default)]
    pub oauth_token: Option<String>,
    #[serde(default)]
    pub channels: Vec<String>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct YouTubeConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub client_secret: Option<String>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub channel_id: Option<String>,
}

impl DanmakuConfig {
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref())
            .with_context(|| format!("failed to read config: {}", path.as_ref().display()))?;
        let config: DanmakuConfig =
            toml::from_str(&content).with_context(|| "failed to parse danmaku config as TOML")?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_config() {
        let toml = r#"
[twitch]
enabled = true
client_id = "abc"
channels = ["foo", "bar"]

[youtube]
enabled = true
refresh_token = "refresh"
"#;
        let cfg: DanmakuConfig = toml::from_str(toml).unwrap();
        assert!(cfg.twitch.unwrap().enabled);
        assert_eq!(cfg.youtube.unwrap().refresh_token.unwrap(), "refresh");
    }
}
