use std::{cmp::max, str::FromStr, sync::Arc, time::Instant};

use anyhow::{Context, Result};
use axum::body::Body;
use axum::http::{HeaderValue, Method, StatusCode};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Multipart, Path, Query, State,
    },
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
use chrono::{DateTime, Utc};
use futures::{stream::SplitSink, SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::{fs, sync::broadcast::error::RecvError};
use tower_http::cors::{Any, CorsLayer};
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use crate::{
    danmaku::{
        DanmakuService, PlaybackItem, StartRequest, StartResponse, StopRequest, StopResponse,
    },
    synth::Synthesizer,
    voice_overrides::{OverrideAudio, VoiceOverrideStore},
};
use danmaku::message::{MessageContent, NormalizedMessage, Platform};
use shimmy::AppState as ShimmyAppState;
use tts_engine::{EngineKind, TtsRequest, TtsResponse, VoiceOverrideUpdate};

const MAX_WORDS_PER_REQUEST: usize = 77;

fn preview_text(value: &str) -> String {
    const LIMIT: usize = 120;
    let trimmed = value.trim();
    let mut preview = String::new();
    let mut chars = trimmed.chars();
    for ch in chars.by_ref().take(LIMIT) {
        preview.push(ch);
    }
    if chars.next().is_some() {
        preview.push('…');
    }
    preview
}

#[derive(Clone)]
pub struct ApiState {
    pub synthesizer: Arc<Synthesizer>,
    pub default_voice: String,
    pub danmaku: Option<Arc<DanmakuService>>,
    pub voice_overrides: Arc<VoiceOverrideStore>,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    voices: usize,
    default_voice: String,
}

#[derive(Debug, Deserialize)]
pub struct SynthesizePayload {
    pub text: String,
    #[serde(default)]
    pub voice_id: Option<String>,
    #[serde(default)]
    pub engine: Option<String>,
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

#[derive(Debug, Serialize)]
pub struct SynthesizeResponse {
    pub request_id: Uuid,
    pub voice_id: String,
    pub engine: String,
    pub engine_label: String,
    pub sample_rate: u32,
    pub audio_base64: String,
    pub waveform_len: usize,
    pub format: &'static str,
}

#[instrument(skip(state))]
pub async fn health(State(state): State<ApiState>) -> impl IntoResponse {
    let voices_count = state.synthesizer.voices().len();
    let response = HealthResponse {
        status: "ok",
        voices: voices_count,
        default_voice: state.default_voice.clone(),
    };
    Json(response)
}

#[instrument(skip(state))]
pub async fn list_voices(State(state): State<ApiState>) -> impl IntoResponse {
    Json(state.synthesizer.voices())
}

#[instrument(skip(state, payload))]
pub async fn synthesize(
    State(state): State<ApiState>,
    Json(payload): Json<SynthesizePayload>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let started_at = Instant::now();
    let voice_id = payload
        .voice_id
        .clone()
        .unwrap_or_else(|| state.default_voice.clone());

    let voice_meta = state.synthesizer.voice_descriptor(&voice_id).ok_or((
        StatusCode::BAD_REQUEST,
        format!("unknown voice_id '{voice_id}'"),
    ))?;

    if let Some(ref engine_name) = payload.engine {
        if engine_name != voice_meta.engine.as_str() {
            return Err((
                StatusCode::BAD_REQUEST,
                format!(
                    "voice '{voice_id}' belongs to engine '{}', not '{engine_name}'",
                    voice_meta.engine.as_str()
                ),
            ));
        }
    }

    let (truncated_text, _) = truncate_text(&payload.text, MAX_WORDS_PER_REQUEST);
    if truncated_text.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "text must not be empty".into()));
    }

    let text_for_request = truncated_text.clone();
    let text_preview_debug = preview_text(&text_for_request);
    let request = build_request(truncated_text, &payload, &voice_id);
    debug!(
        target = "ishowtts::api::tts",
        voice_id = %voice_id,
        requested_engine = payload.engine.as_deref(),
        text_len = text_for_request.len(),
        original_len = payload.text.len(),
        truncated = payload.text.len() != text_for_request.len(),
        text_preview = %text_preview_debug,
        "tts request accepted"
    );
    let response = state
        .synthesizer
        .synthesize(request)
        .await
        .map(map_response)
        .map_err(|err| (StatusCode::BAD_GATEWAY, err.to_string()))?;

    let elapsed_ms = started_at.elapsed().as_millis();
    let (audio_bytes, audio_kb) = match BASE64_STANDARD.decode(response.audio_base64.as_bytes()) {
        Ok(buf) => {
            let len = buf.len();
            let kb = ((len as f64) / 1024.0 * 10.0).round() / 10.0;
            (len, kb)
        }
        Err(err) => {
            warn!(
                target = "ishowtts::api::tts",
                ?err,
                "failed to decode synthesized audio payload for metrics"
            );
            (0, 0.0)
        }
    };

    let text_preview_info = preview_text(&text_for_request);
    info!(
        target = "ishowtts::api::tts",
        voice_id = %response.voice_id,
        engine = %response.engine,
        engine_label = %response.engine_label,
        sample_rate = response.sample_rate,
        waveform_len = response.waveform_len,
        elapsed_ms,
        audio_bytes,
        audio_kb,
        text_len = text_for_request.len(),
        text_preview = %text_preview_info,
        "tts synthesis complete"
    );

    Ok(Json(response))
}

