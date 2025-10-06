mod config;
mod danmaku;
mod routes;
mod shimmy_integration;
mod synth;
mod voice_overrides;

use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Instant};

use anyhow::{anyhow, Context, Result};
use axum::Router;
use clap::Parser;
use routes::{build_api_router, build_openai_router, build_shimmy_router, ApiState};
use shimmy::AppState as ShimmyAppState;
use shimmy_integration::F5ShimmyEngine;
use synth::Synthesizer;
use tokio::signal;
use tower_http::trace::{
    DefaultMakeSpan, DefaultOnFailure, DefaultOnRequest, DefaultOnResponse, TraceLayer,
};
use tracing::{error, info, warn, Level};
use tracing_subscriber::{fmt, EnvFilter};
use tts_engine::{
    EngineKind, F5Engine, IndexTtsEngine, IndexTtsVllmEngine, TtsEngine, VoiceOverrideUpdate,
};
use voice_overrides::VoiceOverrideStore;

use crate::{
    config::AppConfig,
    danmaku::{DanmakuService, RealTwitchConnector, TwitchAuth},
};
use ::danmaku::TwitchConfig;

#[derive(Debug, Parser)]
#[command(
    author,
    version,
    about = "iShowTTS backend service powered by Shimmy + F5-TTS"
)]
struct Cli {
    /// Path to configuration file
    #[arg(long)]
    config: PathBuf,
    /// Logging level (error|warn|info|debug|trace)
    #[arg(long, default_value = "info")]
    log_level: String,
    /// Warm up frequently used voices during startup
    #[arg(long, default_value_t = false)]
    warmup: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(&cli.log_level)?;

    let (config, _config_dir) = AppConfig::load(cli.config.clone())?;
    anyhow::ensure!(
        !config.f5.voices.is_empty(),
        "configuration must declare at least one F5 voice profile"
    );

    let warmup_targets: Vec<(String, EngineKind)> = {
        let mut targets = Vec::new();
        for profile in &config.f5.voices {
            if profile.preload {
                targets.push((profile.id.clone(), EngineKind::F5));
            }
        }
        if let Some(index_cfg) = config.index_tts.as_ref() {
            for profile in &index_cfg.voices {
                if profile.preload {
                    targets.push((profile.id.clone(), EngineKind::IndexTts));
                }
            }
        }
        if let Some(vllm_cfg) = config.index_tts_vllm.as_ref() {
            for profile in &vllm_cfg.voices {
                if profile.preload {
                    targets.push((profile.id.clone(), EngineKind::IndexTtsVllm));
                }
            }
        }
        targets
    };

    let f5_engine = Arc::new(F5Engine::new(config.f5.clone())?);
    let mut engines: Vec<Arc<dyn TtsEngine>> = Vec::new();
    let f5_dyn: Arc<dyn TtsEngine> = f5_engine.clone();
    engines.push(f5_dyn);

    if let Some(index_cfg) = config.index_tts.clone() {
        let index_engine: Arc<dyn TtsEngine> = Arc::new(IndexTtsEngine::new(index_cfg)?);
        engines.push(index_engine);
    }

    if let Some(vllm_cfg) = config.index_tts_vllm.clone() {
        let vllm_engine: Arc<dyn TtsEngine> = Arc::new(IndexTtsVllmEngine::new(vllm_cfg)?);
        engines.push(vllm_engine);
    }

    let synthesizer = Arc::new(Synthesizer::new(engines, config.api.max_parallel)?);
    let voice_summaries_vec = synthesizer.voices();
    anyhow::ensure!(
        !voice_summaries_vec.is_empty(),
        "no voice profiles available after engine initialisation"
    );

    if cli.warmup {
        run_warmup(&synthesizer, &warmup_targets).await;
    }

    let overrides_store = Arc::new(VoiceOverrideStore::load("data/voices/overrides")?);
    apply_existing_overrides(&synthesizer, &overrides_store)?;

    let default_voice = match config.default_voice.clone() {
        Some(candidate) => {
            if voice_summaries_vec.iter().any(|v| v.id == candidate) {
                candidate
            } else {
                let fallback = voice_summaries_vec.first().unwrap().id.clone();
                warn!(
                    target = "ishowtts::backend",
                    configured = %candidate,
                    fallback = %fallback,
                    "configured default voice not found; falling back"
                );
                fallback
            }
        }
        None => voice_summaries_vec.first().unwrap().id.clone(),
    };

    let shimmy_engine = F5ShimmyEngine::new(synthesizer.clone());

    let mut registry = shimmy::model_registry::Registry::new();
    for entry in config.shimmy_entries() {
        registry.register(entry);
    }
    let shimmy_state = Arc::new(ShimmyAppState {
        engine: Box::new(shimmy_engine),
        registry,
    });

