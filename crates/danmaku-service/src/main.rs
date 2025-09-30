use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use parking_lot::Mutex;
use rand::{distributions::Alphanumeric, Rng};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    task::JoinHandle,
    time::{sleep, Duration},
};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use danmaku::config::DanmakuConfig;
use danmaku::message::{NormalizedMessage, Platform, Priority};
use danmaku::twitch::{parse_ping, parse_privmsg};
use danmaku_gateway::{
    config::GatewayConfig, FilteredMessage, MessageFilter, MessageQueue, TtsClient,
};

#[derive(Clone)]
struct AppState {
    queue: Arc<MessageQueue>,
    playback: Arc<Mutex<VecDeque<PlaybackItem>>>,
    tts: TtsClient,
    watchers: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    twitch_connector: Arc<dyn TwitchConnector>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct PlaybackItem {
    message: NormalizedMessage,
    audio_base64: String,
    format: String,
    sample_rate: u32,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct StartRequest {
    platform: String,
    channel: String,
}

#[derive(Debug, serde::Serialize)]
struct StartResponse {
    status: String,
    channel: String,
}

#[async_trait]
trait TwitchConnector: Send + Sync {
    async fn spawn(&self, channel: String, queue: Arc<MessageQueue>) -> Result<JoinHandle<()>>;
}

#[derive(Default)]
struct RealTwitchConnector;

const TWITCH_IRC_ADDR: &str = "irc.chat.twitch.tv:6667";

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let gateway_config =
        GatewayConfig::load_from_file("config/danmaku_gateway.toml").unwrap_or_default();
    let danmaku_config = DanmakuConfig::load_from_file("config/danmaku.toml").unwrap_or_default();
    info!(queue = ?gateway_config.queue, filter = ?gateway_config.filter, "loaded configs");

    let (state, background_handle) =
        build_app_state_with_connector(gateway_config, Arc::new(RealTwitchConnector::default()))
            .await?;

    if let Some(twitch) = danmaku_config.twitch {
        if twitch.enabled {
            info!(channels = ?twitch.channels, "ready to start Twitch collectors via UI");
        }
    }
    if let Some(youtube) = danmaku_config.youtube {
        if youtube.enabled {
            info!(channel = ?youtube.channel_id, "YouTube support coming soon");
        }
    }

    let app = build_router(state.clone());

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 28080));
    info!(%addr, "starting http server");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    let server = axum::serve(listener, app);

    tokio::select! {
        res = server => {
            if let Err(err) = res {
                error!(%err, "server error");
            }
        }
        res = background_handle => {
            if let Err(err) = res {
                error!(%err, "background worker error");
            }
        }
    }

    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}

fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(index_handler))
        .route("/api/start", post(start_handler))
        .route("/api/enqueue", post(enqueue_handler))
        .route("/api/next", get(next_handler))
        .with_state(state)
}

async fn index_handler() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn start_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<StartRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    match payload.platform.to_lowercase().as_str() {
        "twitch" => start_twitch(state, payload.channel).await,
        "youtube" => Err((
            StatusCode::NOT_IMPLEMENTED,
            "YouTube live chat support is coming soon.".into(),
        )),
        other => Err((
            StatusCode::BAD_REQUEST,
            format!("unsupported platform '{other}'"),
        )),
    }
}

async fn start_twitch(
    state: Arc<AppState>,
    user_input: String,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let channel = parse_twitch_channel(&user_input).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "请输入正确的 Twitch 用户名或频道链接".into(),
        )
    })?;

    {
        let mut watchers = state.watchers.lock();
        if let Some(handle) = watchers.get(&channel) {
            if !handle.is_finished() {
                return Err((StatusCode::CONFLICT, "该频道已经在播报中".into()));
            }
            watchers.remove(&channel);
        }
    }

    let queue = state.queue.clone();
    let connector = state.twitch_connector.clone();
    let handle = connector
        .spawn(channel.clone(), queue)
        .await
        .map_err(|err| (StatusCode::BAD_GATEWAY, err.to_string()))?;

    state.watchers.lock().insert(channel.clone(), handle);

    Ok((
        StatusCode::ACCEPTED,
        Json(StartResponse {
            status: "started".into(),
            channel,
        }),
    ))
}

