use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tracing::debug;
use tts_engine::EngineKind;

#[derive(Clone, Debug)]
pub struct OverrideAudio {
    pub bytes: Vec<u8>,
    pub extension: Option<String>,
}

#[derive(Clone, Debug)]
pub struct VoiceOverrideRecord {
    pub voice_id: String,
    pub engine: EngineKind,
    pub reference_audio: Option<PathBuf>,
    pub reference_text: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Default, Serialize, Deserialize)]
struct OverridesFile {
    entries: HashMap<String, StoredOverride>,
}

#[derive(Clone, Serialize, Deserialize)]
struct StoredOverride {
    engine: EngineKind,
    reference_audio: Option<String>,
    reference_text: Option<String>,
    updated_at: Option<DateTime<Utc>>,
}

fn make_key(voice_id: &str, engine: EngineKind) -> String {
    format!("{}::{}", voice_id, engine.as_str())
}

pub struct VoiceOverrideStore {
    base_dir: PathBuf,
    audio_dir: PathBuf,
    data_path: PathBuf,
    state: Mutex<OverridesFile>,
}

impl VoiceOverrideStore {
    pub fn load(base_dir: impl AsRef<Path>) -> Result<Self> {
        let base_dir_input = base_dir.as_ref();
        let base_dir = if base_dir_input.is_absolute() {
            base_dir_input.to_path_buf()
        } else {
            env::current_dir()
                .with_context(|| "failed to resolve current working directory")?
                .join(base_dir_input)
        };
        let audio_dir = base_dir.join("audio");
        let data_path = base_dir.join("overrides.json");
        fs::create_dir_all(&audio_dir).with_context(|| {
            format!(
                "failed to create overrides directory at {}",
                audio_dir.display()
            )
        })?;

        let state = if data_path.exists() {
            let bytes = fs::read(&data_path).with_context(|| {
                format!("failed to read overrides file {}", data_path.display())
            })?;
            serde_json::from_slice(&bytes).with_context(|| "failed to parse overrides.json")?
        } else {
            OverridesFile::default()
        };

        Ok(Self {
            base_dir,
            audio_dir,
            data_path,
            state: Mutex::new(state),
        })
    }

    pub fn get(&self, voice_id: &str, engine: EngineKind) -> Option<VoiceOverrideRecord> {
        let state = self.state.lock();
        let key = make_key(voice_id, engine);
        state
            .entries
            .get(&key)
            .map(|entry| self.record_from_entry(voice_id, entry.clone()))
    }

    pub fn all(&self) -> Vec<VoiceOverrideRecord> {
        let state = self.state.lock();
        state
            .entries
            .iter()
            .filter_map(|(key, entry)| {
                split_key(key).map(|voice_id| self.record_from_entry(voice_id, entry.clone()))
            })
            .collect()
    }

    pub fn set(
        &self,
        voice_id: &str,
        engine: EngineKind,
        temp_audio: Option<OverrideAudio>,
        reference_text: Option<String>,
    ) -> Result<VoiceOverrideRecord> {
        let mut state = self.state.lock();
        let key = make_key(voice_id, engine);
        let mut entry = state.entries.get(&key).cloned().unwrap_or(StoredOverride {
            engine,
            reference_audio: None,
            reference_text: None,
            updated_at: None,
        });

        if let Some(audio) = temp_audio {
            fs::create_dir_all(&self.audio_dir).with_context(|| {
                format!(
                    "failed to create overrides audio directory at {}",
                    self.audio_dir.display()
                )
            })?;

            let final_ext = audio
                .extension
                .as_deref()
                .map(|ext| ext.trim_matches('.').to_ascii_lowercase())
                .filter(|ext| !ext.is_empty())
                .or_else(|| infer_audio_extension_from_bytes(&audio.bytes))
                .filter(|ext| {
                    matches!(
                        ext.as_str(),
                        "wav" | "mp3" | "flac" | "ogg" | "m4a" | "opus"
                    )
                })
                .unwrap_or_else(|| "wav".to_string());
            let file_name = format!("{}_{}.{}", voice_id, engine.as_str(), final_ext);
            let target_path = self.audio_dir.join(file_name);
            fs::write(&target_path, &audio.bytes).with_context(|| {
                format!(
                    "failed to persist override audio to {}",
                    target_path.display()
                )
            })?;
            let metadata = fs::metadata(&target_path).with_context(|| {
                format!(
                    "override audio written but could not read metadata for {}",
                    target_path.display()
                )
            })?;
            debug!(
                target = "ishowtts::voice_overrides",
                voice = %voice_id,
                engine = %engine,
                path = %target_path.display(),
                bytes_written = audio.bytes.len(),
                bytes_on_disk = metadata.len(),
                "override audio persisted"
            );
            let rel = target_path
                .strip_prefix(&self.base_dir)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| target_path.to_string_lossy().to_string());
            entry.reference_audio = Some(rel);
        }

        if let Some(text) = reference_text.clone() {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                entry.reference_text = None;
            } else {
                entry.reference_text = Some(trimmed.to_string());
            }
        }

        entry.updated_at = Some(Utc::now());
        state.entries.insert(key.clone(), entry.clone());
        self.persist(&state)?;

        Ok(self.record_from_entry(voice_id, entry))
    }

    pub fn remove(
        &self,
        voice_id: &str,
        engine: EngineKind,
    ) -> Result<Option<VoiceOverrideRecord>> {
        let mut state = self.state.lock();
        let key = make_key(voice_id, engine);
        let removed = state.entries.remove(&key);
        if let Some(entry) = removed.as_ref() {
            if let Some(rel) = &entry.reference_audio {
                let path = self.base_dir.join(rel);
                let _ = fs::remove_file(path);
            }
        }
        self.persist(&state)?;
        Ok(removed.map(|entry| self.record_from_entry(voice_id, entry)))
    }

    fn persist(&self, state: &OverridesFile) -> Result<()> {
        let json = serde_json::to_vec_pretty(state)?;
        fs::write(&self.data_path, json).with_context(|| {
            format!(
                "failed to write overrides file {}",
                self.data_path.display()
            )
        })
    }

    fn record_from_entry(&self, voice_id: &str, entry: StoredOverride) -> VoiceOverrideRecord {
        let audio_path = entry
            .reference_audio
            .as_ref()
            .map(|rel| self.base_dir.join(rel));
        VoiceOverrideRecord {
            voice_id: voice_id.to_string(),
            engine: entry.engine,
            reference_audio: audio_path,
            reference_text: entry.reference_text,
            updated_at: entry.updated_at,
        }
    }
}

fn split_key(key: &str) -> Option<&str> {
    key.split_once("::").map(|(voice_id, _)| voice_id)
}

fn infer_audio_extension_from_bytes(_bytes: &[u8]) -> Option<String> {
    None
}
