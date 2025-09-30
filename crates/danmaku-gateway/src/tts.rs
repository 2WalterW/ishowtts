use anyhow::{Context, Result};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::time::Duration;

use crate::config::TtsConfig;

#[derive(Debug, Clone, Serialize)]
pub struct TtsRequestPayload {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TtsResponsePayload {
    pub request_id: uuid::Uuid,
    pub voice_id: String,
    pub sample_rate: u32,
    pub audio_base64: String,
    pub format: String,
    pub waveform_len: usize,
}

#[derive(Clone)]
pub struct TtsClient {
    config: TtsConfig,
    http: reqwest::Client,
}

impl TtsClient {
    pub fn new(config: TtsConfig) -> Result<Self> {
        let mut builder = reqwest::Client::builder();
        if let Some(timeout) = config.timeout_secs {
            builder = builder.timeout(Duration::from_secs(timeout));
        }
        let http = builder.build()?;
        Ok(Self { config, http })
    }

    pub async fn synthesize(&self, text: &str) -> Result<TtsResponsePayload> {
        let payload = TtsRequestPayload {
            text: text.to_string(),
            voice_id: self.config.voice_id.clone(),
        };
        let request = self.http.post(&self.config.endpoint).json(&payload);
        let response = request
            .send()
            .await
            .with_context(|| "failed to send TTS request")?;
        if response.status() != StatusCode::OK {
            return Err(anyhow::anyhow!(
                "tts server returned status {}",
                response.status()
            ));
        }
        let payload = response
            .json::<TtsResponsePayload>()
            .await
            .with_context(|| "failed to parse TTS response JSON")?;
        Ok(payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TtsConfig;

    #[test]
    fn serialize_request_payload() {
        let payload = TtsRequestPayload {
            text: "hello".into(),
            voice_id: Some("walter".into()),
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"voice_id\":"));
    }

    #[tokio::test]
    async fn tts_client_handles_response() {
        let server = httpmock::MockServer::start_async().await;
        let response = serde_json::json!({
            "request_id": uuid::Uuid::new_v4(),
            "voice_id": "walter",
            "sample_rate": 24000,
            "audio_base64": "UklGRg==",
            "format": "audio/wav",
            "waveform_len": 10
        });
        let mock = server.mock(|when, then| {
            when.method(httpmock::Method::POST).path("/api/tts");
            then.status(200).json_body(response.clone());
        });

        let client = TtsClient::new(TtsConfig {
            endpoint: format!("{}/api/tts", server.base_url()),
            voice_id: Some("walter".into()),
            timeout_secs: Some(5),
        })
        .unwrap();

        let resp = client.synthesize("hello").await.unwrap();
        assert_eq!(resp.voice_id, "walter");
        mock.assert_async().await;
    }
}