fn map_response(resp: TtsResponse) -> SynthesizeResponse {
    SynthesizeResponse {
        request_id: resp.request_id,
        voice_id: resp.voice_id,
        engine: resp.engine.as_str().to_string(),
        engine_label: resp.engine_label,
        sample_rate: resp.sample_rate,
        audio_base64: resp.audio_base64,
        waveform_len: resp.waveform_len,
        format: "audio/wav",
    }
}

fn build_request(text: String, payload: &SynthesizePayload, voice_id: &str) -> TtsRequest {
    TtsRequest {
        text,
        voice_id: voice_id.to_string(),
        speed: payload.speed,
        target_rms: payload.target_rms,
        cross_fade_duration: payload.cross_fade_duration,
        sway_sampling_coef: payload.sway_sampling_coef,
        cfg_strength: payload.cfg_strength,
        nfe_step: payload.nfe_step,
        fix_duration: payload.fix_duration,
        remove_silence: payload.remove_silence,
        seed: payload.seed,
    }
}

fn truncate_text(text: &str, max_words: usize) -> (String, bool) {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return (String::new(), false);
    }

    if words.len() <= max_words {
        return (text.trim().to_string(), false);
    }

    let truncated = words
        .into_iter()
        .take(max(max_words, 1))
        .collect::<Vec<_>>()
        .join(" ");
    (truncated, true)
}

pub fn build_api_router(state: ApiState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any);

    let api_routes = Router::new()
        .route("/health", get(health))
        .route("/voices", get(list_voices))
        .route(
            "/voices/:voice_id/reference",
            get(get_voice_reference)
                .post(set_voice_reference)
                .delete(delete_voice_reference),
        )
        .route(
            "/voices/:voice_id/reference/audio",
            get(get_voice_reference_audio),
        )
        .route("/tts", post(synthesize))
        .route("/danmaku/start", post(start_danmaku))
        .route("/danmaku/stop", post(stop_danmaku))
        .route("/danmaku/enqueue", post(enqueue_danmaku))
        .route("/danmaku/next", get(next_danmaku))
        .with_state(state.clone())
        .layer(cors);

    Router::new()
        .merge(api_routes)
        .route("/danmaku/stream", get(stream_danmaku_ws))
        .with_state(state)
}

pub fn build_shimmy_router(state: Arc<ShimmyAppState>) -> Router {
    Router::new()
        .route("/generate", post(shimmy::api::generate))
        .route("/models", get(shimmy::api::list_models))
        .route("/models/discover", post(shimmy::api::discover_models))
        .route("/models/:name/load", post(shimmy::api::load_model))
        .route("/models/:name/unload", post(shimmy::api::unload_model))
        .route("/models/:name/status", get(shimmy::api::model_status))
        .route("/ws/generate", get(shimmy::api::ws_generate))
        .with_state(state)
}

pub fn build_openai_router(state: Arc<ShimmyAppState>) -> Router {
    Router::new()
        .route(
            "/chat/completions",
            post(shimmy::openai_compat::chat_completions),
        )
        .route("/models", get(shimmy::openai_compat::models))
        .with_state(state)
}