async fn enqueue_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<NormalizedMessage>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let accepted = state
        .queue
        .enqueue(&payload)
        .await
        .map_err(|err| (StatusCode::BAD_GATEWAY, err.to_string()))?;
    if accepted {
        Ok(StatusCode::ACCEPTED)
    } else {
        Ok(StatusCode::NO_CONTENT)
    }
}

async fn next_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut playback = state.playback.lock();
    if let Some(item) = playback.pop_front() {
        Json(item).into_response()
    } else {
        StatusCode::NO_CONTENT.into_response()
    }
}

async fn build_app_state(config: GatewayConfig) -> Result<(Arc<AppState>, JoinHandle<Result<()>>)> {
    build_app_state_with_connector(config, Arc::new(RealTwitchConnector::default())).await
}

async fn build_app_state_with_connector(
    config: GatewayConfig,
    twitch_connector: Arc<dyn TwitchConnector>,
) -> Result<(Arc<AppState>, JoinHandle<Result<()>>)> {
    let filter = MessageFilter::new(config.filter.clone())?;
    let (queue_inner, mut rx) = MessageQueue::new(filter, config.queue.clone());
    let queue = Arc::new(queue_inner);
    let playback = Arc::new(Mutex::new(VecDeque::new()));
    let tts_client = TtsClient::new(config.tts.clone())?;
    let state = Arc::new(AppState {
        queue: queue.clone(),
        playback: playback.clone(),
        tts: tts_client.clone(),
        watchers: Arc::new(Mutex::new(HashMap::new())),
        twitch_connector,
    });

    let worker_state = state.clone();
    let handle = tokio::spawn(async move {
        while let Some(filtered) = rx.recv().await {
            if let Err(err) = process_message(&worker_state, filtered).await {
                error!(%err, "failed to process message");
            }
        }
        Ok(())
    });

    Ok((state, handle))
}

async fn process_message(state: &Arc<AppState>, filtered: FilteredMessage) -> Result<()> {
    let tts_response = state.tts.synthesize(&filtered.sanitized_text).await?;
    let item = PlaybackItem {
        message: filtered.source,
        audio_base64: tts_response.audio_base64,
        format: tts_response.format,
        sample_rate: tts_response.sample_rate,
    };
    let mut playback = state.playback.lock();
    playback.push_back(item);
    Ok(())
}

fn parse_twitch_channel(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_lowercase();
    let channel = if let Some(idx) = lower.find("twitch.tv/") {
        let after = &trimmed[idx + "twitch.tv/".len()..];
        after
            .split(|c: char| c == '/' || c == '?' || c == '&')
            .next()
            .unwrap_or("")
    } else {
        trimmed
    };
    let channel = channel.trim_matches('/');
    if channel.is_empty() {
        None
    } else {
        Some(channel.to_lowercase())
    }
}