    let danmaku_gateway_cfg = config.danmaku_gateway.clone().unwrap_or_default();
    let twitch_auth = config
        .danmaku
        .as_ref()
        .and_then(|cfg| cfg.twitch.clone())
        .and_then(|tw_cfg| build_twitch_auth(&tw_cfg));
    let danmaku_service = match DanmakuService::new(
        (*synthesizer).clone(),
        default_voice.clone(),
        danmaku_gateway_cfg,
        twitch_auth,
        Arc::new(RealTwitchConnector::default()),
    ) {
        Ok(service) => Some(service),
        Err(err) => {
            error!(target = "ishowtts::backend", %err, "failed to initialise danmaku service");
            None
        }
    };

    if let Some(danmaku_cfg) = config.danmaku.clone() {
        if let Some(twitch_cfg) = danmaku_cfg.twitch {
            if twitch_cfg.enabled && !twitch_cfg.channels.is_empty() {
                info!(channels = ?twitch_cfg.channels, "danmaku configured for twitch channels");
            }
        }
    }

    let api_state = ApiState {
        synthesizer: synthesizer.clone(),
        default_voice: default_voice.clone(),
        danmaku: danmaku_service,
        voice_overrides: overrides_store.clone(),
    };

    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
        .on_request(DefaultOnRequest::new().level(Level::INFO))
        .on_response(DefaultOnResponse::new().level(Level::INFO))
        .on_failure(DefaultOnFailure::new().level(Level::WARN));

    let app = Router::new()
        .nest("/api", build_api_router(api_state))
        .nest("/shimmy", build_shimmy_router(shimmy_state.clone()))
        .nest("/v1", build_openai_router(shimmy_state.clone()))
        .layer(trace_layer);

    let addr: SocketAddr = config
        .bind_addr
        .parse()
        .context("bind_addr must be in host:port format")?;
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind to {}", addr))?;

    info!(target = "ishowtts::backend", %addr, "backend ready");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!(target = "ishowtts::backend", "shutdown complete");
    Ok(())
}

fn init_tracing(level: &str) -> Result<()> {
    let filter = EnvFilter::try_new(level)
        .or_else(|_| EnvFilter::try_new(format!("ishowtts={level}")))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_max_level(Level::INFO)
        .compact()
        .try_init()
        .map_err(|err| anyhow!("failed to initialise tracing subscriber: {err}"))?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};
        let mut term = signal(SignalKind::terminate()).expect("failed to install signal handler");
        term.recv().await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    info!(target = "ishowtts::backend", "shutdown signal received");
}

async fn run_warmup(synth: &Arc<Synthesizer>, targets: &[(String, EngineKind)]) {
    if targets.is_empty() {
        info!(
            target = "ishowtts::backend",
            "warmup skipped (no voices marked preload)"
        );
        return;
    }

    info!(
        target = "ishowtts::backend",
        "starting warmup for {} voices",
        targets.len()
    );
    for (voice_id, engine) in targets {
        let started = Instant::now();
        match synth.warmup_voice(voice_id, "Warmup sample").await {
            Ok(_) => {
                info!(
                    target = "ishowtts::backend",
                    voice = %voice_id,
                    engine = %engine,
                    elapsed_ms = started.elapsed().as_millis(),
                    "warmup completed"
                );
            }
            Err(err) => {
                warn!(
                    target = "ishowtts::backend",
                    voice = %voice_id,
                    engine = %engine,
                    %err,
                    "warmup failed"
                );
            }
        }
    }
}

fn apply_existing_overrides(synth: &Arc<Synthesizer>, store: &VoiceOverrideStore) -> Result<()> {
    for record in store.all() {
        let update = VoiceOverrideUpdate {
            reference_audio: record.reference_audio.clone(),
            reference_text: record.reference_text.clone(),
        };
        if let Err(err) = synth.apply_override(record.engine, &record.voice_id, update) {
            warn!(
                target = "ishowtts::backend",
                voice = %record.voice_id,
                engine = %record.engine,
                %err,
                "failed to apply voice override on startup"
            );
        }
    }
    Ok(())
}

fn build_twitch_auth(cfg: &TwitchConfig) -> Option<TwitchAuth> {
    let username = cfg.bot_username.as_ref()?.trim();
    let raw_token = cfg.oauth_token.as_ref()?.trim();
    let token = normalize_twitch_token(raw_token)?;
    if username.is_empty() || token.is_empty() {
        return None;
    }
    Some(TwitchAuth {
        username: username.to_lowercase(),
        oauth_token: token,
    })
}

fn normalize_twitch_token(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Allow pasting the entire callback fragment or prefixed variants.
    let without_hash = trimmed.trim_start_matches('#');
    let mut candidate = without_hash;

    if let Some(pos) = without_hash.find("access_token=") {
        let rest = &without_hash[pos + "access_token=".len()..];
        let end = rest.find('&').unwrap_or(rest.len());
        candidate = &rest[..end];
    }

    let candidate = candidate.trim();
    if candidate.is_empty() {
        return None;
    }

    let candidate = candidate.strip_prefix("oauth:").unwrap_or(candidate);
    let candidate = candidate
        .strip_prefix("Bearer ")
        .or_else(|| candidate.strip_prefix("bearer "))
        .unwrap_or(candidate);

    let token = candidate.trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}