#[derive(Debug, Serialize)]
struct VoiceReferenceResponse {
    voice_id: String,
    engine: String,
    engine_label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    active_reference_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    baseline_reference_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    override_reference_text: Option<String>,
    baseline_audio_available: bool,
    override_audio_available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    override_updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct VoiceReferenceAudioQuery {
    source: String,
}

#[instrument(skip(state))]
async fn get_voice_reference(
    State(state): State<ApiState>,
    Path(voice_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let payload = build_voice_reference_response(&state, &voice_id)?;
    let text_override = payload
        .override_reference_text
        .as_ref()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    let active_text_len = payload
        .active_reference_text
        .as_ref()
        .map(|text| text.len());
    let active_text_preview = payload
        .active_reference_text
        .as_deref()
        .map(|text| preview_text(text));
    let override_text_len = payload
        .override_reference_text
        .as_ref()
        .map(|text| text.len());
    let override_text_preview = payload
        .override_reference_text
        .as_deref()
        .map(|text| preview_text(text));
    debug!(
        target = "ishowtts::api::voices",
        voice = %payload.voice_id,
        engine = %payload.engine,
        audio_override = payload.override_audio_available,
        text_override,
        active_text_len,
        active_text_preview = active_text_preview.as_deref(),
        override_text_len,
        override_text_preview = override_text_preview.as_deref(),
        "voice reference fetched"
    );
    Ok(Json(payload))
}

#[instrument(skip(state, multipart))]
async fn set_voice_reference(
    State(state): State<ApiState>,
    Path(voice_id): Path<String>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let descriptor = state
        .synthesizer
        .voice_descriptor(&voice_id)
        .or_else(|| {
            state
                .synthesizer
                .voices()
                .into_iter()
                .find(|voice| voice.id == voice_id)
        })
        .ok_or((StatusCode::NOT_FOUND, format!("未知音色 '{voice_id}'")))?;
    let engine = descriptor.engine;

    let mut text_override: Option<String> = None;
    let mut text_supplied = false;
    let mut temp_audio: Option<OverrideAudio> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|err| (StatusCode::BAD_REQUEST, format!("解析上传内容失败: {err}")))?
    {
        let name = field.name().map(|s| s.to_string());
        match name.as_deref() {
            Some("text") => {
                text_supplied = true;
                let value = field
                    .text()
                    .await
                    .map_err(|err| (StatusCode::BAD_REQUEST, format!("读取文本失败: {err}")))?;
                text_override = Some(value.trim().to_string());
            }
            Some("audio") => {
                let filename_ext = field
                    .file_name()
                    .and_then(|name| {
                        std::path::Path::new(name)
                            .extension()
                            .and_then(|ext| ext.to_str())
                    })
                    .map(|ext| ext.to_ascii_lowercase());
                let mime_ext =
                    field
                        .content_type()
                        .map(|mime| mime.to_string())
                        .and_then(|value| match value.as_str() {
                            "audio/wav" | "audio/x-wav" => Some("wav".to_string()),
                            "audio/mpeg" | "audio/mp3" => Some("mp3".to_string()),
                            "audio/aac" => Some("m4a".to_string()),
                            "audio/ogg" => Some("ogg".to_string()),
                            "audio/opus" => Some("opus".to_string()),
                            _ => None,
                        });

                let data = field
                    .bytes()
                    .await
                    .map_err(|err| (StatusCode::BAD_REQUEST, format!("读取音频失败: {err}")))?;
                if data.is_empty() {
                    continue;
                }
                if data.len() > 10 * 1024 * 1024 {
                    return Err((StatusCode::BAD_REQUEST, "音频文件超过 10MB 限制".into()));
                }

                temp_audio = Some(OverrideAudio {
                    bytes: data.to_vec(),
                    extension: filename_ext.or(mime_ext),
                });
            }
            _ => {}
        }
    }

    if temp_audio.is_none() && !text_supplied {
        return Err((
            StatusCode::BAD_REQUEST,
            "请上传参考音频或提供参考文本".into(),
        ));
    }

    let incoming_text_len = text_override.as_ref().map(|text| text.len());
    let incoming_text_preview = text_override.as_ref().map(|text| preview_text(text));
    let audio_ext_hint_dbg = temp_audio
        .as_ref()
        .and_then(|audio| audio.extension.clone());
    debug!(
        target = "ishowtts::api::voices",
        voice = %voice_id,
        engine = %engine,
        has_text = text_supplied,
        has_audio = temp_audio.is_some(),
        incoming_text_len,
        incoming_text_preview = incoming_text_preview.as_deref(),
        incoming_audio_ext = audio_ext_hint_dbg.as_deref(),
        "voice reference update payload parsed"
    );

    let text_for_store = if text_supplied {
        Some(text_override.unwrap_or_default())
    } else {
        None
    };

    let record = state
        .voice_overrides
        .set(&voice_id, engine, temp_audio.clone(), text_for_store)
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("保存音色覆盖失败: {err}"),
            )
        })?;

    let update = VoiceOverrideUpdate {
        reference_audio: record.reference_audio.clone(),
        reference_text: record.reference_text.clone(),
    };

    state
        .synthesizer
        .apply_override(engine, &voice_id, update)
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("应用音色覆盖失败: {err}"),
            )
        })?;

    let payload = build_voice_reference_response(&state, &voice_id)?;
    let text_override = payload
        .override_reference_text
        .as_ref()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    let active_text_len = payload
        .active_reference_text
        .as_ref()
        .map(|text| text.len());
    let active_text_preview = payload
        .active_reference_text
        .as_deref()
        .map(|text| preview_text(text));
    let override_text_len = payload
        .override_reference_text
        .as_ref()
        .map(|text| text.len());
    let override_text_preview = payload
        .override_reference_text
        .as_deref()
        .map(|text| preview_text(text));
    let override_audio_path = record
        .reference_audio
        .as_ref()
        .map(|path| path.to_string_lossy().to_string());
    info!(
        target = "ishowtts::api::voices",
        voice = %payload.voice_id,
        engine = %payload.engine,
        audio_override = payload.override_audio_available,
        text_override,
        incoming_text_len,
        incoming_text_preview = incoming_text_preview.as_deref(),
        incoming_audio_ext = audio_ext_hint_dbg.as_deref(),
        active_text_len,
        active_text_preview = active_text_preview.as_deref(),
        override_text_len,
        override_text_preview = override_text_preview.as_deref(),
        override_audio_path = override_audio_path.as_deref(),
        "voice reference updated"
    );
    Ok(Json(payload))
}