#[async_trait]
impl TwitchConnector for RealTwitchConnector {
    async fn spawn(&self, channel: String, queue: Arc<MessageQueue>) -> Result<JoinHandle<()>> {
        let handle = tokio::spawn(async move {
            loop {
                match twitch_loop(channel.clone(), queue.clone()).await {
                    Ok(_) => break,
                    Err(err) => {
                        error!(%err, "twitch worker error, retrying in 5s");
                        sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        });
        Ok(handle)
    }
}

async fn twitch_loop(channel: String, queue: Arc<MessageQueue>) -> Result<()> {
    let mut stream = TcpStream::connect(TWITCH_IRC_ADDR)
        .await
        .with_context(|| "failed to connect to twitch IRC")?;

    let nick: String = format!(
        "justinfan{}",
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(8)
            .map(char::from)
            .collect::<String>()
    )
    .to_lowercase();

    stream
        .write_all(b"PASS SCHMOOPIIE\r\n")
        .await
        .context("twitch PASS send failed")?;
    stream
        .write_all(format!("NICK {nick}\r\n").as_bytes())
        .await
        .context("twitch NICK send failed")?;
    stream
        .write_all(b"CAP REQ :twitch.tv/tags twitch.tv/commands\r\n")
        .await
        .context("twitch CAP send failed")?;
    stream
        .write_all(format!("JOIN #{channel}\r\n").as_bytes())
        .await
        .context("twitch JOIN send failed")?;

    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    while let Some(line) = lines.next_line().await? {
        if let Some(token) = parse_ping(&line) {
            writer
                .write_all(format!("PONG :{}\r\n", token).as_bytes())
                .await
                .ok();
            continue;
        }
        match parse_privmsg(&line) {
            Ok(Some(chat)) => {
                let normalized = chat.to_normalized();
                let _ = queue.enqueue(&normalized).await;
            }
            Ok(None) => {}
            Err(err) => {
                error!(%err, "failed to parse twitch message");
            }
        }
    }

    Ok(())
}

const INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8" />
  <title>主播弹幕语音播报</title>
  <style>
    body { font-family: Arial, sans-serif; margin: 40px; }
    .panel { max-width: 480px; margin-bottom: 24px; }
    label { display: block; margin-top: 12px; }
    button { margin-top: 16px; padding: 8px 16px; }
    #log { border: 1px solid #ccc; padding: 12px; max-width: 480px; height: 200px; overflow-y: auto; }
  </style>
</head>
<body>
  <h1>主播弹幕语音播报</h1>
  <div class="panel">
    <label>平台：</label>
    <label><input type="radio" name="platform" value="twitch" checked /> Twitch</label>
    <label><input type="radio" name="platform" value="youtube" disabled /> YouTube（即将支持）</label>

    <label for="channel">填写 Twitch 用户名或频道链接：</label>
    <input id="channel" type="text" placeholder="例如：twitch.tv/example 或 example" style="width: 100%; padding: 8px;" />

    <button id="start-btn">开始播放弹幕语音</button>
  </div>

  <h2>播报记录</h2>
  <div id="log"></div>

  <audio id="player" style="display:none;"></audio>

  <script>
    const logEl = document.getElementById('log');
    const player = document.getElementById('player');

    function appendLog(text) {
      const row = document.createElement('div');
      row.textContent = `[${new Date().toLocaleTimeString()}] ${text}`;
      logEl.appendChild(row);
      logEl.scrollTop = logEl.scrollHeight;
    }

    document.getElementById('start-btn').addEventListener('click', async () => {
      const channel = document.getElementById('channel').value.trim();
      const platform = document.querySelector('input[name="platform"]:checked').value;
      if (!channel) {
        appendLog('请先填写频道信息');
        return;
      }
      try {
        const resp = await fetch('/api/start', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ platform, channel })
        });
        if (resp.status === 202) {
          const data = await resp.json();
          appendLog(`已开始监听 ${data.channel} 的弹幕`);
        } else {
          const err = await resp.text();
          appendLog(`启动失败：${err}`);
        }
      } catch (err) {
        appendLog(`请求失败：${err}`);
      }
    });

    async function pollNext() {
      try {
        const resp = await fetch('/api/next');
        if (resp.status === 200) {
          const data = await resp.json();
          appendLog(`${data.message.username}: ${data.message.content.Text}`);
          player.src = `data:${data.format};base64,${data.audio_base64}`;
          player.play().catch(() => appendLog('浏览器拦截了自动播放，请点击页面任意位置启用声音。'));
        }
      } catch (err) {
        appendLog(`轮询出错：${err}`);
      } finally {
        setTimeout(pollNext, 1500);
      }
    }

    pollNext();
  </script>
