use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{anyhow, bail, Context, Result};
use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use parking_lot::Mutex;
use rand::{distributions::Alphanumeric, Rng};
use tokio::sync::broadcast;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    task::JoinHandle,
    time::{sleep, Duration},
};
use tracing::{error, info, trace};

use danmaku::message::{NormalizedMessage, Platform};
use danmaku::twitch::{parse_ping, parse_privmsg};
use danmaku_gateway::{
    config::GatewayConfig, filter::FilteredMessage, MessageFilter, MessageQueue,
};
use tts_engine::{EngineKind, TtsRequest};

use crate::synth::Synthesizer;

const TWITCH_IRC_HOST: &str = "irc.chat.twitch.tv";
const TWITCH_IRC_PORT: u16 = 6667;
const SOCKS_PROXY_ENV: &str = "SOCKS5_PROXY";
const ALL_PROXY_ENV: &str = "ALL_PROXY";
const DEFAULT_TTS_NFE_STEP: u32 = 16;

#[derive(Debug, Clone)]
pub struct PlaybackItem {
    pub platform: Platform,
    pub channel: String,
    pub username: String,
    pub display_text: String,
    pub format: String,
    pub sample_rate: u32,
    pub audio: Arc<Vec<u8>>,
    pub color: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct StartRequest {
    pub platform: String,
    pub channel: String,
    #[serde(default)]
    pub voice_id: Option<String>,
    #[serde(default)]
    pub engine: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct StartResponse {
    pub status: String,
    pub channel: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct StopRequest {
    pub platform: String,
    pub channel: String,
}

#[derive(Debug, serde::Serialize)]
pub struct StopResponse {
    pub status: String,
    pub channel: Option<String>,
}

#[derive(Clone, Debug)]
pub struct TwitchAuth {
    pub username: String,
    pub oauth_token: String,
}

#[derive(Clone, Debug)]
struct ChannelSettings {
    voice_id: String,
    engine: EngineKind,
}

#[derive(Clone)]
pub struct DanmakuService {
    queue: Arc<MessageQueue>,
    playback: Arc<Mutex<VecDeque<PlaybackItem>>>,
    watchers: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    synthesizer: Synthesizer,
    default_voice: String,
    twitch_connector: Arc<dyn TwitchConnector>,
    twitch_auth: Option<TwitchAuth>,
    channel_settings: Arc<Mutex<HashMap<String, ChannelSettings>>>,
    playback_notifier: broadcast::Sender<PlaybackItem>,
}

impl DanmakuService {
    pub fn new(
        synthesizer: Synthesizer,
        fallback_voice: String,
        gateway_config: GatewayConfig,
        twitch_auth: Option<TwitchAuth>,
        twitch_connector: Arc<dyn TwitchConnector>,
    ) -> Result<Arc<Self>> {
        let filter = MessageFilter::new(gateway_config.filter.clone())?;
        let (queue_inner, mut rx) = MessageQueue::new(filter, gateway_config.queue.clone());
        let queue = Arc::new(queue_inner);
        let playback = Arc::new(Mutex::new(VecDeque::new()));
        let watchers = Arc::new(Mutex::new(HashMap::new()));
        let selected_voice = gateway_config
            .tts
            .voice_id
            .clone()
            .unwrap_or(fallback_voice);

        let notifier_capacity = gateway_config.queue.capacity.max(64);
        let (playback_notifier, _) = broadcast::channel(notifier_capacity);

        let service = Arc::new(Self {
            queue: queue.clone(),
            playback: playback.clone(),
            watchers,
            synthesizer,
            default_voice: selected_voice,
            twitch_connector,
            twitch_auth,
            channel_settings: Arc::new(Mutex::new(HashMap::new())),
            playback_notifier,
        });

        let worker_service = service.clone();
        tokio::spawn(async move {
            while let Some(filtered) = rx.recv().await {
                if let Err(err) = worker_service.process_filtered(filtered).await {
                    error!(%err, "failed to process danmaku message");
                }
            }
        });

        Ok(service)
    }

    fn resolve_channel_settings(
        &self,
        voice_id: Option<&str>,
        engine: Option<EngineKind>,
    ) -> Result<ChannelSettings> {
        let resolved_voice = voice_id
            .map(|value| value.to_string())
            .unwrap_or_else(|| self.default_voice.clone());
        let descriptor = self
            .synthesizer
            .voice_descriptor(&resolved_voice)
            .ok_or_else(|| anyhow!("音色 '{resolved_voice}' 未配置"))?;

        if let Some(requested_engine) = engine {
            if descriptor.engine != requested_engine {
                bail!(
                    "音色 '{}' 属于引擎 '{}'，与选择的 '{}' 不匹配",
                    resolved_voice,
                    descriptor.engine,
                    requested_engine
                );
            }
        }

        Ok(ChannelSettings {
            voice_id: resolved_voice,
            engine: descriptor.engine,
        })
    }

    pub async fn enqueue(&self, message: &NormalizedMessage) -> Result<bool> {
        self.queue.enqueue(message).await
    }

    pub async fn start_twitch(
        &self,
        user_input: &str,
        voice_id: Option<String>,
        engine: Option<EngineKind>,
    ) -> Result<String> {
        let channel = parse_twitch_channel(user_input)
            .ok_or_else(|| anyhow!("请输入正确的 Twitch 用户名或频道链接"))?;

        {
            let mut watchers = self.watchers.lock();
            if let Some(handle) = watchers.get(&channel) {
                if !handle.is_finished() {
                    bail!("该频道已经在播报中");
                }
                watchers.remove(&channel);
            }
        }

        self.purge_playback_for_channel(&channel);

        let settings = self.resolve_channel_settings(voice_id.as_deref(), engine)?;
        {
            let mut active = self.channel_settings.lock();
            active.insert(channel.clone(), settings.clone());
        }

        let queue = self.queue.clone();
        let handle = match self
            .twitch_connector
            .spawn(channel.clone(), queue, self.twitch_auth.clone())
            .await
            .with_context(|| format!("failed to start twitch watcher for {channel}"))
        {
            Ok(handle) => handle,
            Err(err) => {
                self.channel_settings.lock().remove(&channel);
                return Err(err);
            }
        };

        self.watchers.lock().insert(channel.clone(), handle);
        Ok(channel)
    }

    pub fn stop_twitch(&self, user_input: &str) -> Result<Option<String>> {
        let channel = parse_twitch_channel(user_input)
            .ok_or_else(|| anyhow!("请输入正确的 Twitch 用户名或频道链接"))?;

        let handle_opt = self.watchers.lock().remove(&channel);
        let mut changed = false;
        if let Some(handle) = handle_opt {
            handle.abort();
            changed = true;
        }

        {
            let mut active = self.channel_settings.lock();
            if active.remove(&channel).is_some() {
                changed = true;
            }
        }

        if self.purge_playback_for_channel(&channel) {
            changed = true;
        }

        if changed {
            info!(
                target = "ishowtts::danmaku",
                %channel,
                "stopped twitch channel"
            );
            Ok(Some(channel))
        } else {
            Ok(None)
        }
    }

    async fn process_filtered(&self, filtered: FilteredMessage) -> Result<()> {
        let channel = filtered.source.channel.clone();
        let channel_settings = match self.channel_settings.lock().get(&channel).cloned() {
            Some(settings) => settings,
            None => {
                trace!(
                    target = "ishowtts::danmaku",
                    %channel,
                    "dropping message for inactive channel"
                );
                return Ok(());
            }
        };
        if !self.is_channel_active(&channel) {
            trace!(
                target = "ishowtts::danmaku",
                %channel,
                "dropping message for inactive channel"
            );
            return Ok(());
        }

        let sanitized = filtered.sanitized_text.clone();
        let speaker = filtered.source.username.trim();
        let spoken_text = if speaker.is_empty() {
            sanitized.clone()
        } else {
            format!("{speaker} says: {sanitized}")
        };

        let request = TtsRequest {
            text: spoken_text.clone(),
            voice_id: channel_settings.voice_id.clone(),
            speed: None,
            target_rms: None,
            cross_fade_duration: None,
            sway_sampling_coef: None,
            cfg_strength: None,
            nfe_step: Some(DEFAULT_TTS_NFE_STEP),
            fix_duration: None,
            remove_silence: Some(true),
            seed: None,
        };

        info!(
            target = "ishowtts::danmaku",
            %channel,
            user = %filtered.source.username,
            voice = %channel_settings.voice_id,
            engine = %channel_settings.engine,
            text = %spoken_text,
            "processing danmaku message"
        );

        let started_at = Instant::now();

        let response = self
            .synthesizer
            .synthesize(request)
            .await
            .with_context(|| "TTS synthesis failed for danmaku message")?;

        let response_voice = response.voice_id.clone();
        let response_engine = response.engine;
        let engine_label = response.engine_label.clone();
        if !self.is_channel_active(&channel) {
            trace!(
                target = "ishowtts::danmaku",
                %channel,
                "dropping synthesized audio for inactive channel"
            );
            return Ok(());
        }

        let sample_rate = response.sample_rate;
        let audio_base64 = response.audio_base64;
        let audio_vec = BASE64_STANDARD
            .decode(audio_base64.as_bytes())
            .context("failed to decode synthesized audio from base64")?;
        let audio_bytes = audio_vec.len();
        let audio_kb = ((audio_bytes as f64) / 1024.0 * 10.0).round() / 10.0;

        let item = PlaybackItem {
            platform: filtered.source.platform.clone(),
            channel: filtered.source.channel.clone(),
            username: filtered.source.username.clone(),
            display_text: sanitized,
            format: "audio/wav".into(),
            sample_rate,
            audio: Arc::new(audio_vec),
            color: filtered
                .source
                .metadata
                .get("color")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
        };

        let queue_depth = {
            let mut playback_queue = self.playback.lock();
            playback_queue.push_back(item.clone());
            playback_queue.len()
        };
        info!(
            target = "ishowtts::danmaku",
            %channel,
            user = %filtered.source.username,
            queue_depth,
            "playback enqueued"
        );
        if let Err(err) = self.playback_notifier.send(item.clone()) {
            trace!(
                target = "ishowtts::danmaku",
                %channel,
                ?err,
                "failed to broadcast playback item"
            );
        }
        let elapsed_ms = started_at.elapsed().as_millis();
        info!(
            target = "ishowtts::danmaku",
            %channel,
            user = %filtered.source.username,
            elapsed_ms,
            audio_kb,
            audio_bytes,
            requested_voice = %channel_settings.voice_id,
            requested_engine = %channel_settings.engine,
            resolved_voice = %response_voice,
            resolved_engine = %response_engine,
            engine_label = %engine_label,
            "tts synthesis complete"
        );
        Ok(())
    }
}

impl DanmakuService {
    fn is_channel_active(&self, channel: &str) -> bool {
        self.channel_settings.lock().contains_key(channel)
    }

    fn purge_playback_for_channel(&self, channel: &str) -> bool {
        let mut playback = self.playback.lock();
        let initial_len = playback.len();
        playback.retain(|item| item.channel != channel);
        playback.len() != initial_len
    }

    pub fn subscribe_playback(&self) -> broadcast::Receiver<PlaybackItem> {
        self.playback_notifier.subscribe()
    }

    pub fn pending_playback(&self) -> Vec<PlaybackItem> {
        self.playback.lock().iter().cloned().collect()
    }
}

#[async_trait]
pub trait TwitchConnector: Send + Sync {
    async fn spawn(
        &self,
        channel: String,
        queue: Arc<MessageQueue>,
        auth: Option<TwitchAuth>,
    ) -> Result<JoinHandle<()>>;
}

#[derive(Default)]
pub struct RealTwitchConnector;

#[async_trait]
impl TwitchConnector for RealTwitchConnector {
    async fn spawn(
        &self,
        channel: String,
        queue: Arc<MessageQueue>,
        auth: Option<TwitchAuth>,
    ) -> Result<JoinHandle<()>> {
        Ok(tokio::spawn(async move {
            loop {
                if let Err(err) = twitch_loop(channel.clone(), queue.clone(), auth.clone()).await {
                    error!(%err, "twitch worker error, retrying in 5s");
                    sleep(Duration::from_secs(5)).await;
                } else {
                    break;
                }
            }
        }))
    }
}

async fn twitch_loop(
    channel: String,
    queue: Arc<MessageQueue>,
    auth: Option<TwitchAuth>,
) -> Result<()> {
    info!(%channel, "connecting to twitch chat");
    let mut stream = connect_twitch_irc(auth.as_ref()).await?;

    let nick = auth
        .as_ref()
        .map(|a| a.username.clone())
        .unwrap_or_else(|| {
            format!(
                "justinfan{}",
                rand::thread_rng()
                    .sample_iter(&Alphanumeric)
                    .take(8)
                    .map(char::from)
                    .collect::<String>()
            )
            .to_lowercase()
        });

    let pass_line = auth.as_ref().map_or_else(
        || "PASS SCHMOOPIIE\r\n".to_string(),
        |auth| {
            let token = if auth.oauth_token.starts_with("oauth:") {
                auth.oauth_token.clone()
            } else {
                format!("oauth:{}", auth.oauth_token)
            };
            format!("PASS {}\r\n", token)
        },
    );
    let user_identity = auth
        .as_ref()
        .map(|auth| auth.username.as_str())
        .unwrap_or_else(|| nick.as_str());
    let nick_line = format!("NICK {}\r\n", user_identity);
    let user_line = format!("USER {} 8 * :{}\r\n", user_identity, user_identity);

    stream
        .write_all(pass_line.as_bytes())
        .await
        .context("twitch PASS send failed")?;
    stream
        .write_all(nick_line.as_bytes())
        .await
        .context("twitch NICK send failed")?;
    stream
        .write_all(user_line.as_bytes())
        .await
        .context("twitch USER send failed")?;
    stream
        .write_all(b"CAP REQ :twitch.tv/membership twitch.tv/tags twitch.tv/commands\r\n")
        .await
        .context("twitch CAP send failed")?;
    stream
        .write_all(format!("JOIN #{channel}\r\n").as_bytes())
        .await
        .context("twitch JOIN send failed")?;

    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();
    info!(target = "ishowtts::danmaku", "joined twitch chat stream");

    loop {
        match lines.next_line().await {
            Ok(Some(line)) => {
                trace!(target = "ishowtts::danmaku", %line, "twitch irc line");
                if let Some(token) = parse_ping(&line) {
                    if let Err(err) = writer
                        .write_all(format!("PONG :{}\r\n", token).as_bytes())
                        .await
                    {
                        return Err(anyhow!("failed to send PONG: {err}"));
                    }
                    continue;
                }

                match parse_privmsg(&line) {
                    Ok(Some(chat)) => {
                        let normalized = chat.to_normalized();
                        trace!(
                            target = "ishowtts::danmaku",
                            channel = %normalized.channel,
                            user = %normalized.username,
                            text = %chat.message,
                            "received twitch chat"
                        );
                        if !queue.enqueue(&normalized).await.unwrap_or(false) {
                            trace!(
                                target = "ishowtts::danmaku",
                                channel = %normalized.channel,
                                user = %normalized.username,
                                "message dropped by queue"
                            );
                        }
                    }
                    Ok(None) => {}
                    Err(err) => {
                        error!(%err, "failed to parse twitch message");
                    }
                }
            }
            Ok(None) => {
                info!(target = "ishowtts::danmaku", "twitch IRC closed connection");
                return Err(anyhow!("twitch chat stream ended unexpectedly"));
            }
            Err(err) => {
                return Err(anyhow!("error reading from twitch IRC: {err}"));
            }
        }
    }
}

fn parse_twitch_channel(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_lowercase();
    let after = if let Some(idx) = lower.find("twitch.tv/") {
        let rest = &trimmed[idx + "twitch.tv/".len()..];
        rest.split(|c: char| c == '/' || c == '?' || c == '&')
            .next()
            .unwrap_or("")
    } else {
        trimmed
    };
    let channel = after.trim_matches('/');
    if channel.is_empty() {
        None
    } else {
        Some(channel.to_lowercase())
    }
}

async fn connect_twitch_irc(auth: Option<&TwitchAuth>) -> Result<TcpStream> {
    if let Some((proxy_host, proxy_port)) = socks_proxy_from_env() {
        info!(
            target = "ishowtts::danmaku",
            proxy = %format!("{}:{}", proxy_host, proxy_port),
            "connecting to twitch via socks proxy"
        );
        connect_via_socks(proxy_host.as_str(), proxy_port, auth).await
    } else {
        info!(
            target = "ishowtts::danmaku",
            "attempting direct twitch IRC connect"
        );
        let stream = TcpStream::connect((TWITCH_IRC_HOST, TWITCH_IRC_PORT))
            .await
            .context("failed to connect to twitch IRC")?;
        info!(
            target = "ishowtts::danmaku",
            "connected to twitch IRC directly"
        );
        Ok(stream)
    }
}

fn socks_proxy_from_env() -> Option<(String, u16)> {
    let raw = std::env::var(SOCKS_PROXY_ENV)
        .or_else(|_| std::env::var(ALL_PROXY_ENV))
        .ok()?;

    parse_proxy_addr(&raw)
}

fn parse_proxy_addr(raw: &str) -> Option<(String, u16)> {
    let trimmed = raw.trim();
    let without_scheme = if let Some(idx) = trimmed.find("://") {
        let (scheme, rest) = trimmed.split_at(idx);
        if !scheme.eq_ignore_ascii_case("socks5") {
            return None;
        }
        &rest[3..]
    } else {
        trimmed
    };

    let mut parts = without_scheme.splitn(2, ':');
    let host = parts.next()?.trim().to_string();
    let port = parts.next()?.trim().parse().ok()?;
    Some((host, port))
}

async fn connect_via_socks(
    proxy_host: &str,
    proxy_port: u16,
    _auth: Option<&TwitchAuth>,
) -> Result<TcpStream> {
    let mut stream = TcpStream::connect((proxy_host, proxy_port))
        .await
        .with_context(|| format!("failed to connect to socks proxy {proxy_host}:{proxy_port}"))?;

    // greeting: SOCKS5, 1 auth method, no auth
    stream.write_all(&[0x05, 0x01, 0x00]).await?;
    let mut greeting = [0u8; 2];
    stream.read_exact(&mut greeting).await?;
    if greeting != [0x05, 0x00] {
        bail!("socks proxy does not support no-auth authentication");
    }

    let host_bytes = TWITCH_IRC_HOST.as_bytes();
    let mut request = Vec::with_capacity(4 + host_bytes.len() + 2);
    request.push(0x05); // version
    request.push(0x01); // connect
    request.push(0x00); // reserved
    request.push(0x03); // domain name
    request.push(host_bytes.len() as u8);
    request.extend_from_slice(host_bytes);
    request.push((TWITCH_IRC_PORT >> 8) as u8);
    request.push((TWITCH_IRC_PORT & 0xff) as u8);

    stream.write_all(&request).await?;

    let mut response_head = [0u8; 4];
    stream.read_exact(&mut response_head).await?;
    if response_head[1] != 0x00 {
        bail!(
            "socks proxy connect request rejected (code {})",
            response_head[1]
        );
    }

    let addr_type = response_head[3];
    match addr_type {
        0x01 => {
            let mut buf = [0u8; 4];
            stream.read_exact(&mut buf).await?;
        }
        0x03 => {
            let mut len_buf = [0u8; 1];
            stream.read_exact(&mut len_buf).await?;
            let mut buf = vec![0u8; len_buf[0] as usize];
            stream.read_exact(&mut buf).await?;
        }
        0x04 => {
            let mut buf = [0u8; 16];
            stream.read_exact(&mut buf).await?;
        }
        other => bail!("unexpected addr type {other} in socks response"),
    }

    let mut port_buf = [0u8; 2];
    stream.read_exact(&mut port_buf).await?;

    info!(
        target = "ishowtts::danmaku",
        proxy = %format!("{}:{}", proxy_host, proxy_port),
        "connected to twitch IRC via socks proxy"
    );

    Ok(stream)
}
