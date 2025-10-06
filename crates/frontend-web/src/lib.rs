use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use gloo_net::http::Request;
use gloo_timers::future::TimeoutFuture;
use js_sys::{Array, Date, Uint8Array};
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{
    BinaryType, Blob, BlobPropertyBag, CloseEvent, Event as DomEvent, File, FormData,
    HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement, MessageEvent, Url, WebSocket,
};
use yew::events::{Event, InputEvent, MouseEvent};
use yew::prelude::*;
use yew::TargetCast;

const BACKEND_URL: &str = env_backend_url();
const HISTORY_CAPACITY: usize = 100;
const PAGE_SIZE: usize = 10;
const HISTORY_STORAGE_KEY: &str = "ishowtts_history_v1";
const DANMAKU_LOG_CAPACITY: usize = 50;
const HEALTH_POLL_INTERVAL_MS: u32 = 30_000;

const fn env_backend_url() -> &'static str {
    match option_env!("ISHOWTTS_BACKEND_URL") {
        Some(url) => url,
        None => "http://127.0.0.1:27121",
    }
}

fn backend_ws_url(path: &str) -> String {
    let trimmed = BACKEND_URL.trim_end_matches('/');
    if let Some(rest) = trimmed.strip_prefix("https://") {
        format!("wss://{}{}", rest, path)
    } else if let Some(rest) = trimmed.strip_prefix("http://") {
        format!("ws://{}{}", rest, path)
    } else {
        format!("ws://{}{}", trimmed, path)
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct VoiceSummary {
    id: String,
    engine: String,
    engine_label: String,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    reference_text: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct VoiceReferenceDetail {
    voice_id: String,
    engine: String,
    engine_label: String,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    active_reference_text: Option<String>,
    #[serde(default)]
    baseline_reference_text: Option<String>,
    #[serde(default)]
    override_reference_text: Option<String>,
    baseline_audio_available: bool,
    override_audio_available: bool,
    #[serde(default)]
    override_updated_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct HealthResponse {
    status: String,
    voices: usize,
    default_voice: String,
}

#[derive(Clone, Debug, Deserialize)]
struct TtsResponse {
    #[allow(dead_code)]
    request_id: String,
    voice_id: String,
    #[serde(default)]
    engine: Option<String>,
    #[serde(default)]
    engine_label: Option<String>,
    sample_rate: u32,
    audio_base64: String,
    waveform_len: usize,
    format: String,
}

#[derive(Clone, Debug, Deserialize)]
struct DanmakuStartResponse {
    #[allow(dead_code)]
    status: String,
    channel: String,
}

#[derive(Clone, Debug, Deserialize)]
struct DanmakuStopResponse {
    #[allow(dead_code)]
    status: String,
    channel: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PacketHeader {
    platform: String,
    channel: String,
    username: String,
    display_text: String,
    format: String,
    color: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct ClipHistoryItem {
    id: usize,
    source: HistorySource,
    engine: String,
    engine_label: String,
    voice_id: String,
    text: String,
    created_at: String,
    sample_rate: u32,
    waveform_len: usize,
    format: String,
    audio_src: String,
}

#[derive(Clone, Debug, PartialEq)]
struct DanmakuLogEntry {
    timestamp: String,
    message: String,
    color: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
enum HistorySource {
    Tts,
    Danmaku,
}

impl HistorySource {
    const fn tag(&self) -> &'static str {
        match self {
            Self::Tts => "TTS",
            Self::Danmaku => "弹幕",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct ShimmyModelListResponse {
    models: Vec<ShimmyModelInfo>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct ShimmyModelInfo {
    name: String,
    #[serde(default)]
    size_bytes: Option<u64>,
    #[serde(default)]
    model_type: Option<String>,
    #[serde(default)]
    parameter_count: Option<String>,
    source: String,
}

#[derive(Clone, Debug, PartialEq)]
enum EngineModelChoice {
    Tts { engine_label: String },
    Shimmy { model_id: String },
}

#[derive(Clone, Debug, PartialEq)]
struct EngineOption {
    value: String,
    label: String,
    choice: EngineModelChoice,
}

impl EngineOption {
    fn matches_value(&self, value: &str) -> bool {
        self.value == value
    }
}

fn parse_engine_choice(value: &str) -> Option<EngineModelChoice> {
    if let Some(rest) = value.strip_prefix("tts:") {
        return Some(EngineModelChoice::Tts {
            engine_label: rest.to_string(),
        });
    }
    if let Some(rest) = value.strip_prefix("shimmy:") {
        return Some(EngineModelChoice::Shimmy {
            model_id: rest.to_string(),
        });
    }
    None
}

#[derive(Clone, Debug, Deserialize)]
struct ShimmyGenerateResponse {
    response: String,
}

#[derive(Clone, Debug, Deserialize)]
struct ShimmyTtsEnvelope {
    response: TtsResponse,
}

#[derive(Clone, Debug, PartialEq, Default)]
struct HistoryState {
    entries: VecDeque<ClipHistoryItem>,
}

enum HistoryAction {
    Push(ClipHistoryItem),
    Clear,
    Hydrate(Vec<ClipHistoryItem>),
}

impl Reducible for HistoryState {
    type Action = HistoryAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        let mut entries = self.entries.clone();
        match action {
            HistoryAction::Push(clip) => {
                entries.push_front(clip);
                while entries.len() > HISTORY_CAPACITY {
                    entries.pop_back();
                }
            }
            HistoryAction::Clear => {
                entries.clear();
            }
            HistoryAction::Hydrate(items) => {
                entries.clear();
                for clip in items.into_iter().take(HISTORY_CAPACITY) {
                    entries.push_back(clip);
                }
            }
        }
        HistoryState { entries }.into()
    }
}

#[derive(Clone, Debug, PartialEq)]
struct AdvancedTtsOptions {
    speed: String,
    target_rms: String,
    cross_fade_duration: String,
    sway_sampling_coef: String,
    cfg_strength: String,
    nfe_step: String,
    fix_duration: String,
    remove_silence: bool,
    seed: String,
}

impl Default for AdvancedTtsOptions {
    fn default() -> Self {
        Self {
            speed: String::new(),
            target_rms: String::new(),
            cross_fade_duration: String::new(),
            sway_sampling_coef: String::new(),
            cfg_strength: String::new(),
            nfe_step: String::new(),
            fix_duration: String::new(),
            remove_silence: false,
            seed: String::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum SynthesisStatus {
    Idle,
    Loading,
    Ready(String),
    Error(String),
}

impl Default for SynthesisStatus {
    fn default() -> Self {
        Self::Idle
    }
}

impl SynthesisStatus {
    fn message(&self) -> String {
        match self {
            Self::Idle => "等待输入，准备开始语音合成".to_string(),
            Self::Loading => "正在合成语音，请稍候...".to_string(),
            Self::Ready(msg) => msg.clone(),
            Self::Error(msg) => format!("⚠️ {msg}"),
        }
    }

    fn css_class(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Loading => "loading",
            Self::Ready(_) => "ready",
            Self::Error(_) => "error",
        }
    }
}

fn now_string() -> String {
    Date::new_0()
        .to_locale_string("zh-CN", &JsValue::UNDEFINED)
        .into()
}

fn log_entry(message: impl Into<String>, color: Option<String>) -> DanmakuLogEntry {
    DanmakuLogEntry {
        timestamp: now_string(),
        message: message.into(),
        color,
    }
}

fn push_log(mut logs: Vec<DanmakuLogEntry>, entry: DanmakuLogEntry) -> Vec<DanmakuLogEntry> {
    logs.insert(0, entry);
    if logs.len() > DANMAKU_LOG_CAPACITY {
        logs.truncate(DANMAKU_LOG_CAPACITY);
    }
    logs
}

fn make_object_url(format: &str, audio: &[u8]) -> Option<String> {
    let array = Uint8Array::new_with_length(audio.len() as u32);
    array.copy_from(audio);
    let parts = Array::new();
    parts.push(&array.buffer().into());
    let bag = BlobPropertyBag::new();
    bag.set_type(format);
    let blob = Blob::new_with_u8_array_sequence_and_options(parts.as_ref(), &bag).ok()?;
    Url::create_object_url_with_blob(&blob).ok()
}

fn float_value(input: &str) -> Option<serde_json::Value> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    let value: f64 = trimmed.parse::<f64>().ok()?;
    serde_json::Number::from_f64(value).map(serde_json::Value::Number)
}

fn u32_value(input: &str) -> Option<serde_json::Value> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    let value: u64 = trimmed.parse::<u64>().ok()?;
    Some(serde_json::Value::Number(value.into()))
}

#[function_component(App)]
fn app() -> Html {
    let text_state = use_state(|| String::new());
    let voices_state = use_state(Vec::<VoiceSummary>::new);
    let shimmy_models_state = use_state(Vec::<ShimmyModelInfo>::new);
    let selected_voice_state = use_state(|| Option::<String>::None);
    let selected_engine_state = use_state(|| Option::<String>::None);
    let voice_manager_open_state = use_state(|| false);
    let toast_state = use_state(|| Option::<ToastMessage>::None);
    let voice_reference_state = use_state(|| Option::<VoiceReferenceDetail>::None);
    let voice_reference_error_state = use_state(|| Option::<String>::None);
    let voice_reference_notice_state = use_state(|| Option::<String>::None);
    let voice_reference_loading_state = use_state(|| false);
    let voice_reference_text_state = use_state(String::new);
    let voice_reference_file_state = use_state(|| Option::<File>::None);
    let voice_reference_file_input = use_node_ref();

    use_effect_with((*toast_state).clone(), {
        let toast_state = toast_state.clone();
        move |current_toast| {
            if current_toast.is_some() {
                let toast_state = toast_state.clone();
                spawn_local(async move {
                    TimeoutFuture::new(3_000).await;
                    toast_state.set(None);
                });
            }
            || ()
        }
    });
    let backend_health_state = use_state(|| Option::<HealthResponse>::None);
    let health_error_state = use_state(|| Option::<String>::None);
    let status_state = use_state(SynthesisStatus::default);
    let advanced_visible = use_state(|| false);
    let advanced_state = use_state(AdvancedTtsOptions::default);
    let history_state = use_reducer(|| HistoryState::default());
    let clip_counter = use_state(|| 0usize);
    let current_page = use_state(|| 0usize);
    let detail_clip_state = use_state(|| Option::<ClipHistoryItem>::None);
    let history_hydrated = use_state(|| false);
    let danmaku_channel_state = use_state(|| String::new());
    let danmaku_status_state = use_state(|| String::from("等待启动"));
    let danmaku_active_state = use_state(|| false);
    let danmaku_stream_ready_state = use_state(|| false);
    let danmaku_active_channel_state = use_state(|| Option::<String>::None);
    let danmaku_log_state = use_state(Vec::<DanmakuLogEntry>::new);
    let danmaku_audio_state = use_state(|| Option::<String>::None);
    let danmaku_websocket = use_mut_ref(|| None::<WebSocket>);
    let danmaku_ws_message = use_mut_ref(|| None::<Closure<dyn FnMut(MessageEvent)>>);
    let danmaku_ws_error = use_mut_ref(|| None::<Closure<dyn FnMut(DomEvent)>>);
    let danmaku_ws_close = use_mut_ref(|| None::<Closure<dyn FnMut(CloseEvent)>>);

    let history_len = history_state.entries.len();
    {
        let current_page = current_page.clone();
        use_effect_with(history_len, move |len| {
            let total_pages = if *len == 0 {
                1
            } else {
                (*len + PAGE_SIZE - 1) / PAGE_SIZE
            };
            if *current_page >= total_pages {
                current_page.set(total_pages - 1);
            }
            || ()
        });
    }

    {
        let history_state = history_state.clone();
        let history_hydrated = history_hydrated.clone();
        let current_page = current_page.clone();
        use_effect_with((), move |_| {
            if !*history_hydrated {
                if let Some(window) = web_sys::window() {
                    if let Ok(Some(storage)) = window.local_storage() {
                        if let Ok(Some(raw)) = storage.get_item(HISTORY_STORAGE_KEY) {
                            if let Ok(items) = serde_json::from_str::<Vec<ClipHistoryItem>>(&raw) {
                                if !items.is_empty() {
                                    history_state.dispatch(HistoryAction::Hydrate(items));
                                    current_page.set(0);
                                }
                            }
                        }
                    }
                }
                history_hydrated.set(true);
            }
            || ()
        });
    }

    {
        let history_hydrated = history_hydrated.clone();
        let entries = history_state.entries.clone();
        use_effect_with((entries, *history_hydrated), move |(entries, hydrated)| {
            if *hydrated {
                if let Some(window) = web_sys::window() {
                    if let Ok(Some(storage)) = window.local_storage() {
                        if entries.is_empty() {
                            let _ = storage.remove_item(HISTORY_STORAGE_KEY);
                        } else if let Ok(json) =
                            serde_json::to_string(&entries.iter().cloned().collect::<Vec<_>>())
                        {
                            let _ = storage.set_item(HISTORY_STORAGE_KEY, &json);
                        }
                    }
                }
            }
            || ()
        });
    }

    {
        let ws_ref = danmaku_websocket.clone();
        let handler_ref = danmaku_ws_message.clone();
        let error_ref = danmaku_ws_error.clone();
        let close_ref = danmaku_ws_close.clone();
        let audio_state = danmaku_audio_state.clone();
        let log_state = danmaku_log_state.clone();
        let status_state = danmaku_status_state.clone();
        let active_state = danmaku_active_state.clone();
        let active_channel_state = danmaku_active_channel_state.clone();
        let stream_ready_state = danmaku_stream_ready_state.clone();
        let cleanup_audio_state = danmaku_audio_state.clone();
        let history_state_ws = history_state.clone();
        let clip_counter_ws = clip_counter.clone();
        let selected_voice_state_ws = selected_voice_state.clone();
        let selected_engine_state_ws = selected_engine_state.clone();
        let voices_state_ws = voices_state.clone();

        use_effect_with((), move |_| {
            let ws_url = backend_ws_url("/api/danmaku/stream");
            match WebSocket::new(&ws_url) {
                Ok(ws) => {
                    ws.set_binary_type(BinaryType::Arraybuffer);

                    let message_handler = {
                        let audio_state = audio_state.clone();
                        let log_state = log_state.clone();
                        let status_state = status_state.clone();
                        let active_state = active_state.clone();
                        let active_channel_state = active_channel_state.clone();
                        let stream_ready_state = stream_ready_state.clone();
                        let history_state = history_state_ws.clone();
                        let clip_counter = clip_counter_ws.clone();
                        let selected_voice_state = selected_voice_state_ws.clone();
                        let selected_engine_state = selected_engine_state_ws.clone();
                        let voices_state = voices_state_ws.clone();
                        Closure::wrap(Box::new(move |event: MessageEvent| {
                            if let Ok(buffer) = event.data().dyn_into::<js_sys::ArrayBuffer>() {
                                let array = Uint8Array::new(&buffer);
                                let mut bytes = vec![0u8; array.length() as usize];
                                array.copy_to(&mut bytes);

                                if bytes.len() < 4 {
                                    status_state.set("解析弹幕音频失败: 包长度不足".into());
                                    return;
                                }
                                let header_len =
                                    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
                                        as usize;
                                if bytes.len() < 4 + header_len {
                                    status_state.set("解析弹幕音频失败: 包头长度异常".into());
                                    return;
                                }

                                let header_bytes = &bytes[4..4 + header_len];
                                let audio_bytes = bytes[4 + header_len..].to_vec();

                                match serde_json::from_slice::<PacketHeader>(header_bytes) {
                                    Ok(header) => {
                                        if let Some(current) = (*audio_state).clone() {
                                            let _ = Url::revoke_object_url(&current);
                                        }
                                        if let Some(url) =
                                            make_object_url(&header.format, &audio_bytes)
                                        {
                                            audio_state.set(Some(url));
                                        }

                                        let entry = log_entry(
                                            format!(
                                                "{} ({})：{}",
                                                header.username,
                                                header.platform,
                                                header.display_text
                                            ),
                                            header.color.clone(),
                                        );
                                        let history = push_log((*log_state).clone(), entry);
                                        log_state.set(history);

                                        status_state.set(format!("正在播报: {}", header.channel));
                                        active_channel_state.set(Some(header.channel.clone()));
                                        active_state.set(true);
                                        stream_ready_state.set(true);

                                        let mut clip_id = *clip_counter;
                                        clip_id += 1;
                                        clip_counter.set(clip_id);

                                        let voices_snapshot = (*voices_state).clone();
                                        let selected_voice = (*selected_voice_state).clone();
                                        let mut engine_value = String::from("danmaku");
                                        let mut engine_label =
                                            format!("弹幕 · {}", header.platform);
                                        let mut voice_label =
                                            format!("{}@{}", header.username, header.channel);

                                        if let Some(voice_id) = selected_voice.clone() {
                                            if let Some(meta) =
                                                voices_snapshot.iter().find(|v| v.id == voice_id)
                                            {
                                                engine_value = meta.engine.clone();
                                                engine_label = meta.engine_label.clone();
                                                voice_label = meta.id.clone();
                                            } else {
                                                voice_label = voice_id;
                                            }
                                        }

                                        if let Some(label) = (*selected_engine_state).clone() {
                                            engine_label = label;
                                        }

                                        let clip_text = format!(
                                            "{} ({})：{}",
                                            header.username, header.platform, header.display_text
                                        );

                                        let audio_base64 = BASE64.encode(&audio_bytes);
                                        let audio_src = format!(
                                            "data:{};base64,{}",
                                            header.format, audio_base64
                                        );

                                        let clip = ClipHistoryItem {
                                            id: clip_id,
                                            source: HistorySource::Danmaku,
                                            engine: engine_value,
                                            engine_label,
                                            voice_id: voice_label,
                                            text: clip_text,
                                            created_at: now_string(),
                                            sample_rate: 24_000,
                                            waveform_len: audio_bytes.len(),
                                            format: header.format.clone(),
                                            audio_src,
                                        };

                                        history_state.dispatch(HistoryAction::Push(clip));
                                    }
                                    Err(err) => {
                                        status_state.set(format!("解析弹幕音频失败: {err}"));
                                    }
                                }
                            } else if let Some(text) = event.data().as_string() {
                                status_state.set(format!(
                                    "收到未知的弹幕消息格式: {}",
                                    text.chars().take(128).collect::<String>()
                                ));
                            }
                        }) as Box<dyn FnMut(MessageEvent)>)
                    };
                    ws.set_onmessage(Some(message_handler.as_ref().unchecked_ref()));
                    handler_ref.borrow_mut().replace(message_handler);

                    let error_handler = {
                        let status_state = status_state.clone();
                        let stream_ready_state = stream_ready_state.clone();
                        Closure::wrap(Box::new(move |_event: DomEvent| {
                            status_state.set("弹幕推送连接异常，正在重试...".into());
                            stream_ready_state.set(false);
                        }) as Box<dyn FnMut(DomEvent)>)
                    };
                    ws.set_onerror(Some(error_handler.as_ref().unchecked_ref()));
                    error_ref.borrow_mut().replace(error_handler);

                    let close_handler = {
                        let status_state = status_state.clone();
                        let active_state = active_state.clone();
                        let stream_ready_state = stream_ready_state.clone();
                        Closure::wrap(Box::new(move |_event: CloseEvent| {
                            status_state.set("弹幕推送连接已断开".into());
                            active_state.set(false);
                            stream_ready_state.set(false);
                        }) as Box<dyn FnMut(CloseEvent)>)
                    };
                    ws.set_onclose(Some(close_handler.as_ref().unchecked_ref()));
                    close_ref.borrow_mut().replace(close_handler);

                    ws_ref.borrow_mut().replace(ws);
                }
                Err(err) => {
                    status_state.set(format!("连接弹幕流失败: {:?}", err));
                }
            }

            move || {
                if let Some(current) = (*cleanup_audio_state).clone() {
                    let _ = Url::revoke_object_url(&current);
                    cleanup_audio_state.set(None);
                }
                if let Some(ws) = ws_ref.borrow_mut().take() {
                    let _ = ws.close();
                }
                handler_ref.borrow_mut().take();
                error_ref.borrow_mut().take();
                close_ref.borrow_mut().take();
                stream_ready_state.set(false);
            }
        });
    }

    {
        let voices_state = voices_state.clone();
        let selected_voice_state = selected_voice_state.clone();
        let selected_engine_state = selected_engine_state.clone();
        let voices_state = voices_state.clone();
        let selected_engine_state = selected_engine_state.clone();
        let status_state = status_state.clone();
        use_effect_with((), move |_| {
            let voices_state = voices_state.clone();
            let selected_voice_state = selected_voice_state.clone();
            let selected_engine_state = selected_engine_state.clone();
            let status_state = status_state.clone();
            spawn_local(async move {
                match Request::get(&format!("{BACKEND_URL}/api/voices"))
                    .send()
                    .await
                {
                    Ok(resp) => match resp.json::<Vec<VoiceSummary>>().await {
                        Ok(voices) if !voices.is_empty() => {
                            let mut engine_order = Vec::new();
                            for voice in &voices {
                                if !engine_order.contains(&voice.engine_label) {
                                    engine_order.push(voice.engine_label.clone());
                                }
                            }

                            let mut engine_to_use = (*selected_engine_state).clone();
                            if engine_to_use
                                .as_ref()
                                .map(|engine| engine_order.contains(engine))
                                != Some(true)
                            {
                                engine_to_use = engine_order.first().cloned();
                            }

                            let voice_to_use = {
                                let current_voice = (*selected_voice_state).clone();
                                let engine_ref = engine_to_use.clone();
                                current_voice.and_then(|voice_id| {
                                    voices
                                        .iter()
                                        .find(|v| {
                                            v.id == voice_id
                                                && Some(v.engine_label.clone()) == engine_ref
                                        })
                                        .map(|v| v.id.clone())
                                })
                            }
                            .or_else(|| {
                                engine_to_use.as_ref().and_then(|engine| {
                                    voices
                                        .iter()
                                        .find(|v| &v.engine_label == engine)
                                        .map(|v| v.id.clone())
                                })
                            });

                            voices_state.set(voices);
                            selected_engine_state.set(engine_to_use);
                            selected_voice_state.set(voice_to_use);
                        }
                        Ok(_) => {
                            status_state.set(SynthesisStatus::Error("后端未配置任何音色".into()));
                        }
                        Err(err) => status_state
                            .set(SynthesisStatus::Error(format!("解析音色列表失败: {err}"))),
                    },
                    Err(err) => {
                        status_state.set(SynthesisStatus::Error(format!("请求音色列表失败: {err}")))
                    }
                }
            });
            || ()
        });
    }

    {
        let shimmy_models_state = shimmy_models_state.clone();
        let status_state = status_state.clone();
        use_effect_with((), move |_| {
            let shimmy_models_state = shimmy_models_state.clone();
            let status_state = status_state.clone();
            spawn_local(async move {
                match Request::get(&format!("{BACKEND_URL}/shimmy/models"))
                    .send()
                    .await
                {
                    Ok(resp) => match resp.json::<ShimmyModelListResponse>().await {
                        Ok(list) => shimmy_models_state.set(list.models),
                        Err(err) => status_state
                            .set(SynthesisStatus::Error(format!("解析模型列表失败: {err}"))),
                    },
                    Err(err) => {
                        status_state.set(SynthesisStatus::Error(format!("请求模型列表失败: {err}")))
                    }
                }
            });
            || ()
        });
    }

    {
        let voice_manager_open_state = voice_manager_open_state.clone();
        let selected_voice_state = selected_voice_state.clone();
        let voice_reference_state = voice_reference_state.clone();
        let voice_reference_error_state = voice_reference_error_state.clone();
        let voice_reference_notice_state = voice_reference_notice_state.clone();
        let voice_reference_loading_state = voice_reference_loading_state.clone();
        let voice_reference_text_state = voice_reference_text_state.clone();
        let voice_reference_file_state = voice_reference_file_state.clone();
        use_effect_with(
            (*voice_manager_open_state, (*selected_voice_state).clone()),
            move |(open, selected): &(bool, Option<String>)| {
                if !*open {
                    voice_reference_state.set(None);
                    voice_reference_error_state.set(None);
                    voice_reference_notice_state.set(None);
                    voice_reference_text_state.set(String::new());
                    voice_reference_file_state.set(None);
                    voice_reference_loading_state.set(false);
                } else {
                    voice_reference_file_state.set(None);
                    voice_reference_notice_state.set(None);

                    match selected.clone() {
                        Some(voice_id) => {
                            voice_reference_loading_state.set(true);
                            voice_reference_error_state.set(None);
                            let voice_reference_state = voice_reference_state.clone();
                            let voice_reference_error_state = voice_reference_error_state.clone();
                            let voice_reference_loading_state =
                                voice_reference_loading_state.clone();
                            let voice_reference_text_state = voice_reference_text_state.clone();
                            spawn_local(async move {
                                let url = format!("{BACKEND_URL}/api/voices/{voice_id}/reference");
                                match Request::get(&url).send().await {
                                    Ok(resp) => match resp.json::<VoiceReferenceDetail>().await {
                                        Ok(detail) => {
                                            let next_text = detail
                                                .override_reference_text
                                                .clone()
                                                .or(detail.active_reference_text.clone())
                                                .unwrap_or_default();
                                            voice_reference_state.set(Some(detail));
                                            voice_reference_text_state.set(next_text);
                                            voice_reference_loading_state.set(false);
                                        }
                                        Err(err) => {
                                            voice_reference_error_state
                                                .set(Some(format!("解析音色覆盖信息失败: {err}")));
                                            voice_reference_state.set(None);
                                            voice_reference_loading_state.set(false);
                                        }
                                    },
                                    Err(err) => {
                                        voice_reference_error_state
                                            .set(Some(format!("请求音色覆盖信息失败: {err}")));
                                        voice_reference_state.set(None);
                                        voice_reference_loading_state.set(false);
                                    }
                                }
                            });
                        }
                        None => {
                            voice_reference_state.set(None);
                            voice_reference_text_state.set(String::new());
                            voice_reference_error_state.set(Some("尚未选择音色".into()));
                            voice_reference_loading_state.set(false);
                        }
                    }
                }

                || ()
            },
        );
    }

    {
        let health_state = backend_health_state.clone();
        let health_error_state = health_error_state.clone();
        use_effect_with((), move |_| {
            let health_state = health_state.clone();
            let health_error_state = health_error_state.clone();
            spawn_local(async move {
                loop {
                    match Request::get(&format!("{BACKEND_URL}/api/health"))
                        .send()
                        .await
                    {
                        Ok(resp) => match resp.json::<HealthResponse>().await {
                            Ok(health) => {
                                health_state.set(Some(health));
                                health_error_state.set(None);
                            }
                            Err(err) => {
                                health_error_state.set(Some(format!("解析健康信息失败: {err}")))
                            }
                        },
                        Err(err) => {
                            health_error_state.set(Some(format!("请求健康信息失败: {err}")))
                        }
                    }
                    TimeoutFuture::new(HEALTH_POLL_INTERVAL_MS).await;
                }
            });
            || ()
        });
    }

    let on_text_input = {
        let text_state = text_state.clone();
        Callback::from(move |event: InputEvent| {
            if let Some(input) = event.target_dyn_into::<HtmlTextAreaElement>() {
                text_state.set(input.value());
            }
        })
    };

    let voices_state_for_model = voices_state.clone();
    let on_model_change = {
        let selected_engine_state = selected_engine_state.clone();
        let selected_voice_state = selected_voice_state.clone();
        let voices_state = voices_state_for_model.clone();
        Callback::from(move |event: Event| {
            if let Some(select) = event.target_dyn_into::<HtmlSelectElement>() {
                let value = select.value();
                if value.is_empty() {
                    selected_engine_state.set(None);
                    selected_voice_state.set(None);
                } else {
                    let voices = (*voices_state).clone();
                    let current_voice = (*selected_voice_state).clone();
                    let choice = parse_engine_choice(&value);
                    let next_voice = match choice {
                        Some(EngineModelChoice::Tts { ref engine_label }) => voices
                            .iter()
                            .find(|v| &v.engine_label == engine_label)
                            .map(|v| v.id.clone())
                            .or_else(|| voices.first().map(|v| v.id.clone())),
                        Some(EngineModelChoice::Shimmy { .. }) => {
                            if let Some(existing) = current_voice {
                                if voices.iter().any(|v| v.id == existing) {
                                    Some(existing)
                                } else {
                                    voices.first().map(|v| v.id.clone())
                                }
                            } else {
                                voices.first().map(|v| v.id.clone())
                            }
                        }
                        None => voices.first().map(|v| v.id.clone()),
                    };
                    selected_engine_state.set(Some(value));
                    selected_voice_state.set(next_voice);
                }
            }
        })
    };

    let on_voice_change = {
        let selected_voice_state = selected_voice_state.clone();
        Callback::from(move |event: Event| {
            if let Some(select) = event.target_dyn_into::<HtmlSelectElement>() {
                let value = select.value();
                if value.is_empty() {
                    selected_voice_state.set(None);
                } else {
                    selected_voice_state.set(Some(value));
                }
            }
        })
    };

    let on_toggle_advanced = {
        let advanced_visible = advanced_visible.clone();
        Callback::from(move |_| {
            advanced_visible.set(!*advanced_visible);
        })
    };

    let on_reset_advanced = {
        let advanced_state = advanced_state.clone();
        Callback::from(move |_| {
            advanced_state.set(AdvancedTtsOptions::default());
        })
    };

    let make_input_handler =
        |mut_field: fn(&mut AdvancedTtsOptions) -> &mut String| -> Callback<InputEvent> {
            let advanced_state = advanced_state.clone();
            Callback::from(move |event: InputEvent| {
                if let Some(input) = event.target_dyn_into::<HtmlInputElement>() {
                    let mut opts = (*advanced_state).clone();
                    *mut_field(&mut opts) = input.value();
                    advanced_state.set(opts);
                }
            })
        };

    let speed_input = make_input_handler(|opts| &mut opts.speed);
    let target_rms_input = make_input_handler(|opts| &mut opts.target_rms);
    let cross_fade_input = make_input_handler(|opts| &mut opts.cross_fade_duration);
    let sway_input = make_input_handler(|opts| &mut opts.sway_sampling_coef);
    let cfg_input = make_input_handler(|opts| &mut opts.cfg_strength);
    let nfe_input = make_input_handler(|opts| &mut opts.nfe_step);
    let fix_duration_input = make_input_handler(|opts| &mut opts.fix_duration);
    let seed_input = make_input_handler(|opts| &mut opts.seed);

    let remove_silence_toggle = {
        let advanced_state = advanced_state.clone();
        Callback::from(move |event: Event| {
            if let Some(input) = event.target_dyn_into::<HtmlInputElement>() {
                let mut opts = (*advanced_state).clone();
                opts.remove_silence = input.checked();
                advanced_state.set(opts);
            }
        })
    };

    let on_reference_text_change = {
        let voice_reference_text_state = voice_reference_text_state.clone();
        let voice_reference_notice_state = voice_reference_notice_state.clone();
        let voice_reference_error_state = voice_reference_error_state.clone();
        Callback::from(move |event: InputEvent| {
            if let Some(textarea) = event.target_dyn_into::<HtmlTextAreaElement>() {
                voice_reference_text_state.set(textarea.value());
                voice_reference_notice_state.set(None);
                voice_reference_error_state.set(None);
            }
        })
    };

    let on_reference_file_change = {
        let voice_reference_file_state = voice_reference_file_state.clone();
        let voice_reference_notice_state = voice_reference_notice_state.clone();
        let voice_reference_error_state = voice_reference_error_state.clone();
        Callback::from(move |event: Event| {
            if let Some(input) = event.target_dyn_into::<HtmlInputElement>() {
                let files = input.files();
                if let Some(files) = files {
                    if files.length() > 0 {
                        let file = files.item(0);
                        voice_reference_file_state.set(file);
                    } else {
                        voice_reference_file_state.set(None);
                    }
                }
                voice_reference_notice_state.set(None);
                voice_reference_error_state.set(None);
            }
        })
    };

    let on_reference_file_clear = {
        let voice_reference_file_state = voice_reference_file_state.clone();
        let voice_reference_notice_state = voice_reference_notice_state.clone();
        let voice_reference_error_state = voice_reference_error_state.clone();
        let reference_file_input = voice_reference_file_input.clone();
        Callback::from(move |_| {
            voice_reference_file_state.set(None);
            voice_reference_notice_state.set(None);
            voice_reference_error_state.set(None);
            if let Some(input) = reference_file_input.cast::<HtmlInputElement>() {
                input.set_value("");
            }
        })
    };

    let toast_for_save = toast_state.clone();
    let modal_state_for_save = voice_manager_open_state.clone();
    let on_reference_save = {
        let selected_voice_state = selected_voice_state.clone();
        let voice_reference_text_state = voice_reference_text_state.clone();
        let voice_reference_file_state = voice_reference_file_state.clone();
        let voice_reference_state = voice_reference_state.clone();
        let voice_reference_error_state = voice_reference_error_state.clone();
        let voice_reference_notice_state = voice_reference_notice_state.clone();
        let voice_reference_loading_state = voice_reference_loading_state.clone();
        let voice_reference_text_state_reset = voice_reference_text_state.clone();
        let voice_reference_file_state_reset = voice_reference_file_state.clone();
        let reference_file_input = voice_reference_file_input.clone();
        let toast_success = toast_for_save.clone();
        let modal_state = modal_state_for_save.clone();
        Callback::from(move |event: MouseEvent| {
            event.prevent_default();
            let Some(voice_id) = (*selected_voice_state).clone() else {
                voice_reference_error_state.set(Some("尚未选择音色".into()));
                return;
            };

            let text_value = (*voice_reference_text_state).clone();
            let file_value = (*voice_reference_file_state).clone();

            if file_value.is_none() && text_value.trim().is_empty() {
                voice_reference_error_state.set(Some("请上传参考音频或输入参考文本".into()));
                return;
            }

            voice_reference_loading_state.set(true);
            voice_reference_error_state.set(None);
            voice_reference_notice_state.set(None);

            let voice_reference_state = voice_reference_state.clone();
            let voice_reference_error_state = voice_reference_error_state.clone();
            let voice_reference_notice_state = voice_reference_notice_state.clone();
            let voice_reference_loading_state = voice_reference_loading_state.clone();
            let voice_reference_text_state = voice_reference_text_state_reset.clone();
            let voice_reference_file_state = voice_reference_file_state_reset.clone();
            let reference_file_input = reference_file_input.clone();
            let toast_success = toast_success.clone();
            let modal_state = modal_state.clone();
            spawn_local(async move {
                let form = match FormData::new() {
                    Ok(data) => data,
                    Err(err) => {
                        voice_reference_error_state.set(Some(format!("创建表单失败: {:?}", err)));
                        voice_reference_loading_state.set(false);
                        return;
                    }
                };

                if !text_value.trim().is_empty() {
                    if let Err(err) = form.append_with_str("text", text_value.trim()) {
                        voice_reference_error_state.set(Some(format!("附加文本失败: {:?}", err)));
                        voice_reference_loading_state.set(false);
                        return;
                    }
                }

                if let Some(file) = file_value.clone() {
                    if let Err(err) =
                        form.append_with_blob_and_filename("audio", &file, &file.name())
                    {
                        voice_reference_error_state.set(Some(format!("附加音频失败: {:?}", err)));
                        voice_reference_loading_state.set(false);
                        return;
                    }
                }

                let builder =
                    Request::post(&format!("{BACKEND_URL}/api/voices/{}/reference", voice_id));

                let response = match builder.body(form) {
                    Ok(request) => request.send().await,
                    Err(err) => {
                        voice_reference_error_state.set(Some(format!("发送请求失败: {err}")));
                        voice_reference_loading_state.set(false);
                        return;
                    }
                };

                match response {
                    Ok(resp) => match resp.json::<VoiceReferenceDetail>().await {
                        Ok(detail) => {
                            let next_text = detail
                                .override_reference_text
                                .clone()
                                .or(detail.active_reference_text.clone())
                                .unwrap_or_default();
                            voice_reference_state.set(Some(detail));
                            voice_reference_text_state.set(next_text);
                            voice_reference_file_state.set(None);
                            voice_reference_notice_state.set(Some("参考覆盖已保存".into()));
                            toast_success.set(Some(ToastMessage::success("参考音色已保存")));
                            modal_state.set(false);
                            voice_reference_loading_state.set(false);
                            if let Some(input) = reference_file_input.cast::<HtmlInputElement>() {
                                input.set_value("");
                            }
                        }
                        Err(err) => {
                            voice_reference_error_state
                                .set(Some(format!("解析服务响应失败: {err}")));
                            voice_reference_loading_state.set(false);
                        }
                    },
                    Err(err) => {
                        voice_reference_error_state.set(Some(format!("请求失败: {err}")));
                        voice_reference_loading_state.set(false);
                    }
                }
            });
        })
    };

    let toast_for_reset = toast_state.clone();
    let modal_state_for_reset = voice_manager_open_state.clone();
    let on_reference_reset = {
        let selected_voice_state = selected_voice_state.clone();
        let voice_reference_state = voice_reference_state.clone();
        let voice_reference_error_state = voice_reference_error_state.clone();
        let voice_reference_notice_state = voice_reference_notice_state.clone();
        let voice_reference_loading_state = voice_reference_loading_state.clone();
        let voice_reference_text_state = voice_reference_text_state.clone();
        let voice_reference_file_state = voice_reference_file_state.clone();
        let reference_file_input = voice_reference_file_input.clone();
        let toast_info = toast_for_reset.clone();
        let modal_state = modal_state_for_reset.clone();
        Callback::from(move |event: MouseEvent| {
            event.prevent_default();
            let Some(voice_id) = (*selected_voice_state).clone() else {
                voice_reference_error_state.set(Some("尚未选择音色".into()));
                return;
            };

            voice_reference_loading_state.set(true);
            voice_reference_error_state.set(None);
            voice_reference_notice_state.set(None);

            let voice_reference_state = voice_reference_state.clone();
            let voice_reference_error_state = voice_reference_error_state.clone();
            let voice_reference_notice_state = voice_reference_notice_state.clone();
            let voice_reference_loading_state = voice_reference_loading_state.clone();
            let voice_reference_text_state = voice_reference_text_state.clone();
            let voice_reference_file_state = voice_reference_file_state.clone();
            let reference_file_input = reference_file_input.clone();
            let toast_info = toast_info.clone();
            let modal_state = modal_state.clone();
            spawn_local(async move {
                match Request::delete(&format!("{BACKEND_URL}/api/voices/{}/reference", voice_id))
                    .send()
                    .await
                {
                    Ok(resp) => match resp.json::<VoiceReferenceDetail>().await {
                        Ok(detail) => {
                            let next_text = detail
                                .override_reference_text
                                .clone()
                                .or(detail.active_reference_text.clone())
                                .unwrap_or_default();
                            voice_reference_state.set(Some(detail));
                            voice_reference_text_state.set(next_text);
                            voice_reference_file_state.set(None);
                            voice_reference_notice_state.set(Some("已恢复默认参考".into()));
                            toast_info.set(Some(ToastMessage::info("已恢复默认参考")));
                            modal_state.set(false);
                            voice_reference_loading_state.set(false);
                            if let Some(input) = reference_file_input.cast::<HtmlInputElement>() {
                                input.set_value("");
                            }
                        }
                        Err(err) => {
                            voice_reference_error_state
                                .set(Some(format!("解析服务响应失败: {err}")));
                            voice_reference_loading_state.set(false);
                        }
                    },
                    Err(err) => {
                        voice_reference_error_state.set(Some(format!("请求失败: {err}")));
                        voice_reference_loading_state.set(false);
                    }
                }
            });
        })
    };

    let text_state_submit = text_state.clone();
    let selected_voice_state_submit = selected_voice_state.clone();
    let selected_engine_state_submit = selected_engine_state.clone();
    let advanced_state_submit = advanced_state.clone();
    let status_state_submit = status_state.clone();
    let history_state_submit = history_state.clone();
    let clip_counter_submit = clip_counter.clone();
    let voices_state_submit = voices_state.clone();

    let on_submit = {
        let text_state = text_state_submit;
        let selected_voice_state = selected_voice_state_submit;
        let selected_engine_state = selected_engine_state_submit;
        let advanced_state = advanced_state_submit;
        let status_state = status_state_submit;
        let history_state = history_state_submit;
        let clip_counter = clip_counter_submit;
        let voices_state = voices_state_submit;
        let engine_options = engine_options_snapshot.clone();

        Callback::from(move |_| {
            let text = (*text_state).trim().to_string();
            if text.is_empty() {
                status_state.set(SynthesisStatus::Error("请输入要合成的文本".into()));
                return;
            }

            let voice_id = match (*selected_voice_state).clone() {
                Some(value) => value,
                None => {
                    status_state.set(SynthesisStatus::Error("尚未选择音色".into()));
                    return;
                }
            };

            let voices_snapshot = (*voices_state).clone();
            let Some(voice_meta) = voices_snapshot.iter().find(|v| v.id == voice_id) else {
                status_state.set(SynthesisStatus::Error("找不到对应的音色".into()));
                return;
            };

            let engine_option = {
                let current = (*selected_engine_state).clone();
                current
                    .and_then(|value| {
                        engine_options
                            .iter()
                            .find(|opt| opt.value == value)
                            .cloned()
                    })
                    .or_else(|| engine_options.first().cloned())
            };

            let Some(engine_option) = engine_option else {
                status_state.set(SynthesisStatus::Error("尚未选择模型".into()));
                return;
            };

            if let EngineModelChoice::Tts { ref engine_label } = engine_option.choice {
                if voice_meta.engine_label != *engine_label {
                    status_state.set(SynthesisStatus::Error("音色不属于当前模型".into()));
                    return;
                }
            }

            let engine_value = voice_meta.engine.clone();
            let engine_label_display = engine_option.label.clone();
            let engine_choice = engine_option.choice.clone();
            let engine_prompt_value = serde_json::Value::String(engine_value.clone());

            status_state.set(SynthesisStatus::Loading);
            let options = (*advanced_state).clone();
            let mut payload = serde_json::Map::new();
            payload.insert("text".into(), serde_json::Value::String(text.clone()));
            payload.insert(
                "voice_id".into(),
                serde_json::Value::String(voice_id.clone()),
            );
            payload.insert("engine".into(), engine_prompt_value);

            if let Some(value) = float_value(&options.speed) {
                payload.insert("speed".into(), value);
            }
            if let Some(value) = float_value(&options.target_rms) {
                payload.insert("target_rms".into(), value);
            }
            if let Some(value) = float_value(&options.cross_fade_duration) {
                payload.insert("cross_fade_duration".into(), value);
            }
            if let Some(value) = float_value(&options.sway_sampling_coef) {
                payload.insert("sway_sampling_coef".into(), value);
            }
            if let Some(value) = float_value(&options.cfg_strength) {
                payload.insert("cfg_strength".into(), value);
            }
            if let Some(value) = u32_value(&options.nfe_step) {
                payload.insert("nfe_step".into(), value);
            }
            if let Some(value) = float_value(&options.fix_duration) {
                payload.insert("fix_duration".into(), value);
            }
            if options.remove_silence {
                payload.insert("remove_silence".into(), serde_json::Value::Bool(true));
            }
            if let Some(value) = u32_value(&options.seed) {
                payload.insert("seed".into(), value);
            }

            let payload_value = serde_json::Value::Object(payload.clone());
            let history_state = history_state.clone();
            let status_state = status_state.clone();
            let clip_counter = clip_counter.clone();
            let engine_value_clone = engine_value.clone();
            let engine_label_clone = engine_label_display.clone();
            let text_clone = text.clone();
            let engine_choice_clone = engine_choice.clone();

            spawn_local(async move {
                let handle_success = |data: TtsResponse| {
                    let mut clip_id = *clip_counter;
                    clip_id += 1;
                    clip_counter.set(clip_id);

                    let audio_src = format!("data:{};base64,{}", data.format, data.audio_base64);
                    let clip = ClipHistoryItem {
                        id: clip_id,
                        source: HistorySource::Tts,
                        engine: data
                            .engine
                            .clone()
                            .unwrap_or_else(|| engine_value_clone.clone()),
                        engine_label: data
                            .engine_label
                            .clone()
                            .unwrap_or_else(|| engine_label_clone.clone()),
                        voice_id: data.voice_id.clone(),
                        text: text_clone.clone(),
                        created_at: now_string(),
                        sample_rate: data.sample_rate,
                        waveform_len: data.waveform_len,
                        format: data.format.clone(),
                        audio_src,
                    };
                    history_state.dispatch(HistoryAction::Push(clip));
                    status_state.set(SynthesisStatus::Ready("生成完成 ✅".into()));
                };

                match engine_choice_clone {
                    EngineModelChoice::Tts { .. } => {
                        let request_body = payload_value.to_string();
                        let request = Request::post(&format!("{BACKEND_URL}/api/tts"))
                            .header("Content-Type", "application/json")
                            .body(request_body);

                        let response = match request {
                            Ok(req) => req.send().await,
                            Err(err) => {
                                status_state
                                    .set(SynthesisStatus::Error(format!("构建请求失败: {err}")));
                                return;
                            }
                        };

                        match response {
                            Ok(resp) => match resp.json::<TtsResponse>().await {
                                Ok(data) => handle_success(data),
                                Err(err) => status_state
                                    .set(SynthesisStatus::Error(format!("解析响应失败: {err}"))),
                            },
                            Err(err) => {
                                status_state.set(SynthesisStatus::Error(format!("请求失败: {err}")))
                            }
                        }
                    }
                    EngineModelChoice::Shimmy { model_id } => {
                        let shimmy_body = serde_json::json!({
                            "model": model_id,
                            "prompt": payload_value.to_string(),
                        });

                        let request = Request::post(&format!("{BACKEND_URL}/shimmy/generate"))
                            .header("Content-Type", "application/json")
                            .body(shimmy_body.to_string());

                        let response = match request {
                            Ok(req) => req.send().await,
                            Err(err) => {
                                status_state
                                    .set(SynthesisStatus::Error(format!("构建请求失败: {err}")));
                                return;
                            }
                        };

                        match response {
                            Ok(resp) => match resp.json::<ShimmyGenerateResponse>().await {
                                Ok(data) => {
                                    match serde_json::from_str::<ShimmyTtsEnvelope>(&data.response)
                                    {
                                        Ok(envelope) => handle_success(envelope.response),
                                        Err(err) => status_state.set(SynthesisStatus::Error(
                                            format!("解析 Shimmy 响应失败: {err}"),
                                        )),
                                    }
                                }
                                Err(err) => status_state
                                    .set(SynthesisStatus::Error(format!("解析响应失败: {err}"))),
                            },
                            Err(err) => {
                                status_state.set(SynthesisStatus::Error(format!("请求失败: {err}")))
                            }
                        }
                    }
                }
            });
        })
    };

    let on_clear_history = {
        let history_state = history_state.clone();
        let detail_clip_state = detail_clip_state.clone();
        Callback::from(move |_| {
            detail_clip_state.set(None);
            history_state.dispatch(HistoryAction::Clear);
        })
    };

    let on_start_danmaku = {
        let channel_state = danmaku_channel_state.clone();
        let status_state = danmaku_status_state.clone();
        let active_state = danmaku_active_state.clone();
        let active_channel_state = danmaku_active_channel_state.clone();
        let log_state = danmaku_log_state.clone();
        let stream_ready_state = danmaku_stream_ready_state.clone();
        let audio_state = danmaku_audio_state.clone();
        let selected_voice_state = selected_voice_state.clone();
        let selected_engine_state = selected_engine_state.clone();
        let voices_state = voices_state.clone();

        Callback::from(move |_| {
            let channel = (*channel_state).clone();
            if channel.trim().is_empty() {
                status_state.set("请先填写频道".into());
                return;
            }

            let voice_option = (*selected_voice_state).clone();
            if voice_option.is_none() {
                status_state.set("请选择要使用的音色".into());
                return;
            }
            let voice_id = voice_option.unwrap();

            let voices_snapshot = (*voices_state).clone();
            let Some(voice_meta) = voices_snapshot.iter().find(|v| v.id == voice_id) else {
                status_state.set("找不到对应的音色".into());
                return;
            };

            let engine_payload = (*selected_engine_state)
                .clone()
                .and_then(|value| parse_engine_choice(&value))
                .map(|choice| match choice {
                    EngineModelChoice::Tts { .. } => voice_meta.engine.clone(),
                    EngineModelChoice::Shimmy { .. } => voice_meta.engine.clone(),
                });

            if *active_state {
                status_state.set("当前已有频道在播报，先停止后再尝试。".into());
                return;
            }

            active_state.set(true);
            stream_ready_state.set(false);
            status_state.set("正在连接 Twitch 频道...".into());
            active_channel_state.set(None);
            let status_state = status_state.clone();
            let active_state = active_state.clone();
            let log_state = log_state.clone();
            let audio_state = audio_state.clone();
            let active_channel_state_async = active_channel_state.clone();
            let stream_ready_state = stream_ready_state.clone();

            spawn_local(async move {
                let mut payload = serde_json::Map::<String, serde_json::Value>::new();
                payload.insert(
                    "platform".into(),
                    serde_json::Value::String("twitch".into()),
                );
                payload.insert("channel".into(), serde_json::Value::String(channel));
                payload.insert(
                    "voice_id".into(),
                    serde_json::Value::String(voice_id.clone()),
                );
                if let Some(engine) = engine_payload.clone() {
                    payload.insert("engine".into(), serde_json::Value::String(engine));
                }

                match Request::post(&format!("{BACKEND_URL}/api/danmaku/start"))
                    .header("Content-Type", "application/json")
                    .body(serde_json::Value::Object(payload).to_string())
                {
                    Ok(req) => match req.send().await {
                        Ok(resp) => match resp.status() {
                            202 => match resp.json::<DanmakuStartResponse>().await {
                                Ok(data) => {
                                    if let Some(current) = (*audio_state).clone() {
                                        let _ = Url::revoke_object_url(&current);
                                    }
                                    audio_state.set(None);
                                    active_channel_state_async.set(Some(data.channel.clone()));
                                    status_state.set(format!("正在播报: {}", data.channel));
                                    log_state.set(push_log(
                                        (*log_state).clone(),
                                        log_entry(format!("开始监听 {}", data.channel), None),
                                    ));
                                    // 等待 SSE 推送确认后再置为 ready
                                }
                                Err(err) => {
                                    status_state.set(format!("解析启动响应失败: {err}"));
                                    active_state.set(false);
                                    active_channel_state_async.set(None);
                                    stream_ready_state.set(false);
                                }
                            },
                            501 => {
                                status_state.set("后端未启用弹幕播报".into());
                                active_state.set(false);
                                active_channel_state_async.set(None);
                                stream_ready_state.set(false);
                            }
                            status => {
                                let body = resp.text().await.unwrap_or_default();
                                status_state.set(format!("启动失败: {} {}", status, body));
                                active_state.set(false);
                                active_channel_state_async.set(None);
                                stream_ready_state.set(false);
                            }
                        },
                        Err(err) => {
                            status_state.set(format!("请求失败: {err}"));
                            active_state.set(false);
                            active_channel_state_async.set(None);
                            stream_ready_state.set(false);
                        }
                    },
                    Err(err) => {
                        status_state.set(format!("构建请求失败: {err}"));
                        active_state.set(false);
                        active_channel_state_async.set(None);
                        stream_ready_state.set(false);
                    }
                }
            });
        })
    };

    let on_copy_clip = {
        let toast_state = toast_state.clone();
        Callback::from(move |clip: ClipHistoryItem| {
            if let Some(window) = web_sys::window() {
                let navigator = window.navigator();
                let clipboard = navigator.clipboard();
                let text = clip.text.clone();
                let toast_state = toast_state.clone();
                let promise = clipboard.write_text(&text);
                spawn_local(async move {
                    let message = if JsFuture::from(promise).await.is_ok() {
                        ToastMessage::info("文本已复制")
                    } else {
                        ToastMessage::info("复制失败，请手动复制")
                    };
                    toast_state.set(Some(message));
                });
            }
        })
    };

    let detail_clip = (*detail_clip_state).clone();
    let on_close_detail = {
        let detail_clip_state = detail_clip_state.clone();
        Callback::from(move |_| detail_clip_state.set(None))
    };

    let detail_view = detail_clip
        .map(|clip| {
            let download_ext = clip
                .format
                .split('/')
                .nth(1)
                .map(|ext| ext.replace(';', ""))
                .filter(|ext| !ext.is_empty())
                .unwrap_or_else(|| "wav".to_string());
            let download_name = format!(
                "ishowtts-{}-{}-{}.{}",
                clip.engine_label, clip.voice_id, clip.id, download_ext
            );
            let copy_cb = {
                let on_copy_clip = on_copy_clip.clone();
                let clip = clip.clone();
                Callback::from(move |_| on_copy_clip.emit(clip.clone()))
            };
            html! {
                <div class="detail-overlay" onclick={on_close_detail.clone()}>
                    <div class="detail-panel" onclick={Callback::from(|event: MouseEvent| event.stop_propagation())}>
                        <header class="detail-header">
                            <h3>{"记录详情"}</h3>
                            <button class="ghost" onclick={on_close_detail.clone()}>{"关闭"}</button>
                        </header>
                        <div class="detail-body">
                            <div class="detail-meta">
                                <span class="pill source">{clip.source.tag()}</span>
                                <span class="pill">{clip.engine_label.clone()}</span>
                                <span class="pill">{clip.voice_id.clone()}</span>
                            </div>
                            <div class="detail-line">
                                <span class="label">{"时间"}</span>
                                <span>{clip.created_at.clone()}</span>
                            </div>
                            <div class="detail-line">
                                <span class="label">{"采样率"}</span>
                                <span>{format!("{} Hz", clip.sample_rate)}</span>
                            </div>
                            <div class="detail-line">
                                <span class="label">{"音频大小"}</span>
                                <span>{format!("{:.1} KB", clip.waveform_len as f64 / 1024.0)}</span>
                            </div>
                            <div class="detail-text">
                                <span class="label">{"文本"}</span>
                                <p>{clip.text.clone()}</p>
                            </div>
                            <audio controls=true src={clip.audio_src.clone()} preload="auto" />
                        </div>
                        <footer class="detail-footer">
                            <button class="primary" onclick={copy_cb}>{"复制文本"}</button>
                            <a class="ghost" href={clip.audio_src.clone()} download={download_name}>{"下载音频"}</a>
                        </footer>
                    </div>
                </div>
            }
        })
        .unwrap_or(Html::default());

    let on_stop_danmaku = {
        let active_state = danmaku_active_state.clone();
        let status_state = danmaku_status_state.clone();
        let log_state = danmaku_log_state.clone();
        let active_channel_state = danmaku_active_channel_state.clone();
        let audio_state = danmaku_audio_state.clone();
        let stream_ready_state = danmaku_stream_ready_state.clone();
        Callback::from(move |_| {
            if !*active_state {
                status_state.set("当前没有正在播报的频道".into());
                return;
            }

            let current_channel = (*active_channel_state).clone();
            active_state.set(false);
            if let Some(current) = (*audio_state).clone() {
                let _ = Url::revoke_object_url(&current);
            }
            audio_state.set(None);
            stream_ready_state.set(false);

            if let Some(channel) = current_channel.clone() {
                status_state.set(format!("正在停止 {channel}..."));
                let stop_channel = channel.clone();
                let status_state_async = status_state.clone();
                let log_state = log_state.clone();
                let active_channel_state = active_channel_state.clone();
                let active_state_async = active_state.clone();
                let stream_ready_state_async = stream_ready_state.clone();
                spawn_local(async move {
                    let payload = serde_json::json!({
                        "platform": "twitch",
                        "channel": stop_channel.clone(),
                    });
                    let request = Request::post(&format!("{BACKEND_URL}/api/danmaku/stop"))
                        .header("Content-Type", "application/json")
                        .body(payload.to_string());

                    match request {
                        Ok(req) => match req.send().await {
                            Ok(resp) => {
                                let status_code = resp.status();
                                if (200..300).contains(&status_code) {
                                    match resp.json::<DanmakuStopResponse>().await {
                                        Ok(data) => {
                                            active_channel_state.set(None);
                                            status_state_async.set("已停止播报".into());
                                            let display_channel = data
                                                .channel
                                                .filter(|c| !c.is_empty())
                                                .unwrap_or(stop_channel.clone());
                                            log_state.set(push_log(
                                                (*log_state).clone(),
                                                log_entry(
                                                    format!("停止监听 {}", display_channel),
                                                    None,
                                                ),
                                            ));
                                            stream_ready_state_async.set(false);
                                        }
                                        Err(err) => {
                                            status_state_async
                                                .set(format!("解析停止响应失败: {err}"));
                                            active_state_async.set(true);
                                            stream_ready_state_async.set(false);
                                        }
                                    }
                                } else {
                                    let body = resp.text().await.unwrap_or_default();
                                    status_state_async
                                        .set(format!("停止失败: {} {}", status_code, body));
                                    active_state_async.set(true);
                                    stream_ready_state_async.set(false);
                                }
                            }
                            Err(err) => {
                                status_state_async.set(format!("停止请求失败: {err}"));
                                active_state_async.set(true);
                                stream_ready_state_async.set(false);
                            }
                        },
                        Err(err) => {
                            status_state_async.set(format!("构建停止请求失败: {err}"));
                            active_state_async.set(true);
                            stream_ready_state_async.set(false);
                        }
                    }
                });
            } else {
                status_state.set("已停止播报".into());
                active_channel_state.set(None);
                stream_ready_state.set(false);
                log_state.set(push_log((*log_state).clone(), log_entry("停止监听", None)));
            }
        })
    };

    let status_message = status_state.message();
    let status_class = status_state.css_class();
    let history = history_state.entries.clone();
    let history_len = history.len();
    let total_pages = if history_len == 0 {
        1
    } else {
        (history_len + PAGE_SIZE - 1) / PAGE_SIZE
    };
    let current_page_value = (*current_page).min(total_pages - 1);
    let page_start = current_page_value * PAGE_SIZE;
    let page_end = (page_start + PAGE_SIZE).min(history_len);
    let page_entries: Vec<ClipHistoryItem> = history
        .iter()
        .skip(page_start)
        .take(page_end - page_start)
        .cloned()
        .collect();
    let voices = (*voices_state).clone();
    let text_value = (*text_state).clone();
    let text_len = text_value.chars().count();
    let advanced_options = (*advanced_state).clone();
    let advanced_open = *advanced_visible;
    let health_info = (*backend_health_state).clone();
    let health_error = (*health_error_state).clone();
    let danmaku_logs = (*danmaku_log_state).clone();
    let danmaku_active = *danmaku_active_state;
    let danmaku_audio_src = (*danmaku_audio_state).clone();
    let danmaku_status = (*danmaku_status_state).clone();
    let danmaku_stream_ready = *danmaku_stream_ready_state;
    let selected_voice = (*selected_voice_state).clone().unwrap_or_default();
    let shimmy_models = (*shimmy_models_state).clone();
    let mut engine_options: Vec<EngineOption> = Vec::new();
    let mut seen_labels: HashSet<String> = HashSet::new();
    for voice in &voices {
        if seen_labels.insert(voice.engine_label.clone()) {
            let label = voice.engine_label.clone();
            engine_options.push(EngineOption {
                value: format!("tts:{label}"),
                label: label.clone(),
                choice: EngineModelChoice::Tts {
                    engine_label: label,
                },
            });
        }
    }
    for model in &shimmy_models {
        let model_name = model.name.clone();
        engine_options.push(EngineOption {
            value: format!("shimmy:{model_name}"),
            label: format!("Shimmy · {model_name}"),
            choice: EngineModelChoice::Shimmy {
                model_id: model_name,
            },
        });
    }

    let selected_engine_raw = (*selected_engine_state).clone().unwrap_or_default();
    let mut selected_engine_value = selected_engine_raw.clone();
    if selected_engine_value.is_empty()
        || !engine_options
            .iter()
            .any(|option| option.value == selected_engine_value)
    {
        selected_engine_value = engine_options
            .first()
            .map(|option| option.value.clone())
            .unwrap_or_default();
    }
    let selected_engine_option = engine_options
        .iter()
        .find(|option| option.value == selected_engine_value)
        .cloned();
    let selected_engine_choice = selected_engine_option
        .as_ref()
        .map(|option| option.choice.clone());
    let voices_for_engine: Vec<VoiceSummary> = match selected_engine_choice {
        Some(EngineModelChoice::Tts { ref engine_label }) => voices
            .iter()
            .filter(|voice| &voice.engine_label == engine_label)
            .cloned()
            .collect(),
        _ => voices.clone(),
    };
    let voice_ready = !selected_voice.is_empty();
    let engine_options_snapshot = engine_options.clone();

    let voice_reference_detail_view = (*voice_reference_state).clone();
    let voice_reference_error_msg = (*voice_reference_error_state).clone();
    let voice_reference_notice_msg = (*voice_reference_notice_state).clone();
    let voice_reference_loading = *voice_reference_loading_state;
    let voice_reference_text_value = (*voice_reference_text_state).clone();
    let selected_file_label = (*voice_reference_file_state)
        .clone()
        .map(|file| file.name())
        .unwrap_or_else(|| "未选择".into());

    let voice_manager_modal = if *voice_manager_open_state {
        let close_cb = {
            let voice_manager_open_state = voice_manager_open_state.clone();
            Callback::from(move |_| voice_manager_open_state.set(false))
        };

        let modal_body = if voice_reference_loading && voice_reference_detail_view.is_none() {
            html! {
                <div class="modal-card-grid single-column">
                    <section class="modal-card skeleton-card">
                        <p class="muted">{"正在加载音色参考信息..."}</p>
                    </section>
                </div>
            }
        } else if let Some(detail) = voice_reference_detail_view.clone() {
            let baseline_audio_link = if detail.baseline_audio_available {
                Some(format!(
                    "{BACKEND_URL}/api/voices/{}/reference/audio?source=baseline",
                    detail.voice_id
                ))
            } else {
                None
            };
            let override_audio_link = if detail.override_audio_available {
                Some(format!(
                    "{BACKEND_URL}/api/voices/{}/reference/audio?source=override",
                    detail.voice_id
                ))
            } else {
                None
            };
            let override_present = detail.override_audio_available
                || detail
                    .override_reference_text
                    .as_ref()
                    .map(|text| !text.is_empty())
                    .unwrap_or(false);
            let active_text = detail
                .active_reference_text
                .clone()
                .unwrap_or_else(|| "（无）".into());
            let baseline_text = detail
                .baseline_reference_text
                .clone()
                .unwrap_or_else(|| "（无）".into());
            let override_text = detail
                .override_reference_text
                .clone()
                .unwrap_or_else(|| "（未设置）".into());
            let updated_display = detail
                .override_updated_at
                .clone()
                .unwrap_or_else(|| "--".into());

            html! {
                <div class="modal-card-grid">
                    <section class="modal-card summary-card">
                        <header class="modal-card-header">
                            <div>
                                <h4>{"当前参考"}</h4>
                                <p class="muted small">{format!("音色 {}", detail.voice_id)}</p>
                            </div>
                            <span class="badge-soft">{detail.engine_label.clone()}</span>
                        </header>
                        <div class="modal-card-body">
                            <div class="metric-group">
                                <div class="metric-item">
                                    <span class="metric-label">{"当前参考文本"}</span>
                                    <p class="metric-value">{active_text}</p>
                                </div>
                                <div class="metric-item">
                                    <span class="metric-label">{"默认文本"}</span>
                                    <p class="metric-value">{baseline_text}</p>
                                </div>
                                <div class="metric-item">
                                    <span class="metric-label">{"自定义文本"}</span>
                                    <p class="metric-value">{override_text}</p>
                                </div>
                            </div>
                            <div class="pill-group">
                                <span class={classes!("status-chip", if detail.baseline_audio_available { "accent" } else { "muted" })}>
                                    { if detail.baseline_audio_available { "默认音频可用" } else { "默认音频缺失" } }
                                </span>
                                <span class={classes!("status-chip", if detail.override_audio_available { "accent" } else { "muted" })}>
                                    { if detail.override_audio_available { "自定义音频已上传" } else { "暂无自定义音频" } }
                                </span>
                                <span class="status-chip subtle">{format!("更新时间 {updated_display}")}</span>
                            </div>
                        </div>
                        <footer class="modal-card-footer link-footer">
                            {
                                baseline_audio_link.map(|link| html! {
                                    <a class="badge-link" href={link} target="_blank">{"下载默认音频"}</a>
                                }).unwrap_or(Html::default())
                            }
                            {
                                override_audio_link.map(|link| html! {
                                    <a class="badge-link" href={link} target="_blank">{"下载自定义音频"}</a>
                                }).unwrap_or(Html::default())
                            }
                        </footer>
                    </section>
                    <section class="modal-card editor-card">
                        <header class="modal-card-header">
                            <h4>{"更新参考"}</h4>
                            <p class="muted small">{"上传新的语音片段或调整文本"}</p>
                        </header>
                        <div class="modal-card-body form-body">
                            <label class="field">
                                <span>{"参考文本（留空则保持默认）"}</span>
                                <textarea
                                    id="voice-reference-text"
                                    rows={5}
                                    value={voice_reference_text_value.clone()}
                                    oninput={on_reference_text_change.clone()}
                                    disabled={voice_reference_loading}
                                />
                            </label>
                            <div class="field file-field">
                                <span>{"参考音频（可选）"}</span>
                                <label class="file-pill">
                                    <input
                                        id="voice-reference-audio"
                                        type="file"
                                        accept="audio/*"
                                        onchange={on_reference_file_change.clone()}
                                        ref={voice_reference_file_input.clone()}
                                        disabled={voice_reference_loading}
                                    />
                                    <span class="file-label">{"选择音频"}</span>
                                    <span class="file-selected">{selected_file_label.clone()}</span>
                                </label>
                                <button
                                    class="ghost compact"
                                    onclick={on_reference_file_clear.clone()}
                                    disabled={voice_reference_loading}
                                >{"清除选择"}</button>
                            </div>
                        </div>
                        <footer class="modal-card-footer action-footer">
                            <button
                                class="primary"
                                onclick={on_reference_save.clone()}
                                disabled={voice_reference_loading}
                            >{"保存覆盖"}</button>
                            <button
                                class="ghost"
                                onclick={on_reference_reset.clone()}
                                disabled={voice_reference_loading || !override_present}
                            >{"恢复默认"}</button>
                        </footer>
                    </section>
                </div>
            }
        } else {
            let message = voice_reference_error_msg
                .clone()
                .unwrap_or_else(|| "尚未选择音色".into());
            html! {
                <div class="modal-card-grid single-column">
                    <section class="modal-card empty-card">
                        <p class="muted">{message}</p>
                    </section>
                </div>
            }
        };

        html! {
            <div class="modal-backdrop" onclick={close_cb.clone()}>
                <div class="modal modal-floating" onclick={Callback::from(|event: MouseEvent| event.stop_propagation())}>
                    <header class="modal-header">
                        <h3>{"音色设置"}</h3>
                        <button class="ghost" onclick={close_cb.clone()}>{"关闭"}</button>
                    </header>
                    {
                        voice_reference_notice_msg.clone().map(|msg| html! {
                            <p class="notice success">{msg}</p>
                        }).unwrap_or(Html::default())
                    }
                    {
                        voice_reference_error_msg.clone().map(|msg| html! {
                            <p class="notice error">{msg}</p>
                        }).unwrap_or(Html::default())
                    }
                    { modal_body }
                </div>
            </div>
        }
    } else {
        Html::default()
    };

    let has_prev = current_page_value > 0;
    let has_next = (current_page_value + 1) < total_pages && total_pages > 0;
    let page_label = format!("第 {} / {} 页", current_page_value + 1, total_pages);

    let on_prev_page = {
        let current_page = current_page.clone();
        Callback::from(move |_| {
            let page = *current_page;
            if page > 0 {
                current_page.set(page - 1);
            }
        })
    };

    let on_next_page = {
        let current_page = current_page.clone();
        let total_pages = total_pages;
        Callback::from(move |_| {
            let page = *current_page;
            if page + 1 < total_pages {
                current_page.set(page + 1);
            }
        })
    };

    let advanced_section = if advanced_open {
        html! {
            <div class="advanced-panel">
                <div class="fields-grid">
                    <label>
                        {"语速 (speed)"}
                        <input type="number" step="0.01" value={advanced_options.speed.clone()} oninput={speed_input.clone()} placeholder="默认 1.0" />
                    </label>
                    <label>
                        {"目标响度 (target_rms)"}
                        <input type="number" step="0.01" value={advanced_options.target_rms.clone()} oninput={target_rms_input.clone()} placeholder="默认 0.1" />
                    </label>
                    <label>
                        {"交叉渐变 (cross_fade_duration)"}
                        <input type="number" step="0.01" value={advanced_options.cross_fade_duration.clone()} oninput={cross_fade_input.clone()} placeholder="默认 0.15" />
                    </label>
                    <label>
                        {"摇摆采样 (sway_sampling_coef)"}
                        <input type="number" step="0.01" value={advanced_options.sway_sampling_coef.clone()} oninput={sway_input.clone()} placeholder="默认 -1" />
                    </label>
                    <label>
                        {"CFG 强度"}
                        <input type="number" step="0.1" value={advanced_options.cfg_strength.clone()} oninput={cfg_input.clone()} placeholder="默认 2.0" />
                    </label>
                    <label>
                        {"NFE 步数"}
                        <input type="number" value={advanced_options.nfe_step.clone()} oninput={nfe_input.clone()} placeholder="默认 32" />
                    </label>
                    <label>
                        {"固定时长 (秒)"}
                        <input type="number" step="0.05" value={advanced_options.fix_duration.clone()} oninput={fix_duration_input.clone()} placeholder="留空为自动" />
                    </label>
                    <label>
                        {"随机种子"}
                        <input type="number" value={advanced_options.seed.clone()} oninput={seed_input.clone()} placeholder="留空使用随机" />
                    </label>
                </div>
                <label class="toggle">
                    <input type="checkbox" checked={advanced_options.remove_silence} onchange={remove_silence_toggle} />
                    <span>{"移除生成语音中的静音"}</span>
                </label>
                <button class="ghost" onclick={on_reset_advanced.clone()}>{"重置高级参数"}</button>
            </div>
        }
    } else {
        Html::default()
    };

    let danmaku_active_channel = (*danmaku_active_channel_state).clone();

    let toast_render = (*toast_state)
        .clone()
        .map(|toast| {
            html! {
                <div class={classes!("toast", toast.level.class_name())}>
                    { toast.message }
                </div>
            }
        })
        .unwrap_or(Html::default());

    let history_rows: Vec<Html> = page_entries
        .iter()
        .cloned()
        .map(|clip| {
            let timestamp = clip.created_at.clone();
            let summary = clip.text.clone();
            let key = clip.id;
            let detail_cb = {
                let detail_clip_state = detail_clip_state.clone();
                let clip = clip.clone();
                Callback::from(move |_| detail_clip_state.set(Some(clip.clone())))
            };
            html! {
                <div class="history-row" key={key}>
                    <button class="history-entry" type="button" onclick={detail_cb}>
                        <span class="history-time">{timestamp}</span>
                        <span class="history-preview">{summary}</span>
                    </button>
                </div>
            }
        })
        .collect();

    html! {
        <>
        <main class="app-shell">
            <header class="topbar">
                <div class="brand">
                    <div class="logo">{"iShow"}<span class="badge-twitch">{"TTS"}</span></div>
                    <p class="tagline">{"Rust 加速 · Twitch 风格控制台"}</p>
                </div>
                <div class="topbar-controls">
                    <label>
                        <span>{"模型"}</span>
                        <select onchange={on_model_change} value={selected_engine_value.clone()}>
                            {
                                for engine_options.iter().map(|option| {
                                    let value = option.value.clone();
                                    let label = option.label.clone();
                                    html! { <option value={value}>{ label }</option> }
                                })
                            }
                        </select>
                    </label>
                    <label>
                        <span>{"音色"}</span>
                        <select onchange={on_voice_change} value={selected_voice.clone()}>
                            { for voices_for_engine.iter().map(|voice| {
                                let label = match &voice.language {
                                    Some(lang) => format!("{} ({})", voice.id, lang),
                                    None => voice.id.clone(),
                                };
                                html! { <option value={voice.id.clone()}>{ label }</option> }
                            }) }
                        </select>
                    </label>
                    <button class="ghost" onclick={Callback::from({
                        let voice_manager_open_state = voice_manager_open_state.clone();
                        move |_| voice_manager_open_state.set(true)
                    })}>{"音色设置"}</button>
                </div>
                <div class="topbar-status">
                    <span class={classes!("status-pill", if health_info.is_some() { "online" } else { "offline" })}>
                        { if health_info.is_some() { "后端在线" } else { "后端离线" } }
                    </span>
                    {
                        if let Some(health) = health_info.clone() {
                            html! { <span class="status-meta">{format!("默认音色 · {}", health.default_voice)}</span> }
                        } else {
                            html! { <span class="status-meta muted">{"等待健康检查"}</span> }
                        }
                    }
                    {
                        if let Some(channel) = danmaku_active_channel.clone() {
                            html! { <span class="status-pill highlight">{format!("正在播报 {channel}")}</span> }
                        } else {
                            Html::default()
                        }
                    }
                </div>
            </header>

            { voice_manager_modal }

            {
                if let Some(error) = health_error {
                    html! { <div class="alert">{error}</div> }
                } else {
                    Html::default()
                }
            }

            <div class="content-grid">
                <div class="column main-column">
                    <section class="panel stream-panel">
                        <header class="panel-heading">
                            <div>
                                <h2>{"弹幕播报"}</h2>
                                <span class="panel-sub">{"Twitch 聊天 → 实时语音"}</span>
                            </div>
                            <span class="panel-meta">{format!("日志 {}", danmaku_logs.len())}</span>
                        </header>
                        <div class="channel-form">
                            <label class="field">
                                <span>{"频道"}</span>
                                <input
                                    placeholder="例如：twitch.tv/example 或 example"
                                    value={(*danmaku_channel_state).clone()}
                                    oninput={Callback::from({
                                        let channel_state = danmaku_channel_state.clone();
                                        move |event: InputEvent| {
                                            if let Some(input) = event.target_dyn_into::<HtmlInputElement>() {
                                                channel_state.set(input.value());
                                            }
                                        }
                                    })}
                                />
                            </label>
                            <div class="button-row">
                                <button
                                    onclick={on_start_danmaku}
                                    disabled={danmaku_active}
                                    class={classes!("primary", danmaku_stream_ready.then_some("active"))}
                                >
                                    { if danmaku_stream_ready { "正在播报" } else if danmaku_active { "连接中..." } else { "开始播报" } }
                                </button>
                                <button class="ghost" onclick={on_stop_danmaku}>{"停止"}</button>
                            </div>
                        </div>
                        <div class="stream-status">{ danmaku_status }</div>
                        {
                            if let Some(src) = danmaku_audio_src {
                                html! { <audio autoplay=true src={src} /> }
                            } else {
                                Html::default()
                            }
                        }
                        <div class="log-wrapper">
                            { for danmaku_logs.iter().map(|entry| {
                                let timestamp = entry.timestamp.clone();
                                let message = entry.message.clone();
                                let style = entry
                                    .color
                                    .as_ref()
                                    .map(|value| format!("color: {}", value))
                                    .unwrap_or_default();
                                html! {
                                    <div class="log-line">
                                        <span class="timestamp">{timestamp}</span>
                                        <span class="log-message" style={style}>{message}</span>
                                    </div>
                                }
                            }) }
                        </div>
                    </section>
                    <section class="panel history-panel">
                        <header class="panel-heading">
                            <div>
                                <h2>{"生成记录"}</h2>
                            </div>
                            <div class="panel-actions">
                                <span class="panel-meta">{format!("共 {} 条", history_len)}</span>
                                <div class="pager">
                                    <button class="ghost compact" onclick={on_prev_page.clone()} disabled={!has_prev}>{"上一页"}</button>
                                    <span class="panel-meta">{page_label.clone()}</span>
                                    <button class="ghost compact" onclick={on_next_page.clone()} disabled={!has_next}>{"下一页"}</button>
                                </div>
                                <button class="ghost" onclick={on_clear_history}>{"清空"}</button>
                            </div>
                        </header>
                        {
                            if history_len == 0 {
                                html! { <p class="muted">{"暂无历史记录，先合成一段语音或启动弹幕播报吧！"}</p> }
                            } else {
                                html! {
                                    <div class="history-list-wrapper">
                                        <div class="history-virtual-list">
                                            { for history_rows.iter().cloned() }
                                        </div>
                                    </div>
                                }
                            }
                        }
                    </section>
                </div>

                <div class="column side-column">
                    <section class="panel tts-panel">
                        <header class="panel-heading">
                            <div>
                                <h2>{"文本转语音"}</h2>
                                <span class="panel-sub">{"Rust + GPU 加速"}</span>
                            </div>
                            <span class="panel-meta">{format!("字数 {}", text_len)}</span>
                        </header>

                        <label class="field">
                            <span>{"输入文本"}</span>
                            <textarea
                                rows="6"
                                placeholder="输入直播弹幕或任意文本，可按回车换行"
                                value={text_value}
                                oninput={on_text_input}
                            />
                        </label>

                        <div class="button-row">
                            <button onclick={on_submit.clone()} disabled={!voice_ready}>{"立即合成"}</button>
                            <button class={classes!("ghost", advanced_open.then_some("active"))} onclick={on_toggle_advanced.clone()}>
                                { if advanced_open { "隐藏高级参数" } else { "显示高级参数" } }
                            </button>
                        </div>

                        { advanced_section }

                        <div class={classes!("form-status", status_class)}>{ status_message }</div>
                    </section>

                </div>
            </div>
        </main>
        { detail_view }
        { toast_render }
        </>
    }
}

#[derive(Clone, Debug, PartialEq)]
struct ToastMessage {
    level: ToastLevel,
    message: String,
}

impl ToastMessage {
    fn success(msg: impl Into<String>) -> Self {
        Self {
            level: ToastLevel::Success,
            message: msg.into(),
        }
    }

    fn info(msg: impl Into<String>) -> Self {
        Self {
            level: ToastLevel::Info,
            message: msg.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum ToastLevel {
    Success,
    Info,
}

impl ToastLevel {
    const fn class_name(&self) -> &'static str {
        match self {
            ToastLevel::Success => "success",
            ToastLevel::Info => "info",
        }
    }
}

#[wasm_bindgen(start)]
pub fn start_app() {
    yew::Renderer::<App>::new().render();
}