#[instrument(skip(state))]
async fn delete_voice_reference(
    State(state): State<ApiState>,
    Path(voice_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let descriptor = state
        .synthesizer
        .voice_descriptor(&voice_id)
        .or_else(|| {
            state
                .synthesizer
                .voices()
                .into_iter()
                .find(|voice| voice.id == voice_id)
        })
        .ok_or((StatusCode::NOT_FOUND, format!("未知音色 '{voice_id}'")))?;
    let engine = descriptor.engine;

    debug!(
        target = "ishowtts::api::voices",
        voice = %voice_id,
        engine = %engine,
        "voice reference reset requested"
    );

    state
        .voice_overrides
        .remove(&voice_id, engine)
        .map_err(|err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("清除音色覆盖失败: {err}"),
            )
        })?;

    if let Some(baseline) = state.synthesizer.baseline(&voice_id) {
        let update = VoiceOverrideUpdate {
            reference_audio: Some(baseline.reference_audio.clone()),
            reference_text: baseline.reference_text.clone(),
        };
        state
            .synthesizer
            .apply_override(engine, &voice_id, update)
            .map_err(|err| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("恢复默认参考失败: {err}"),
                )
            })?;
    } else {
        warn!(
            target = "ishowtts::api::voices",
            voice = %voice_id,
            "baseline reference missing when clearing override"
        );
    }

    let payload = build_voice_reference_response(&state, &voice_id)?;
    let active_text_len = payload
        .active_reference_text
        .as_ref()
        .map(|text| text.len());
    let active_text_preview = payload
        .active_reference_text
        .as_deref()
        .map(|text| preview_text(text));
    let override_text_len = payload
        .override_reference_text
        .as_ref()
        .map(|text| text.len());
    let override_text_preview = payload
        .override_reference_text
        .as_deref()
        .map(|text| preview_text(text));
    info!(
        target = "ishowtts::api::voices",
        voice = %payload.voice_id,
        engine = %payload.engine,
        active_text_len,
        active_text_preview = active_text_preview.as_deref(),
        override_text_len,
        override_text_preview = override_text_preview.as_deref(),
        "voice reference reset to baseline"
    );
    Ok(Json(payload))
}

