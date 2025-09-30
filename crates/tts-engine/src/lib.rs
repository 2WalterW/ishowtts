use std::str::FromStr;
use std::{
    collections::{HashMap, HashSet},
    ffi::OsString,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use hound::{SampleFormat, WavSpec, WavWriter};
use numpy::{PyArray1, PyArray2, PyArrayMethods, PyReadonlyArray1, PyReadonlyArray2};
use once_cell::sync::Lazy;
use parking_lot::{Mutex, RwLock};
use pyo3::{
    prelude::PyAnyMethods,
    types::{PyDict, PyModule},
    IntoPy, Py, PyAny, PyResult, Python,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::task;
use tracing::{info, instrument};
use uuid::Uuid;

static PYTHONPATH_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
static PYTHONPATH_ENTRIES: Lazy<Mutex<HashSet<OsString>>> =
    Lazy::new(|| Mutex::new(HashSet::new()));
const TARGET_SAMPLE_RATE: u32 = 24_000;

#[derive(Debug, Error)]
pub enum TtsEngineError {
    #[error("voice profile '{0}' not found")]
    VoiceNotFound(String),
    #[error(transparent)]
    Python(#[from] pyo3::PyErr),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VoiceProfileConfig {
    pub id: String,
    pub reference_audio: PathBuf,
    pub reference_text: String,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub engine_label: Option<String>,
    #[serde(default)]
    pub preload: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct F5EngineConfig {
    pub model: String,
    #[serde(default)]
    pub ckpt_file: Option<PathBuf>,
    #[serde(default)]
    pub vocab_file: Option<PathBuf>,
    #[serde(default)]
    pub ode_method: Option<String>,
    #[serde(default)]
    pub use_ema: Option<bool>,
    #[serde(default)]
    pub vocoder_local_path: Option<PathBuf>,
    #[serde(default)]
    pub device: Option<String>,
    #[serde(default)]
    pub hf_cache_dir: Option<PathBuf>,
    #[serde(default)]
    pub default_nfe_step: Option<u32>,
    pub python_package_path: PathBuf,
    pub voices: Vec<VoiceProfileConfig>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IndexTtsEngineConfig {
    pub python_package_path: PathBuf,
    pub config_file: PathBuf,
    pub model_dir: PathBuf,
    #[serde(default)]
    pub device: Option<String>,
    #[serde(default)]
    pub use_fp16: Option<bool>,
    #[serde(default)]
    pub use_cuda_kernel: Option<bool>,
    #[serde(default)]
    pub use_deepspeed: Option<bool>,
    #[serde(default)]
    pub voices: Vec<IndexTtsVoiceConfig>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IndexTtsVoiceConfig {
    pub id: String,
    pub reference_audio: PathBuf,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub reference_text: Option<String>,
    #[serde(default)]
    pub emo_audio: Option<PathBuf>,
    #[serde(default)]
    pub emo_text: Option<String>,
    #[serde(default)]
    pub emo_alpha: Option<f32>,
    #[serde(default)]
    pub engine_label: Option<String>,
    #[serde(default)]
    pub preload: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TtsRequest {
    pub text: String,
    pub voice_id: String,
    #[serde(default)]
    pub speed: Option<f32>,
    #[serde(default)]
    pub target_rms: Option<f32>,
    #[serde(default)]
    pub cross_fade_duration: Option<f32>,
    #[serde(default)]
    pub sway_sampling_coef: Option<f32>,
    #[serde(default)]
    pub cfg_strength: Option<f32>,
    #[serde(default)]
    pub nfe_step: Option<u32>,
    #[serde(default)]
    pub fix_duration: Option<f32>,
    #[serde(default)]
    pub remove_silence: Option<bool>,
    #[serde(default)]
    pub seed: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct VoiceOverrideUpdate {
    pub reference_audio: Option<PathBuf>,
    pub reference_text: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TtsResponse {
    pub request_id: Uuid,
    pub sample_rate: u32,
    pub audio_base64: String,
    pub waveform_len: usize,
    pub voice_id: String,
    pub engine: EngineKind,
    pub engine_label: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineKind {
    F5,
    IndexTts,
}

impl EngineKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            EngineKind::F5 => "f5",
            EngineKind::IndexTts => "index_tts",
        }
    }
}

impl std::fmt::Display for EngineKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for EngineKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "f5" => Ok(EngineKind::F5),
            "index_tts" | "index-tts" | "indextts" => Ok(EngineKind::IndexTts),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VoiceDescriptor {
    pub id: String,
    pub engine: EngineKind,
    pub engine_label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_text: Option<String>,
}

#[async_trait]
pub trait TtsEngine: Send + Sync {
    fn kind(&self) -> EngineKind;
    fn voice_descriptors(&self) -> Vec<VoiceDescriptor>;
    async fn synthesize(&self, request: TtsRequest) -> Result<TtsResponse>;
    fn apply_override(&self, voice_id: &str, update: VoiceOverrideUpdate) -> Result<()>;
    fn resolve_reference(&self, voice_id: &str) -> Option<(PathBuf, Option<String>)>;
}

fn ensure_python_path(path: &Path) {
    let canonical = path.to_path_buf();
    let os_path = canonical.as_os_str().to_os_string();

    let _guard = PYTHONPATH_LOCK.lock();
    let mut entries = PYTHONPATH_ENTRIES.lock();
    if entries.contains(&os_path) {
        return;
    }

    let mut current_paths: Vec<PathBuf> = std::env::var_os("PYTHONPATH")
        .map(|existing| std::env::split_paths(&existing).collect())
        .unwrap_or_default();

    if !current_paths.iter().any(|p| p == &canonical) {
        current_paths.push(canonical.clone());
    }

    let joined =
        std::env::join_paths(current_paths.into_iter()).expect("failed to construct PYTHONPATH");
    std::env::set_var("PYTHONPATH", joined);
    entries.insert(os_path);
}

#[derive(Clone)]
pub struct F5Engine {
    inner: Arc<EngineInner>,
}

struct EngineInner {
    runtime: Mutex<PythonRuntime>,
    voices: RwLock<HashMap<String, VoiceProfileConfig>>,
    default_nfe_step: Option<u32>,
}

struct PythonRuntime {
    engine: Py<PyAny>,
}

#[derive(Clone)]
pub struct IndexTtsEngine {
    inner: Arc<IndexEngineInner>,
}

struct IndexEngineInner {
    runtime: Mutex<IndexRuntime>,
    voices: RwLock<HashMap<String, IndexVoice>>,
}

struct IndexRuntime {
    engine: Py<PyAny>,
}

#[derive(Clone)]
struct IndexVoice {
    id: String,
    reference_audio: PathBuf,
    language: Option<String>,
    reference_text: Option<String>,
    emo_audio: Option<PathBuf>,
    emo_text: Option<String>,
    emo_alpha: Option<f32>,
    engine_label: Option<String>,
}

impl F5Engine {
    pub fn new(config: F5EngineConfig) -> Result<Self> {
        let python_package_path = config
            .python_package_path
            .canonicalize()
            .context("failed to canonicalize python package path")?;

        ensure_python_path(&python_package_path);

        let mut voices = HashMap::new();
        for profile in &config.voices {
            let mut canonical = profile.clone();
            canonical.reference_audio =
                canonical.reference_audio.canonicalize().with_context(|| {
                    format!(
                        "failed to canonicalize reference audio for voice {}",
                        profile.id
                    )
                })?;
            voices.insert(canonical.id.clone(), canonical);
        }

        let runtime = Python::with_gil(|py| -> Result<PythonRuntime> {
            let f5_module = PyModule::import(py, "f5_tts.api")?;
            let cls = f5_module.getattr("F5TTS")?;
            let kwargs = Self::build_kwargs(py, &config)?;
            let engine = cls.call((), Some(kwargs))?.into_py(py);
            Ok(PythonRuntime { engine })
        })?;

        info!(target = "ishowtts::tts_engine", model = %config.model, voice_count = voices.len(), "initialized F5-TTS runtime");

        Ok(Self {
            inner: Arc::new(EngineInner {
                runtime: Mutex::new(runtime),
                voices: RwLock::new(voices),
                default_nfe_step: config.default_nfe_step,
            }),
        })
    }

    fn build_kwargs<'py>(py: Python<'py>, config: &F5EngineConfig) -> PyResult<&'py PyDict> {
        let kwargs = PyDict::new(py);
        kwargs.set_item("model", config.model.as_str())?;
        if let Some(ref ckpt) = config.ckpt_file {
            kwargs.set_item("ckpt_file", ckpt.as_os_str())?;
        }
        if let Some(ref vocab) = config.vocab_file {
            kwargs.set_item("vocab_file", vocab.as_os_str())?;
        }
        if let Some(ref ode) = config.ode_method {
            kwargs.set_item("ode_method", ode.as_str())?;
        }
        if let Some(use_ema) = config.use_ema {
            kwargs.set_item("use_ema", use_ema)?;
        }
        if let Some(ref vocoder) = config.vocoder_local_path {
            kwargs.set_item("vocoder_local_path", vocoder.as_os_str())?;
        }
        if let Some(ref device) = config.device {
            kwargs.set_item("device", device.as_str())?;
        }
        if let Some(ref cache) = config.hf_cache_dir {
            kwargs.set_item("hf_cache_dir", cache.as_os_str())?;
        }
        Ok(kwargs)
    }

    pub fn available_voices(&self) -> Vec<String> {
        self.inner.voices.read().keys().cloned().collect()
    }

    pub fn voice_profiles(&self) -> Vec<VoiceProfileConfig> {
        self.inner.voices.read().values().cloned().collect()
    }

    #[instrument(skip(self))]
    pub async fn synthesize(&self, request: TtsRequest) -> Result<TtsResponse> {
        let inner = self.inner.clone();
        task::spawn_blocking(move || inner.synthesize_blocking(request)).await?
    }
}

impl IndexTtsEngine {
    pub fn new(config: IndexTtsEngineConfig) -> Result<Self> {
        if config.voices.is_empty() {
            anyhow::bail!("IndexTTS configuration must declare at least one voice profile");
        }

        let python_package_path = config
            .python_package_path
            .canonicalize()
            .context("failed to canonicalize IndexTTS python package path")?;
        ensure_python_path(&python_package_path);

        let config_file = config
            .config_file
            .canonicalize()
            .context("failed to canonicalize IndexTTS config file path")?;
        let model_dir = config
            .model_dir
            .canonicalize()
            .context("failed to canonicalize IndexTTS model directory")?;

        let mut voices = HashMap::new();
        for voice in config.voices {
            let reference_audio = voice.reference_audio.canonicalize().with_context(|| {
                format!(
                    "failed to canonicalize reference audio for IndexTTS voice {}",
                    voice.id
                )
            })?;

            let emo_audio = match voice.emo_audio {
                Some(path) => Some(path.canonicalize().with_context(|| {
                    format!(
                        "failed to canonicalize emotion audio for IndexTTS voice {}",
                        voice.id
                    )
                })?),
                None => None,
            };

            let entry = IndexVoice {
                id: voice.id.clone(),
                reference_audio,
                language: voice.language.clone(),
                reference_text: voice.reference_text.clone(),
                emo_audio,
                emo_text: voice.emo_text.clone(),
                emo_alpha: voice.emo_alpha,
                engine_label: voice.engine_label.clone(),
            };

            if voices.insert(entry.id.clone(), entry).is_some() {
                anyhow::bail!(
                    "duplicate IndexTTS voice id '{}' detected in configuration",
                    voice.id
                );
            }
        }

        let model_dir_for_log = model_dir.clone();
        let runtime = Python::with_gil(|py| -> Result<IndexRuntime> {
            let module = PyModule::import(py, "indextts.infer_v2")?;
            let cls = module.getattr("IndexTTS2")?;
            let kwargs = PyDict::new(py);
            kwargs.set_item("cfg_path", config_file.as_os_str())?;
            kwargs.set_item("model_dir", model_dir.as_os_str())?;
            if let Some(ref device) = config.device {
                kwargs.set_item("device", device.as_str())?;
            }
            if let Some(use_fp16) = config.use_fp16 {
                kwargs.set_item("use_fp16", use_fp16)?;
            }
            if let Some(use_cuda_kernel) = config.use_cuda_kernel {
                kwargs.set_item("use_cuda_kernel", use_cuda_kernel)?;
            }
            if let Some(use_deepspeed) = config.use_deepspeed {
                kwargs.set_item("use_deepspeed", use_deepspeed)?;
            }
            let engine = cls.call((), Some(kwargs))?.into_py(py);
            Ok(IndexRuntime { engine })
        })?;

        info!(
            target = "ishowtts::tts_engine",
            model_dir = %model_dir_for_log.display(),
            voice_count = voices.len(),
            "initialized IndexTTS runtime"
        );

        Ok(Self {
            inner: Arc::new(IndexEngineInner {
                runtime: Mutex::new(runtime),
                voices: RwLock::new(voices),
            }),
        })
    }
}

#[async_trait]
impl TtsEngine for F5Engine {
    fn kind(&self) -> EngineKind {
        EngineKind::F5
    }

    fn voice_descriptors(&self) -> Vec<VoiceDescriptor> {
        self.voice_profiles()
            .into_iter()
            .map(|profile| VoiceDescriptor {
                id: profile.id,
                engine: EngineKind::F5,
                engine_label: profile
                    .engine_label
                    .clone()
                    .unwrap_or_else(|| EngineKind::F5.as_str().to_string()),
                language: profile.language,
                reference_text: Some(profile.reference_text),
            })
            .collect()
    }

    async fn synthesize(&self, request: TtsRequest) -> Result<TtsResponse> {
        F5Engine::synthesize(self, request).await
    }

    fn apply_override(&self, voice_id: &str, update: VoiceOverrideUpdate) -> Result<()> {
        let mut voices = self.inner.voices.write();
        let entry = voices
            .get_mut(voice_id)
            .ok_or_else(|| TtsEngineError::VoiceNotFound(voice_id.to_string()))?;

        if let Some(audio) = update.reference_audio {
            let canonical = audio.canonicalize().with_context(|| {
                format!("failed to canonicalize override audio for voice {voice_id}")
            })?;
            entry.reference_audio = canonical;
        }

        if let Some(text) = update.reference_text {
            entry.reference_text = text;
        }

        Ok(())
    }

    fn resolve_reference(&self, voice_id: &str) -> Option<(PathBuf, Option<String>)> {
        self.inner.voices.read().get(voice_id).map(|profile| {
            (
                profile.reference_audio.clone(),
                Some(profile.reference_text.clone()),
            )
        })
    }
}

#[async_trait]
impl TtsEngine for IndexTtsEngine {
    fn kind(&self) -> EngineKind {
        EngineKind::IndexTts
    }

    fn voice_descriptors(&self) -> Vec<VoiceDescriptor> {
        self.inner
            .voices
            .read()
            .values()
            .map(|voice| VoiceDescriptor {
                id: voice.id.clone(),
                engine: EngineKind::IndexTts,
                engine_label: voice
                    .engine_label
                    .clone()
                    .unwrap_or_else(|| EngineKind::IndexTts.as_str().to_string()),
                language: voice.language.clone(),
                reference_text: voice.reference_text.clone(),
            })
            .collect()
    }

    async fn synthesize(&self, request: TtsRequest) -> Result<TtsResponse> {
        let inner = self.inner.clone();
        task::spawn_blocking(move || inner.synthesize_blocking(request)).await?
    }

    fn apply_override(&self, voice_id: &str, update: VoiceOverrideUpdate) -> Result<()> {
        let mut voices = self.inner.voices.write();
        let entry = voices
            .get_mut(voice_id)
            .ok_or_else(|| anyhow!("IndexTTS voice '{}' not found", voice_id))?;

        if let Some(audio) = update.reference_audio {
            let canonical = audio.canonicalize().with_context(|| {
                format!("failed to canonicalize override audio for voice {voice_id}")
            })?;
            entry.reference_audio = canonical;
        }

        if let Some(text) = update.reference_text {
            entry.reference_text = Some(text);
        }

        Ok(())
    }

    fn resolve_reference(&self, voice_id: &str) -> Option<(PathBuf, Option<String>)> {
        self.inner
            .voices
            .read()
            .get(voice_id)
            .map(|voice| (voice.reference_audio.clone(), voice.reference_text.clone()))
    }
}

impl EngineInner {
    fn synthesize_blocking(&self, request: TtsRequest) -> Result<TtsResponse> {
        let voice = {
            let voices = self.voices.read();
            voices
                .get(&request.voice_id)
                .cloned()
                .ok_or_else(|| TtsEngineError::VoiceNotFound(request.voice_id.clone()))?
        };

        let target_rms = request.target_rms.unwrap_or(0.1);
        let cross_fade_duration = request.cross_fade_duration.unwrap_or(0.15);
        let sway = request.sway_sampling_coef.unwrap_or(-1.0);
        let cfg_strength = request.cfg_strength.unwrap_or(2.0);
        // Use configured default NFE step (default 16 for speed) or request override
        let nfe_step = request.nfe_step.unwrap_or_else(|| self.default_nfe_step.unwrap_or(16));
        let speed = request.speed.unwrap_or(1.0);
        let fix_duration = request.fix_duration;
        let remove_silence = request.remove_silence.unwrap_or(false);
        let seed = request.seed;

        let mut runtime = self.runtime.lock();
        let (samples, sample_rate) = runtime.run_infer(
            &voice,
            &request.text,
            target_rms,
            cross_fade_duration,
            sway,
            cfg_strength,
            nfe_step,
            speed,
            fix_duration,
            remove_silence,
            seed,
        )?;

        let mut sample_rate = sample_rate;
        let mut samples = samples;
        if sample_rate != TARGET_SAMPLE_RATE {
            samples = resample_linear(&samples, sample_rate, TARGET_SAMPLE_RATE);
            sample_rate = TARGET_SAMPLE_RATE;
        }

        let wav_bytes = encode_wav(&samples, sample_rate)?;
        let encoded = BASE64.encode(&wav_bytes);
        let response = TtsResponse {
            request_id: Uuid::new_v4(),
            sample_rate,
            audio_base64: encoded,
            waveform_len: samples.len(),
            voice_id: voice.id.clone(),
            engine: EngineKind::F5,
            engine_label: voice
                .engine_label
                .clone()
                .unwrap_or_else(|| EngineKind::F5.as_str().to_string()),
        };
        Ok(response)
    }
}

impl PythonRuntime {
    fn run_infer(
        &mut self,
        voice: &VoiceProfileConfig,
        text: &str,
        target_rms: f32,
        cross_fade_duration: f32,
        sway_sampling_coef: f32,
        cfg_strength: f32,
        nfe_step: u32,
        speed: f32,
        fix_duration: Option<f32>,
        remove_silence: bool,
        seed: Option<u64>,
    ) -> Result<(Vec<f32>, u32)> {
        Python::with_gil(|py| -> Result<(Vec<f32>, u32)> {
            let engine = self.engine.as_ref(py);
            let infer = engine.getattr("infer")?;
            let kwargs = PyDict::new(py);
            kwargs.set_item("target_rms", target_rms)?;
            kwargs.set_item("cross_fade_duration", cross_fade_duration)?;
            kwargs.set_item("sway_sampling_coef", sway_sampling_coef)?;
            kwargs.set_item("cfg_strength", cfg_strength)?;
            kwargs.set_item("nfe_step", nfe_step)?;
            kwargs.set_item("speed", speed)?;
            if let Some(duration) = fix_duration {
                kwargs.set_item("fix_duration", duration)?;
            }
            kwargs.set_item("remove_silence", remove_silence)?;
            if let Some(seed) = seed {
                kwargs.set_item("seed", seed)?;
            }

            let result = infer.call(
                (
                    voice.reference_audio.as_os_str(),
                    voice.reference_text.as_str(),
                    text,
                ),
                Some(kwargs),
            )?;

            let tuple: (Py<PyAny>, u32, Py<PyAny>) = result.extract()?;
            let wav_array: Py<PyAny> = tuple.0;
            let sr = tuple.1;

            let bound = wav_array.bind(py);
            // Handle either float32 or float64 output from the Python runtime.
            if let Ok(array_f32) = bound.downcast::<PyArray1<f32>>() {
                let readonly: PyReadonlyArray1<f32> = array_f32.readonly();
                let waveform = readonly.as_slice()?.to_vec();
                return Ok((waveform, sr));
            }

            if let Ok(array_f64) = bound.downcast::<PyArray1<f64>>() {
                let readonly: PyReadonlyArray1<f64> = array_f64.readonly();
                let waveform = readonly
                    .as_slice()?
                    .iter()
                    .map(|&sample| sample as f32)
                    .collect();
                return Ok((waveform, sr));
            }

            Err(anyhow!(
                "unsupported waveform dtype: expected float32 or float64"
            ))
        })
    }
}

impl IndexEngineInner {
    fn synthesize_blocking(&self, request: TtsRequest) -> Result<TtsResponse> {
        let voice = {
            let voices = self.voices.read();
            voices
                .get(&request.voice_id)
                .cloned()
                .ok_or_else(|| anyhow!("IndexTTS voice '{}' not found", request.voice_id))?
        };

        let mut runtime = self.runtime.lock();
        let (mut samples, mut sample_rate) = runtime.run_infer(&voice, &request.text)?;

        if sample_rate != TARGET_SAMPLE_RATE {
            samples = resample_linear(&samples, sample_rate, TARGET_SAMPLE_RATE);
            sample_rate = TARGET_SAMPLE_RATE;
        }

        if request.remove_silence.unwrap_or(false) {
            samples = trim_trailing_silence(&samples, 1e-3);
        }

        let wav_bytes = encode_wav(&samples, sample_rate)?;
        let encoded = BASE64.encode(&wav_bytes);

        Ok(TtsResponse {
            request_id: Uuid::new_v4(),
            sample_rate,
            audio_base64: encoded,
            waveform_len: samples.len(),
            voice_id: voice.id.clone(),
            engine: EngineKind::IndexTts,
            engine_label: voice
                .engine_label
                .clone()
                .unwrap_or_else(|| EngineKind::IndexTts.as_str().to_string()),
        })
    }
}

impl IndexRuntime {
    fn run_infer(&mut self, voice: &IndexVoice, text: &str) -> Result<(Vec<f32>, u32)> {
        Python::with_gil(|py| -> Result<(Vec<f32>, u32)> {
            let engine = self.engine.as_ref(py);
            let infer = engine.getattr("infer")?;

            let kwargs = PyDict::new(py);
            if let Some(ref emo_audio) = voice.emo_audio {
                kwargs.set_item("emo_audio_prompt", emo_audio.as_os_str())?;
            }
            if let Some(alpha) = voice.emo_alpha {
                kwargs.set_item("emo_alpha", alpha)?;
            }
            if let Some(ref emo_text) = voice.emo_text {
                kwargs.set_item("emo_text", emo_text)?;
                kwargs.set_item("use_emo_text", true)?;
            }
            kwargs.set_item("verbose", false)?;

            let args = (voice.reference_audio.as_os_str(), text, "");

            let result = infer.call(args, Some(kwargs))?;
            let tuple = result
                .downcast::<pyo3::types::PyTuple>()
                .map_err(|err| anyhow!(err.to_string()))?;
            let sr: u32 = tuple.get_item(0)?.extract()?;
            let bound = tuple.get_item(1)?;

            if let Ok(array) = bound.downcast::<PyArray2<i16>>() {
                let readonly: PyReadonlyArray2<i16> = array.readonly();
                let view = readonly.as_array();
                let mut waveform = Vec::with_capacity(view.len());
                for &sample in view.iter() {
                    waveform.push(sample as f32 / i16::MAX as f32);
                }
                return Ok((waveform, sr));
            }

            if let Ok(array) = bound.downcast::<PyArray1<i16>>() {
                let readonly: PyReadonlyArray1<i16> = array.readonly();
                let slice = readonly.as_slice()?;
                let mut waveform = Vec::with_capacity(slice.len());
                for &sample in slice {
                    waveform.push(sample as f32 / i16::MAX as f32);
                }
                return Ok((waveform, sr));
            }

            if let Ok(array) = bound.downcast::<PyArray1<f32>>() {
                let readonly: PyReadonlyArray1<f32> = array.readonly();
                let waveform = readonly.as_slice()?.to_vec();
                return Ok((waveform, sr));
            }

            if let Ok(array) = bound.downcast::<PyArray2<f32>>() {
                let readonly: PyReadonlyArray2<f32> = array.readonly();
                let view = readonly.as_array();
                let mut waveform = Vec::with_capacity(view.len());
                for &sample in view.iter() {
                    waveform.push(sample);
                }
                return Ok((waveform, sr));
            }

            if let Ok(array) = bound.downcast::<PyArray1<f64>>() {
                let readonly: PyReadonlyArray1<f64> = array.readonly();
                let waveform = readonly
                    .as_slice()?
                    .iter()
                    .map(|&sample| sample as f32)
                    .collect();
                return Ok((waveform, sr));
            }

            Err(anyhow!(
                "unsupported waveform dtype returned by IndexTTS runtime"
            ))
        })
    }
}

fn encode_wav(samples: &[f32], sample_rate: u32) -> Result<Vec<u8>> {
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    // Pre-allocate buffer: WAV header (44 bytes) + samples (2 bytes each)
    let mut buffer = Vec::with_capacity(44 + samples.len() * 2);

    {
        let mut cursor = std::io::Cursor::new(&mut buffer);
        let mut writer = WavWriter::new(&mut cursor, spec)?;

        // Optimized: batch convert and write samples
        for &sample in samples {
            let clamped = sample.clamp(-1.0, 1.0);
            let value = (clamped * i16::MAX as f32) as i16;
            writer.write_sample(value)?;
        }
        writer.finalize()?;
    }

    Ok(buffer)
}

fn resample_linear(input: &[f32], src_rate: u32, dst_rate: u32) -> Vec<f32> {
    if src_rate == dst_rate || input.is_empty() {
        return input.to_vec();
    }

    let ratio = dst_rate as f64 / src_rate as f64;
    let output_len = (input.len() as f64 * ratio).ceil() as usize;
    let mut output = Vec::with_capacity(output_len);

    // Optimized: precompute inverse ratio and use f32 for faster operations
    let inv_ratio = (src_rate as f32) / (dst_rate as f32);
    let input_len_minus_1 = (input.len() - 1) as f32;

    for i in 0..output_len {
        let src_pos = (i as f32) * inv_ratio;
        let idx = src_pos as usize;

        if idx + 1 >= input.len() {
            output.push(*input.last().unwrap_or(&0.0));
        } else {
            let frac = src_pos - idx as f32;
            let a = unsafe { *input.get_unchecked(idx) };
            let b = unsafe { *input.get_unchecked(idx + 1) };
            // Linear interpolation: a + (b - a) * frac
            output.push(a + (b - a) * frac);
        }
    }

    output
}

fn trim_trailing_silence(samples: &[f32], threshold: f32) -> Vec<f32> {
    if samples.is_empty() {
        return Vec::new();
    }

    let thresh = threshold.abs();
    let mut end = samples.len();
    while end > 0 && samples[end - 1].abs() <= thresh {
        end -= 1;
    }

    if end == 0 {
        return vec![0.0];
    }

    samples[..end].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_wav() {
        let sample_rate = 16000;
        let samples = vec![0.0_f32, 0.5, -0.5, 1.0, -1.0];
        let encoded = encode_wav(&samples, sample_rate).unwrap();
        assert!(!encoded.is_empty());
        // RIFF header check
        assert_eq!(&encoded[0..4], b"RIFF");
        assert_eq!(&encoded[8..12], b"WAVE");
    }
}
