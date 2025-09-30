use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use tokio::sync::Semaphore;
use tracing::instrument;

use parking_lot::RwLock;

use tts_engine::{
    EngineKind, TtsEngine, TtsRequest, TtsResponse, VoiceDescriptor, VoiceOverrideUpdate,
};

pub struct Synthesizer {
    engines: HashMap<EngineKind, Arc<dyn TtsEngine>>,
    voice_map: RwLock<HashMap<String, VoiceDescriptor>>,
    baseline_map: HashMap<String, VoiceBaseline>,
    limiter: Arc<Semaphore>,
}

#[derive(Clone)]
pub struct VoiceBaseline {
    pub reference_audio: PathBuf,
    pub reference_text: Option<String>,
}

impl Synthesizer {
    pub fn new(engines: Vec<Arc<dyn TtsEngine>>, max_parallel: usize) -> Result<Self> {
        let limiter = Arc::new(Semaphore::new(max_parallel.max(1)));

        let mut engine_map: HashMap<EngineKind, Arc<dyn TtsEngine>> = HashMap::new();
        let mut voice_map: HashMap<String, VoiceDescriptor> = HashMap::new();
        let mut baseline_map: HashMap<String, VoiceBaseline> = HashMap::new();

        for engine in engines {
            let kind = engine.kind();
            if engine_map.contains_key(&kind) {
                anyhow::bail!("engine '{}' registered more than once", kind);
            }
            let mut duplicates = Vec::new();
            for descriptor in engine.voice_descriptors() {
                if voice_map.contains_key(&descriptor.id) {
                    duplicates.push(descriptor.id.clone());
                    continue;
                }
                if let Some((audio_path, reference_text)) = engine.resolve_reference(&descriptor.id)
                {
                    baseline_map.insert(
                        descriptor.id.clone(),
                        VoiceBaseline {
                            reference_audio: audio_path,
                            reference_text,
                        },
                    );
                }
                voice_map.insert(descriptor.id.clone(), descriptor);
            }
            if !duplicates.is_empty() {
                anyhow::bail!(
                    "duplicate voice ids detected for engine '{}': {}",
                    kind,
                    duplicates.join(", ")
                );
            }
            engine_map.insert(kind, engine);
        }

        Ok(Self {
            engines: engine_map,
            voice_map: RwLock::new(voice_map),
            baseline_map,
            limiter,
        })
    }

    #[instrument(skip(self, request))]
    pub async fn synthesize(&self, request: TtsRequest) -> Result<TtsResponse> {
        let _permit = self
            .limiter
            .acquire()
            .await
            .expect("semaphore closed unexpectedly");
        let voice_id = request.voice_id.clone();
        let descriptor = {
            let voices = self.voice_map.read();
            voices
                .get(&voice_id)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("voice '{}' is not registered", voice_id))?
        };
        let engine = self.engines.get(&descriptor.engine).ok_or_else(|| {
            anyhow::anyhow!(
                "engine '{}' not initialised for voice '{}'",
                descriptor.engine,
                voice_id
            )
        })?;
        engine.synthesize(request).await
    }

    pub fn voices(&self) -> Vec<VoiceDescriptor> {
        let voices_guard = self.voice_map.read();
        let mut voices: Vec<VoiceDescriptor> = voices_guard.values().cloned().collect();
        voices.sort_by(|a, b| a.id.cmp(&b.id));
        voices
    }

    pub fn voice_descriptor(&self, voice_id: &str) -> Option<VoiceDescriptor> {
        self.voice_map.read().get(voice_id).cloned()
    }

    pub async fn warmup_voice(&self, voice_id: &str, text: &str) -> Result<()> {
        let request = TtsRequest {
            text: text.to_string(),
            voice_id: voice_id.to_string(),
            speed: None,
            target_rms: None,
            cross_fade_duration: None,
            sway_sampling_coef: None,
            cfg_strength: None,
            nfe_step: None,
            fix_duration: None,
            remove_silence: None,
            seed: None,
        };

        let _ = self.synthesize(request).await?;
        Ok(())
    }

    pub fn apply_override(
        &self,
        engine: EngineKind,
        voice_id: &str,
        update: VoiceOverrideUpdate,
    ) -> Result<()> {
        if let Some(engine_impl) = self.engines.get(&engine) {
            engine_impl.apply_override(voice_id, update.clone())?;
            if let Some(descriptor) = self.voice_map.write().get_mut(voice_id) {
                if let Some(text) = update.reference_text {
                    descriptor.reference_text = Some(text);
                } else if let Some(baseline) = self.baseline_map.get(voice_id) {
                    descriptor.reference_text = baseline.reference_text.clone();
                }
            }
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "engine '{}' not initialised for voice '{}'",
                engine,
                voice_id
            ))
        }
    }

    pub fn baseline(&self, voice_id: &str) -> Option<VoiceBaseline> {
        self.baseline_map.get(voice_id).cloned()
    }
}

impl Clone for Synthesizer {
    fn clone(&self) -> Self {
        Self {
            engines: self.engines.clone(),
            voice_map: RwLock::new(self.voice_map.read().clone()),
            baseline_map: self.baseline_map.clone(),
            limiter: self.limiter.clone(),
        }
    }
}