#[instrument(skip(state))]
async fn get_voice_reference_audio(
    State(state): State<ApiState>,
    Path(voice_id): Path<String>,
    Query(query): Query<VoiceReferenceAudioQuery>,
) -> Result<Response, (StatusCode, String)> {
    debug!(
        target = "ishowtts::api::voices",
        voice = %voice_id,
        source = %query.source,
        "voice reference audio requested"
    );
    let descriptor = state
        .synthesizer
        .voice_descriptor(&voice_id)
        .ok_or((StatusCode::NOT_FOUND, format!("未知音色 '{voice_id}'")))?;
    let engine = descriptor.engine;

    let (source_label, audio_path) = match query.source.to_ascii_lowercase().as_str() {
        "baseline" => {
            let baseline = state
                .synthesizer
                .baseline(&voice_id)
                .ok_or((StatusCode::NOT_FOUND, "该音色没有默认参考音频".into()))?;
            ("baseline", baseline.reference_audio)
        }
        "override" => {
            let record = state
                .voice_overrides
                .get(&voice_id, engine)
                .ok_or((StatusCode::NOT_FOUND, "尚未上传参考音频覆盖".into()))?;
            let path = record
                .reference_audio
                .ok_or((StatusCode::NOT_FOUND, "覆盖记录缺少音频文件".into()))?;
            ("override", path)
        }
        other => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("未知的 source 参数 '{other}'"),
            ));
        }
    };

    let data = fs::read(&audio_path)
        .await
        .map_err(|err| (StatusCode::NOT_FOUND, format!("读取音频失败: {err}")))?;

    let mut builder = Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "audio/wav")
        .header("Cache-Control", "no-store")
        .header("X-Voice-Reference-Source", source_label);

    if let Some(filename) = audio_path.file_name().and_then(|s| s.to_str()) {
        if let Ok(value) = HeaderValue::from_str(&format!("inline; filename=\"{}\"", filename)) {
            builder = builder.header("Content-Disposition", value);
        }
    }

    builder.body(Body::from(data)).map_err(|err| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("构建响应失败: {err}"),
        )
    })
}

fn build_voice_reference_response(
    state: &ApiState,
    voice_id: &str,
) -> Result<VoiceReferenceResponse, (StatusCode, String)> {
    let descriptor = state
        .synthesizer
        .voice_descriptor(voice_id)
        .or_else(|| {
            state
                .synthesizer
                .voices()
                .into_iter()
                .find(|voice| voice.id == voice_id)
        })
        .ok_or((StatusCode::NOT_FOUND, format!("未知音色 '{voice_id}'")))?;

    let engine = descriptor.engine;
    let baseline = state.synthesizer.baseline(voice_id);
    let override_record = state.voice_overrides.get(voice_id, engine);

    let baseline_audio_available = baseline
        .as_ref()
        .map(|record| record.reference_audio.exists())
        .unwrap_or(false);

    let override_audio_available = override_record
        .as_ref()
        .and_then(|record| record.reference_audio.as_ref())
        .map(|path| path.exists())
        .unwrap_or(false);

    Ok(VoiceReferenceResponse {
        voice_id: voice_id.to_string(),
        engine: engine.as_str().to_string(),
        engine_label: descriptor.engine_label.clone(),
        language: descriptor.language.clone(),
        active_reference_text: descriptor.reference_text.clone(),
        baseline_reference_text: baseline
            .as_ref()
            .and_then(|record| record.reference_text.clone()),
        override_reference_text: override_record
            .as_ref()
            .and_then(|record| record.reference_text.clone()),
        baseline_audio_available,
        override_audio_available,
        override_updated_at: override_record.and_then(|record| record.updated_at),
    })
}

