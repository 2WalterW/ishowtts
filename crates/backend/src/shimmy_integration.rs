use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use shimmy::engine::{GenOptions, InferenceEngine, LoadedModel, ModelSpec};
use tracing::instrument;

use tts_engine::{TtsRequest, TtsResponse};

use crate::synth::Synthesizer;

#[derive(Clone)]
pub struct F5ShimmyEngine {
    synthesizer: Arc<Synthesizer>,
}

impl F5ShimmyEngine {
    pub fn new(synthesizer: Arc<Synthesizer>) -> Self {
        Self { synthesizer }
    }
}

struct F5LoadedModel {
    synthesizer: Arc<Synthesizer>,
    default_voice: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ShimmyTtsPayload {
    text: String,
    #[serde(default)]
    voice_id: Option<String>,
    #[serde(default)]
    speed: Option<f32>,
    #[serde(default)]
    target_rms: Option<f32>,
    #[serde(default)]
    cross_fade_duration: Option<f32>,
    #[serde(default)]
    sway_sampling_coef: Option<f32>,
    #[serde(default)]
    cfg_strength: Option<f32>,
    #[serde(default)]
    nfe_step: Option<u32>,
    #[serde(default)]
    fix_duration: Option<f32>,
    #[serde(default)]
    remove_silence: Option<bool>,
    #[serde(default)]
    seed: Option<u64>,
}

#[derive(Debug, Serialize)]
struct ShimmyTtsEnvelope {
    response: TtsResponse,
}

#[async_trait]
impl InferenceEngine for F5ShimmyEngine {
    async fn load(&self, spec: &ModelSpec) -> Result<Box<dyn LoadedModel>> {
        let default_voice = extract_default_voice(spec);
        Ok(Box::new(F5LoadedModel {
            synthesizer: self.synthesizer.clone(),
            default_voice,
        }))
    }
}

fn extract_default_voice(spec: &ModelSpec) -> Option<String> {
    spec.template.as_ref().and_then(|template| {
        template
            .split(',')
            .find_map(|segment| segment.trim().strip_prefix("voice:"))
            .map(|s| s.trim().to_string())
    })
}

#[async_trait]
impl LoadedModel for F5LoadedModel {
    #[instrument(skip(self, prompt, on_token), fields(default_voice = ?self.default_voice))]
    async fn generate(
        &self,
        prompt: &str,
        _opts: GenOptions,
        mut on_token: Option<Box<dyn FnMut(String) + Send>>,
    ) -> Result<String> {
        let payload: ShimmyTtsPayload = if prompt.trim_start().starts_with('{') {
            serde_json::from_str(prompt).context("failed to parse shimmy TTS payload JSON")?
        } else {
            ShimmyTtsPayload {
                text: prompt.to_string(),
                voice_id: None,
                speed: None,
                target_rms: None,
                cross_fade_duration: None,
                sway_sampling_coef: None,
                cfg_strength: None,
                nfe_step: None,
                fix_duration: None,
                remove_silence: None,
                seed: None,
            }
        };

        let voice_id = payload
            .voice_id
            .or_else(|| self.default_voice.clone())
            .context("voice_id missing in request and no default voice configured")?;

        let request = TtsRequest {
            text: payload.text,
            voice_id,
            speed: payload.speed,
            target_rms: payload.target_rms,
            cross_fade_duration: payload.cross_fade_duration,
            sway_sampling_coef: payload.sway_sampling_coef,
            cfg_strength: payload.cfg_strength,
            nfe_step: payload.nfe_step,
            fix_duration: payload.fix_duration,
            remove_silence: payload.remove_silence,
            seed: payload.seed,
        };

        let response = self.synthesizer.synthesize(request).await?;
        let envelope = ShimmyTtsEnvelope { response };
        let serialized = serde_json::to_string(&envelope)?;

        if let Some(ref mut callback) = on_token {
            callback(serialized.clone());
        }

        Ok(serialized)
    }
}
