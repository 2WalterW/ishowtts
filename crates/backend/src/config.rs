use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use config as config_rs;
use danmaku::config::DanmakuConfig;
use danmaku_gateway::config::GatewayConfig as DanmakuGatewayConfig;
use serde::Deserialize;
use shimmy::model_registry::ModelEntry;
use tts_engine::{F5EngineConfig, IndexTtsEngineConfig, IndexTtsVllmEngineConfig};

#[derive(Clone, Debug, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_bind_addr")]
    pub bind_addr: String,
    #[serde(default)]
    pub api: ApiConfig,
    #[serde(default)]
    pub default_voice: Option<String>,
    pub f5: F5EngineConfig,
    #[serde(default)]
    pub index_tts: Option<IndexTtsEngineConfig>,
    #[serde(default)]
    pub index_tts_vllm: Option<IndexTtsVllmEngineConfig>,
    #[serde(default)]
    pub shimmy: ShimmyConfig,
    #[serde(default)]
    pub danmaku: Option<DanmakuConfig>,
    #[serde(default)]
    pub danmaku_gateway: Option<DanmakuGatewayConfig>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ApiConfig {
    #[serde(default = "default_max_parallel")]
    pub max_parallel: usize,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            max_parallel: default_max_parallel(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct ShimmyConfig {
    #[serde(default = "ShimmyConfig::default_model_name")]
    pub model_name: String,
    #[serde(default)]
    pub template: Option<String>,
    #[serde(default)]
    pub ctx_len: Option<usize>,
    #[serde(default)]
    pub n_threads: Option<i32>,
    #[serde(default)]
    pub extra_models: Vec<ShimmyAdditionalModel>,
}

impl Default for ShimmyConfig {
    fn default() -> Self {
        Self {
            model_name: Self::default_model_name(),
            template: Some("text-to-speech".to_string()),
            ctx_len: Some(1024),
            n_threads: None,
            extra_models: Vec::new(),
        }
    }
}

impl ShimmyConfig {
    fn default_model_name() -> String {
        "f5-tts".to_string()
    }

    pub fn to_model_entry(&self, base_path: PathBuf) -> ModelEntry {
        ModelEntry {
            name: self.model_name.clone(),
            base_path,
            lora_path: None,
            template: self.template.clone(),
            ctx_len: self.ctx_len,
            n_threads: self.n_threads,
        }
    }

    pub fn to_model_entries(&self, base_path: PathBuf) -> Vec<ModelEntry> {
        let mut entries = Vec::with_capacity(1 + self.extra_models.len());
        entries.push(self.to_model_entry(base_path));
        entries.extend(
            self.extra_models
                .iter()
                .map(ShimmyAdditionalModel::to_model_entry),
        );
        entries
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct ShimmyAdditionalModel {
    pub name: String,
    pub base_path: PathBuf,
    #[serde(default)]
    pub template: Option<String>,
    #[serde(default)]
    pub ctx_len: Option<usize>,
    #[serde(default)]
    pub n_threads: Option<i32>,
}

impl ShimmyAdditionalModel {
    fn to_model_entry(&self) -> ModelEntry {
        ModelEntry {
            name: self.name.clone(),
            base_path: self.base_path.clone(),
            lora_path: None,
            template: self.template.clone(),
            ctx_len: self.ctx_len,
            n_threads: self.n_threads,
        }
    }

    fn rebase(&mut self, base: &Path) -> Result<()> {
        let label = format!("Shimmy model {} base path", self.name);
        self.base_path = normalize_required(base, &self.base_path, &label)?;
        Ok(())
    }
}

fn default_bind_addr() -> String {
    "0.0.0.0:27121".to_string()
}

fn default_max_parallel() -> usize {
    2
}

impl AppConfig {
    pub fn load(path: PathBuf) -> Result<(Self, PathBuf)> {
        let config_dir = path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));

        let builder = config_rs::Config::builder()
            .add_source(config_rs::File::from(path.clone()))
            .add_source(config_rs::Environment::with_prefix("ISHOWTTS").separator("__"));

        let cfg = builder
            .build()
            .with_context(|| format!("failed to load configuration from {}", path.display()))?;

        let mut app_cfg: AppConfig = cfg
            .try_deserialize()
            .context("failed to deserialize configuration")?;
        app_cfg.rebase_paths(&config_dir)?;
        Ok((app_cfg, config_dir))
    }

    fn rebase_paths(&mut self, base: &Path) -> Result<()> {
        // Top-level F5 paths
        self.f5.python_package_path =
            normalize_required(base, &self.f5.python_package_path, "F5 python package path")?;
        if let Some(ref mut ckpt) = self.f5.ckpt_file {
            *ckpt = normalize_optional(base, ckpt)?;
        }
        if let Some(ref mut vocab) = self.f5.vocab_file {
            *vocab = normalize_optional(base, vocab)?;
        }
        if let Some(ref mut vocoder) = self.f5.vocoder_local_path {
            *vocoder = normalize_optional(base, vocoder)?;
        }
        if let Some(ref mut cache) = self.f5.hf_cache_dir {
            *cache = normalize_optional(base, cache)?;
        }

        for profile in &mut self.f5.voices {
            let label = format!("reference audio for voice {}", profile.id);
            profile.reference_audio = normalize_required(base, &profile.reference_audio, &label)?;
        }

        for extra in &mut self.shimmy.extra_models {
            extra.rebase(base)?;
        }

        if let Some(ref mut index_cfg) = self.index_tts {
            index_cfg.python_package_path = normalize_required(
                base,
                &index_cfg.python_package_path,
                "IndexTTS python package path",
            )?;
            index_cfg.config_file =
                normalize_required(base, &index_cfg.config_file, "IndexTTS config file path")?;
            index_cfg.model_dir =
                normalize_required(base, &index_cfg.model_dir, "IndexTTS model directory")?;

            for voice in &mut index_cfg.voices {
                let label = format!("reference audio for IndexTTS voice {}", voice.id);
                voice.reference_audio = normalize_required(base, &voice.reference_audio, &label)?;
                if let Some(ref mut emo_audio) = voice.emo_audio {
                    *emo_audio = normalize_required(
                        base,
                        emo_audio,
                        &format!("emotion audio for IndexTTS voice {}", voice.id),
                    )?;
                }
            }
        }
        Ok(())
    }

    pub fn shimmy_entries(&self) -> Vec<ModelEntry> {
        self.shimmy
            .to_model_entries(self.f5.python_package_path.clone())
    }
}

fn normalize_required(base: &Path, path: &Path, label: &str) -> Result<PathBuf> {
    let candidate = absolute_path(base, path);
    candidate
        .canonicalize()
        .with_context(|| format!("{label} not found at {}", candidate.display()))
}

fn normalize_optional(base: &Path, path: &Path) -> Result<PathBuf> {
    let candidate = absolute_path(base, path);
    Ok(candidate.canonicalize().unwrap_or(candidate))
}

fn absolute_path(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}