#[instrument(skip(state, payload))]
async fn start_danmaku(
    State(state): State<ApiState>,
    Json(payload): Json<StartRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let service = state
        .danmaku
        .ok_or((StatusCode::NOT_IMPLEMENTED, "弹幕播报未启用".into()))?;
    debug!(
        target = "ishowtts::api::danmaku",
        platform = %payload.platform,
        channel = %payload.channel,
        voice_id = payload.voice_id.as_deref(),
        engine = payload.engine.as_deref(),
        "danmaku start requested"
    );
    match payload.platform.to_lowercase().as_str() {
        "twitch" => {
            let engine = match payload.engine.as_deref() {
                Some(value) => match EngineKind::from_str(value) {
                    Ok(kind) => Some(kind),
                    Err(_) => {
                        return Err((StatusCode::BAD_REQUEST, format!("不支持的模型 '{value}'")))
                    }
                },
                None => None,
            };

            let channel = service
                .start_twitch(&payload.channel, payload.voice_id.clone(), engine)
                .await
                .map_err(|err| (StatusCode::BAD_GATEWAY, err.to_string()))?;
            info!(
                target = "ishowtts::api::danmaku",
                platform = %payload.platform,
                channel = %channel,
                voice_id = payload.voice_id.as_deref(),
                engine = payload.engine.as_deref(),
                "danmaku start accepted"
            );
            Ok((
                StatusCode::ACCEPTED,
                Json(StartResponse {
                    status: "started".into(),
                    channel,
                }),
            ))
        }
        "youtube" => Err((
            StatusCode::NOT_IMPLEMENTED,
            "YouTube 弹幕播报即将支持".into(),
        )),
        other => Err((
            StatusCode::BAD_REQUEST,
            format!("unsupported platform '{other}'"),
        )),
    }
}

#[instrument(skip(state, payload))]
async fn stop_danmaku(
    State(state): State<ApiState>,
    Json(payload): Json<StopRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let service = state
        .danmaku
        .ok_or((StatusCode::NOT_IMPLEMENTED, "弹幕播报未启用".into()))?;

    debug!(
        target = "ishowtts::api::danmaku",
        platform = %payload.platform,
        channel = %payload.channel,
        "danmaku stop requested"
    );

    match payload.platform.to_lowercase().as_str() {
        "twitch" => match service.stop_twitch(&payload.channel) {
            Ok(Some(channel)) => {
                info!(
                    target = "ishowtts::api::danmaku",
                    platform = %payload.platform,
                    channel = %channel,
                    "danmaku stop accepted"
                );
                Ok((
                    StatusCode::ACCEPTED,
                    Json(StopResponse {
                        status: "stopped".into(),
                        channel: Some(channel),
                    }),
                ))
            }
            Ok(None) => {
                info!(
                    target = "ishowtts::api::danmaku",
                    platform = %payload.platform,
                    channel = %payload.channel,
                    "danmaku already idle"
                );
                Ok((
                    StatusCode::OK,
                    Json(StopResponse {
                        status: "idle".into(),
                        channel: None,
                    }),
                ))
            }
            Err(err) => Err((StatusCode::BAD_REQUEST, err.to_string())),
        },
        "youtube" => Err((
            StatusCode::NOT_IMPLEMENTED,
            "YouTube 弹幕播报即将支持".into(),
        )),
        other => Err((
            StatusCode::BAD_REQUEST,
            format!("unsupported platform '{other}'"),
        )),
    }
}

#[instrument(skip(state, payload))]
async fn enqueue_danmaku(
    State(state): State<ApiState>,
    Json(payload): Json<NormalizedMessage>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let service = state
        .danmaku
        .ok_or((StatusCode::NOT_IMPLEMENTED, "弹幕播报未启用".into()))?;
    let message_preview = match &payload.content {
        MessageContent::Text(text) | MessageContent::System(text) => preview_text(text),
    };
    let message_len = match &payload.content {
        MessageContent::Text(text) | MessageContent::System(text) => text.len(),
    };
    debug!(
        target = "ishowtts::api::danmaku",
        platform = ?payload.platform,
        channel = %payload.channel,
        user = %payload.username,
        message_len,
        message_preview = %message_preview,
        "danmaku enqueue received"
    );
    let accepted = service
        .enqueue(&payload)
        .await
        .map_err(|err| (StatusCode::BAD_GATEWAY, err.to_string()))?;
    if accepted {
        info!(
            target = "ishowtts::api::danmaku",
            platform = ?payload.platform,
            channel = %payload.channel,
            user = %payload.username,
            message_len,
            message_preview = %message_preview,
            "danmaku accepted"
        );
        Ok(StatusCode::ACCEPTED)
    } else {
        debug!(
            target = "ishowtts::api::danmaku",
            platform = ?payload.platform,
            channel = %payload.channel,
            user = %payload.username,
            message_len,
            message_preview = %message_preview,
            "danmaku dropped"
        );
        Ok(StatusCode::NO_CONTENT)
    }
}