</body>
</html>"#;

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode as HyperStatus};
    use httpmock::{Method::POST, MockServer};
    use tokio::time::Duration;
    use tower::ServiceExt;
    use uuid::Uuid;

    struct MockTwitchConnector;

    #[async_trait]
    impl TwitchConnector for MockTwitchConnector {
        async fn spawn(&self, channel: String, queue: Arc<MessageQueue>) -> Result<JoinHandle<()>> {
            Ok(tokio::spawn(async move {
                sleep(Duration::from_millis(20)).await;
                let message = NormalizedMessage::new_text(
                    Platform::Twitch,
                    channel,
                    Some("mock".into()),
                    "mock_user",
                    Priority::Normal,
                    "mock message",
                    serde_json::Value::Null,
                );
                let _ = queue.enqueue(&message).await;
            }))
        }
    }

    #[tokio::test]
    async fn enqueue_then_fetch() {
        let server = MockServer::start();
        let response = serde_json::json!({
            "request_id": Uuid::new_v4(),
            "voice_id": "walter",
            "sample_rate": 24000,
            "audio_base64": "UklGRg==",
            "format": "audio/wav",
            "waveform_len": 10
        });
        let mock = server.mock(|when, then| {
            when.method(POST).path("/api/tts");
            then.status(200).json_body(response.clone());
        });

        let gateway_config = GatewayConfig {
            queue: danmaku_gateway::QueueConfig {
                capacity: 16,
                rate_limit_per_sec: 100.0,
            },
            filter: danmaku_gateway::FilterConfig {
                max_words: 10,
                max_chars: 200,
                banned_keywords: vec![],
                allow_links: true,
            },
            tts: danmaku_gateway::TtsConfig {
                endpoint: format!("{}/api/tts", server.base_url()),
                voice_id: Some("walter".into()),
                timeout_secs: Some(5),
            },
        };
        let (state, worker) =
            build_app_state_with_connector(gateway_config, Arc::new(MockTwitchConnector))
                .await
                .unwrap();
        let app = build_router(state.clone());
        let message = NormalizedMessage::new_text(
            Platform::Twitch,
            "channel",
            Some("uid".into()),
            "user",
            Priority::Normal,
            "hello world",
            serde_json::Value::Null,
        );
        let request = Request::builder()
            .method("POST")
            .uri("/api/enqueue")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&message).unwrap()))
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), HyperStatus::ACCEPTED);

        sleep(Duration::from_millis(50)).await;

        let request = Request::builder()
            .method("GET")
            .uri("/api/next")
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), HyperStatus::OK);
        mock.assert();

        worker.abort();
        let _ = worker.await;
    }

    #[tokio::test]
    async fn start_endpoint_uses_mock_connector() {
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(POST).path("/api/tts");
            then.status(200).json_body(serde_json::json!({
                "request_id": Uuid::new_v4(),
                "voice_id": "walter",
                "sample_rate": 24000,
                "audio_base64": "UklGRg==",
                "format": "audio/wav",
                "waveform_len": 10
            }));
        });

        let gateway_config = GatewayConfig {
            queue: danmaku_gateway::QueueConfig::default(),
            filter: danmaku_gateway::FilterConfig::default(),
            tts: danmaku_gateway::TtsConfig {
                endpoint: format!("{}/api/tts", server.base_url()),
                voice_id: Some("walter".into()),
                timeout_secs: Some(5),
            },
        };
        let (state, worker) =
            build_app_state_with_connector(gateway_config, Arc::new(MockTwitchConnector))
                .await
                .unwrap();
        let app = build_router(state.clone());

        let request = Request::builder()
            .method("POST")
            .uri("/api/start")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&StartRequest {
                    platform: "twitch".into(),
                    channel: "https://www.twitch.tv/test_channel".into(),
                })
                .unwrap(),
            ))
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), HyperStatus::ACCEPTED);

        sleep(Duration::from_millis(80)).await;

        let request = Request::builder()
            .method("GET")
            .uri("/api/next")
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), HyperStatus::OK);

        worker.abort();
        let _ = worker.await;
    }
}