#[instrument(skip(state))]
async fn next_danmaku(State(state): State<ApiState>) -> impl IntoResponse {
    if state.danmaku.is_some() {
        (
            StatusCode::GONE,
            "HTTP 轮询接口已弃用，请改用 WebSocket /api/danmaku/stream",
        )
            .into_response()
    } else {
        StatusCode::NOT_IMPLEMENTED.into_response()
    }
}

#[instrument(skip(state))]
async fn stream_danmaku_ws(
    State(state): State<ApiState>,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let service = state
        .danmaku
        .as_ref()
        .ok_or((StatusCode::NOT_IMPLEMENTED, "弹幕播报未启用".into()))?
        .clone();

    Ok(ws.on_upgrade(move |socket| async move {
        if let Err(err) = handle_danmaku_ws(socket, service).await {
            error!(%err, "danmaku websocket channel terminated with error");
        }
    }))
}

async fn handle_danmaku_ws(socket: WebSocket, service: Arc<DanmakuService>) -> Result<()> {
    let (mut sink, mut stream) = socket.split();

    for item in service.pending_playback() {
        if let Err(err) = send_packet(&mut sink, &item).await {
            return Err(err);
        }
    }

    let mut receiver = service.subscribe_playback();

    loop {
        tokio::select! {
            msg = receiver.recv() => {
                match msg {
                    Ok(item) => {
                        if let Err(err) = send_packet(&mut sink, &item).await {
                            return Err(err);
                        }
                    }
                    Err(RecvError::Lagged(skipped)) => {
                        warn!(skipped, "websocket listener lagged; dropping playback events");
                    }
                    Err(RecvError::Closed) => break,
                }
            }
            ws_msg = stream.next() => {
                match ws_msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(payload))) => {
                        sink.send(Message::Pong(payload)).await.ok();
                    }
                    Some(Ok(Message::Text(_))) | Some(Ok(Message::Binary(_))) | Some(Ok(Message::Pong(_))) => {
                        // ignore client data
                    }
                    Some(Err(err)) => {
                        return Err(anyhow::Error::new(err));
                    }
                }
            }
        }
    }

    Ok(())
}

async fn send_packet(sink: &mut SplitSink<WebSocket, Message>, item: &PlaybackItem) -> Result<()> {
    use serde_json::json;

    let platform = match item.platform {
        Platform::Twitch => "Twitch",
        Platform::YouTube => "YouTube",
    };

    let header = json!({
        "platform": platform,
        "channel": item.channel,
        "username": item.username,
        "display_text": item.display_text,
        "format": item.format,
        "color": item.color,
    });

    let header_bytes = serde_json::to_vec(&header).context("failed to encode playback header")?;
    let header_len =
        u32::try_from(header_bytes.len()).context("playback header too large to encode")?;

    let mut payload = Vec::with_capacity(4 + header_bytes.len() + item.audio.len());
    payload.extend_from_slice(&header_len.to_le_bytes());
    payload.extend_from_slice(&header_bytes);
    payload.extend_from_slice(&item.audio);

    sink.send(Message::Binary(payload))
        .await
        .context("failed to send playback packet over websocket")?;

    let audio_bytes = item.audio.len();
    let audio_kb = ((audio_bytes as f64) / 1024.0 * 10.0).round() / 10.0;

    info!(
        target = "ishowtts::playback",
        platform = %platform,
        channel = %item.channel,
        user = %item.username,
        sample_rate = item.sample_rate,
        audio_bytes,
        audio_kb,
        "playback packet sent"
    );

    Ok(())
}
