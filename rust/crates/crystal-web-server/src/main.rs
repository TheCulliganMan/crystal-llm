use std::collections::HashMap;
use std::env;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, Request, State};
use axum::http::header::CACHE_CONTROL;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use crystal_web_server::hub::{
    ClientMessage, Delivery, Hub, MAX_RELAY_BYTES, ModpackIdentity, PROTOCOL_VERSION, ServerMessage,
};
use futures_util::{SinkExt, StreamExt};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{Mutex, Notify, OwnedSemaphorePermit, Semaphore, mpsc, watch};
use tokio::time::{Instant, timeout};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing::{info, warn};
use uuid::Uuid;

const DEFAULT_PORT: u16 = 8080;
const OUTBOUND_CAPACITY: usize = 256;
const MAX_MESSAGES_PER_SECOND: u32 = 120;
const WRITE_TIMEOUT: Duration = Duration::from_secs(2);
#[cfg(not(test))]
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);
#[cfg(test)]
const HEARTBEAT_INTERVAL: Duration = Duration::from_millis(50);
#[cfg(not(test))]
const IDLE_TIMEOUT: Duration = Duration::from_secs(45);
#[cfg(test)]
const IDLE_TIMEOUT: Duration = Duration::from_millis(200);

#[derive(Debug, Clone)]
struct Config {
    root: PathBuf,
    pack_dir: PathBuf,
    data_dir: PathBuf,
    host: IpAddr,
    port: u16,
    auth_token: Option<String>,
    auth_secret: Option<String>,
    allowed_modpacks: HashMap<String, String>,
    max_clients: usize,
}

impl Config {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self> {
        let mut root =
            PathBuf::from(env::var("CRYSTAL_WEB_ROOT").unwrap_or_else(|_| "web-dist".into()));
        let mut pack_dir =
            PathBuf::from(env::var("CRYSTAL_PACK_DIR").unwrap_or_else(|_| "packs".into()));
        let data_dir =
            PathBuf::from(env::var("CRYSTAL_DATA_DIR").unwrap_or_else(|_| "data".into()));
        let mut host = env::var("CRYSTAL_HOST")
            .ok()
            .map(|value| value.parse())
            .transpose()?
            .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));
        let mut port = env::var("CRYSTAL_PORT")
            .ok()
            .map(|value| value.parse())
            .transpose()?
            .unwrap_or(DEFAULT_PORT);
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--dir" => root = PathBuf::from(args.next().context("--dir requires a path")?),
                "--pack-dir" => {
                    pack_dir = PathBuf::from(args.next().context("--pack-dir requires a path")?)
                }
                "--host" => host = args.next().context("--host requires an IP")?.parse()?,
                "--port" => port = args.next().context("--port requires a number")?.parse()?,
                "-h" | "--help" => {
                    println!(
                        "crystal-web-server [--dir web-dist] [--pack-dir packs] [--host 127.0.0.1] [--port 8080]"
                    );
                    std::process::exit(0);
                }
                other => bail!("unknown argument '{other}'"),
            }
        }
        let auth_token = env::var("CRYSTAL_SERVER_TOKEN")
            .ok()
            .filter(|value| !value.is_empty());
        let auth_secret = env::var("CRYSTAL_AUTH_SECRET")
            .ok()
            .filter(|value| !value.is_empty());
        if auth_token.is_some() && auth_secret.is_some() {
            bail!("set only one of CRYSTAL_SERVER_TOKEN or CRYSTAL_AUTH_SECRET");
        }
        if auth_secret.as_ref().is_some_and(|secret| secret.len() < 32) {
            bail!("CRYSTAL_AUTH_SECRET must contain at least 32 bytes");
        }
        validate_public_auth(
            host,
            auth_secret.as_deref(),
            env::var("CRYSTAL_ALLOW_ANONYMOUS").as_deref() == Ok("true"),
        )?;
        let allowed_modpacks = parse_modpacks(&env::var("CRYSTAL_MODPACKS").unwrap_or_default())?;
        let max_clients = env::var("CRYSTAL_MAX_CLIENTS")
            .ok()
            .map(|value| value.parse())
            .transpose()
            .context("CRYSTAL_MAX_CLIENTS must be a positive integer")?
            .unwrap_or(20_000);
        if max_clients == 0 || max_clients > u32::MAX as usize {
            bail!("CRYSTAL_MAX_CLIENTS must be between 1 and 4294967295");
        }
        Ok(Self {
            root,
            pack_dir,
            data_dir,
            host,
            port,
            auth_token,
            auth_secret,
            allowed_modpacks,
            max_clients,
        })
    }
}

#[derive(Clone)]
struct AppState {
    hub: Arc<Mutex<Hub>>,
    senders: Arc<Mutex<HashMap<Uuid, ClientSender>>>,
    config: Arc<Config>,
    ratings_dirty: mpsc::Sender<()>,
    connection_slots: Arc<Semaphore>,
    shutdown: watch::Sender<bool>,
}

#[derive(Clone)]
struct ClientSender {
    tx: mpsc::Sender<OutboundMessage>,
    disconnect: Arc<Notify>,
}

enum OutboundMessage {
    Protocol(ServerMessage),
    Binary(Vec<u8>),
}

#[derive(Debug, Deserialize)]
struct SocketQuery {
    token: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusBody {
    ok: bool,
    protocol_version: u16,
    clients: usize,
    queued: usize,
    sessions: usize,
    auth_required: bool,
    max_clients: usize,
    modpacks: Vec<ModpackIdentity>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RatingFile {
    version: u16,
    ratings: HashMap<String, i32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthClaims {
    user_id: String,
    expires_at: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.first().is_some_and(|arg| arg == "--healthcheck") {
        let address = args.get(1).map(String::as_str).unwrap_or("127.0.0.1:8080");
        return run_healthcheck(address).await;
    }
    if args.first().is_some_and(|arg| arg == "--issue-token") {
        let user_id = args.get(1).context("--issue-token requires a user id")?;
        let ttl_seconds = args
            .get(2)
            .map(|value| value.parse())
            .transpose()
            .context("token TTL must be seconds")?
            .unwrap_or(30 * 24 * 60 * 60_u64);
        let secret = env::var("CRYSTAL_AUTH_SECRET")
            .context("CRYSTAL_AUTH_SECRET is required to issue user tokens")?;
        println!("{}", issue_user_token(&secret, user_id, ttl_seconds)?);
        return Ok(());
    }
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "crystal_web_server=info,tower_http=info".into()),
        )
        .init();
    let config = Arc::new(Config::parse(args)?);
    let mut hub = Hub::default();
    hub.replace_ratings(load_ratings(&config.data_dir).await?)
        .map_err(anyhow::Error::msg)?;
    let hub = Arc::new(Mutex::new(hub));
    let (ratings_dirty, ratings_rx) = mpsc::channel(1);
    let persistence_task = tokio::spawn(rating_persistence_task(
        Arc::clone(&hub),
        config.data_dir.clone(),
        ratings_rx,
    ));
    let state = AppState {
        hub,
        senders: Arc::new(Mutex::new(HashMap::new())),
        config: Arc::clone(&config),
        ratings_dirty,
        connection_slots: Arc::new(Semaphore::new(config.max_clients)),
        shutdown: watch::channel(false).0,
    };
    let static_files = ServeDir::new(&config.root)
        .precompressed_gzip()
        .append_index_html_on_directories(true);
    let app = Router::new()
        .route("/healthz", get(health))
        .route("/v1/status", get(status))
        .route("/v1/session", post(create_session))
        .route("/v1/ws", get(websocket))
        .nest_service("/packs", ServeDir::new(&config.pack_dir))
        .fallback_service(static_files)
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(cache_policy_headers))
        .with_state(state.clone());
    let address = SocketAddr::new(config.host, config.port);
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .with_context(|| format!("bind http://{address}"))?;
    info!(%address, web_root=%config.root.display(), pack_dir=%config.pack_dir.display(), "Rust multiplayer server ready");
    let shutdown = state.shutdown.clone();
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown_signal().await;
            shutdown.send_replace(true);
        })
        .await?;
    // Upgraded WebSockets outlive Axum's HTTP connection drain. Wait for their
    // cleanup (including pending rating notifications) before stopping persistence.
    let _drained = timeout(
        Duration::from_secs(10),
        state
            .connection_slots
            .acquire_many(config.max_clients as u32),
    )
    .await
    .context("WebSocket shutdown timed out")??;
    drop(_drained);
    drop(state);
    timeout(Duration::from_secs(2), persistence_task)
        .await
        .context("rating persistence shutdown timed out")?
        .context("rating persistence task failed")?;
    Ok(())
}

async fn run_healthcheck(address: &str) -> Result<()> {
    let mut stream = timeout(
        Duration::from_secs(2),
        tokio::net::TcpStream::connect(address),
    )
    .await
    .context("healthcheck connection timed out")?
    .with_context(|| format!("connect healthcheck endpoint {address}"))?;
    stream
        .write_all(b"GET /healthz HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .await?;
    let mut response = Vec::with_capacity(256);
    timeout(Duration::from_secs(2), stream.read_to_end(&mut response))
        .await
        .context("healthcheck response timed out")??;
    if !response.starts_with(b"HTTP/1.1 200") {
        bail!("healthcheck returned a non-200 response");
    }
    Ok(())
}

async fn health() -> &'static str {
    "ok\n"
}

async fn cache_policy_headers(request: Request, next: Next) -> Response {
    let policy = cache_policy(request.uri().path());
    let mut response = next.run(request).await;
    if response.status().is_success() {
        response
            .headers_mut()
            .insert(CACHE_CONTROL, HeaderValue::from_static(policy));
    }
    response.headers_mut().insert("origin-agent-cluster", HeaderValue::from_static("?1"));
    response.headers_mut().insert("permissions-policy", HeaderValue::from_static("tools=(self)"));
    response
}

fn cache_policy(path: &str) -> &'static str {
    if path == "/healthz" || path.starts_with("/v1/") {
        "no-store"
    } else {
        "public, max-age=0, must-revalidate"
    }
}

#[derive(Serialize)]
struct BrowserSession {
    token: String,
}

async fn create_session(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let secret = state.config.auth_secret.as_deref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    // The server chooses the identity; callers cannot claim another player's save.
    let player_id = (Uuid::new_v4().as_u128() as u64) | 1;
    let token = issue_user_token(secret, &format!("player-{player_id}"), 10 * 365 * 24 * 60 * 60)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(([(CACHE_CONTROL, "no-store")], Json(BrowserSession { token })))
}

async fn status(State(state): State<AppState>) -> Json<StatusBody> {
    let hub = state.hub.lock().await;
    let mut modpacks = state
        .config
        .allowed_modpacks
        .iter()
        .map(|(id, content_hash)| ModpackIdentity {
            id: id.clone(),
            content_hash: content_hash.clone(),
        })
        .collect::<Vec<_>>();
    modpacks.sort_by(|a, b| a.id.cmp(&b.id));
    Json(StatusBody {
        ok: true,
        protocol_version: PROTOCOL_VERSION,
        clients: hub.client_count(),
        queued: hub.queued_count(),
        sessions: hub.session_count(),
        auth_required: state.config.auth_token.is_some() || state.config.auth_secret.is_some(),
        max_clients: state.config.max_clients,
        modpacks,
    })
}

async fn websocket(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(query): Query<SocketQuery>,
    headers: HeaderMap,
) -> Response {
    let principal = match authenticate(&state.config, query.token.as_deref(), &headers) {
        Ok(principal) => principal,
        Err(()) => {
            return (
                StatusCode::UNAUTHORIZED,
                "invalid or expired server token\n",
            )
                .into_response();
        }
    };
    let permit = match Arc::clone(&state.connection_slots).try_acquire_owned() {
        Ok(permit) => permit,
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                "server is at player capacity\n",
            )
                .into_response();
        }
    };
    ws.max_message_size(MAX_RELAY_BYTES + 4096)
        .max_frame_size(MAX_RELAY_BYTES + 4096)
        .on_upgrade(move |socket| handle_socket(socket, state, principal, permit))
}

async fn handle_socket(
    socket: WebSocket,
    state: AppState,
    principal: Option<String>,
    _permit: OwnedSemaphorePermit,
) {
    let connection_id = Uuid::new_v4();
    let mut shutdown = state.shutdown.subscribe();
    if *shutdown.borrow() {
        return;
    }
    let disconnect = Arc::new(Notify::new());
    let (mut writer, mut reader) = socket.split();
    let (tx, mut rx) = mpsc::channel(OUTBOUND_CAPACITY);
    let first_result = tokio::select! {
        _ = shutdown.changed() => return,
        first = timeout(Duration::from_secs(10), reader.next()) => first,
    };
    let first = match first_result {
        Ok(Some(Ok(Message::Text(text)))) => text,
        _ => {
            let _ = timeout(WRITE_TIMEOUT, writer.send(Message::Close(None))).await;
            return;
        }
    };
    let hello = match serde_json::from_str::<ClientMessage>(&first) {
        Ok(ClientMessage::Hello {
            protocol_version,
            identity,
        }) if protocol_version == PROTOCOL_VERSION => identity,
        _ => {
            let _ = timeout(
                WRITE_TIMEOUT,
                writer.send(json_message(&ServerMessage::Error {
                    code: "invalid_hello".into(),
                    message: "first message must be a compatible hello".into(),
                })),
            )
            .await;
            return;
        }
    };
    if principal
        .as_ref()
        .is_some_and(|user_id| user_id != &hello.user_id)
    {
        let _ = timeout(
            WRITE_TIMEOUT,
            writer.send(json_message(&ServerMessage::Error {
                code: "identity_mismatch".into(),
                message: "authenticated token does not belong to hello user id".into(),
            })),
        )
        .await;
        return;
    }
    if !modpack_allowed(&state.config, &hello.world.modpack) {
        let _ = timeout(
            WRITE_TIMEOUT,
            writer.send(json_message(&ServerMessage::Error {
                code: "unsupported_modpack".into(),
                message: "server does not host this exact modpack hash".into(),
            })),
        )
        .await;
        return;
    }
    state.senders.lock().await.insert(
        connection_id,
        ClientSender {
            tx,
            disconnect: Arc::clone(&disconnect),
        },
    );
    let connected = state.hub.lock().await.connect(connection_id, hello);
    let deliveries = match connected {
        Ok(value) => value,
        Err(message) => {
            state.senders.lock().await.remove(&connection_id);
            let _ = timeout(
                WRITE_TIMEOUT,
                writer.send(json_message(&ServerMessage::Error {
                    code: "invalid_identity".into(),
                    message,
                })),
            )
            .await;
            return;
        }
    };
    dispatch(&state, deliveries).await;
    let mut heartbeat = tokio::time::interval(HEARTBEAT_INTERVAL);
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut last_received = Instant::now();
    let mut window = Instant::now();
    let mut message_count = 0_u32;
    loop {
        let result = tokio::select! {
            _ = shutdown.changed() => break,
            _ = disconnect.notified() => break,
            _ = tokio::time::sleep_until(last_received + IDLE_TIMEOUT) => break,
            _ = heartbeat.tick() => {
                if !matches!(timeout(WRITE_TIMEOUT, writer.send(Message::Ping(Vec::new().into()))).await, Ok(Ok(()))) { break; }
                continue;
            }
            message = rx.recv() => {
                let Some(message) = message else { break; };
                let message = match message {
                    OutboundMessage::Protocol(message) => json_message(&message),
                    OutboundMessage::Binary(bytes) => Message::Binary(bytes.into()),
                };
                if !matches!(timeout(WRITE_TIMEOUT, writer.send(message)).await, Ok(Ok(()))) { break; }
                continue;
            }
            result = reader.next() => {
                let Some(result) = result else { break; };
                result
            }
        };
        last_received = Instant::now();
        let message = match result {
            Ok(value) => value,
            Err(error) => {
                warn!(%connection_id, %error, "websocket read failed");
                break;
            }
        };
        if window.elapsed() >= Duration::from_secs(1) {
            window = Instant::now();
            message_count = 0;
        }
        message_count += 1;
        if message_count > MAX_MESSAGES_PER_SECOND {
            warn!(%connection_id, "rate limit exceeded");
            break;
        }
        match message {
            Message::Text(text) => match serde_json::from_str::<ClientMessage>(&text) {
                Ok(message) => {
                    let may_change_ratings = matches!(&message, ClientMessage::Result { .. });
                    let deliveries = state.hub.lock().await.handle(connection_id, message);
                    let settled = deliveries.iter().any(|delivery| {
                        matches!(delivery.message, ServerMessage::ResultSettled { .. })
                    });
                    dispatch(&state, deliveries).await;
                    if may_change_ratings && settled {
                        let _ = state.ratings_dirty.try_send(());
                    }
                }
                Err(error) => {
                    dispatch(
                        &state,
                        vec![Delivery {
                            connection_id,
                            message: ServerMessage::Error {
                                code: "invalid_json".into(),
                                message: error.to_string(),
                            },
                        }],
                    )
                    .await
                }
            },
            Message::Binary(envelope) => {
                let deliveries = relay_binary(&state, connection_id, envelope.to_vec()).await;
                dispatch(&state, deliveries).await;
            }
            Message::Close(_) => break,
            Message::Ping(payload) => {
                if !matches!(
                    timeout(WRITE_TIMEOUT, writer.send(Message::Pong(payload))).await,
                    Ok(Ok(()))
                ) {
                    break;
                }
            }
            _ => {}
        }
    }
    let deliveries = state.hub.lock().await.disconnect(connection_id);
    state.senders.lock().await.remove(&connection_id);
    dispatch(&state, deliveries).await;
    let _ = timeout(WRITE_TIMEOUT, writer.send(Message::Close(None))).await;
}

async fn load_ratings(data_dir: &PathBuf) -> Result<HashMap<String, i32>> {
    let path = data_dir.join("ratings.json");
    let bytes = match tokio::fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(HashMap::new()),
        Err(error) => return Err(error).with_context(|| format!("read {}", path.display())),
    };
    let file: RatingFile =
        serde_json::from_slice(&bytes).with_context(|| format!("decode {}", path.display()))?;
    if file.version != 1 {
        bail!(
            "{} has unsupported ratings version {}",
            path.display(),
            file.version
        );
    }
    Ok(file.ratings)
}

async fn rating_persistence_task(
    hub: Arc<Mutex<Hub>>,
    data_dir: PathBuf,
    mut dirty: mpsc::Receiver<()>,
) {
    while dirty.recv().await.is_some() {
        tokio::time::sleep(Duration::from_millis(100)).await;
        while dirty.try_recv().is_ok() {}
        if let Err(error) = persist_ratings(&hub, &data_dir).await {
            warn!(%error, "persisting multiplayer ratings failed");
        }
    }
}

async fn persist_ratings(hub: &Arc<Mutex<Hub>>, data_dir: &PathBuf) -> Result<()> {
    let ratings = hub.lock().await.ratings_snapshot();
    let file = RatingFile {
        version: 1,
        ratings,
    };
    tokio::fs::create_dir_all(data_dir)
        .await
        .with_context(|| format!("create {}", data_dir.display()))?;
    let path = data_dir.join("ratings.json");
    let temporary = data_dir.join(format!("ratings.{}.tmp", Uuid::new_v4()));
    tokio::fs::write(&temporary, serde_json::to_vec_pretty(&file)?)
        .await
        .with_context(|| format!("write {}", temporary.display()))?;
    tokio::fs::rename(&temporary, &path)
        .await
        .with_context(|| format!("replace {}", path.display()))?;
    Ok(())
}

async fn disconnect_sender(state: &AppState, connection_id: Uuid) {
    if let Some(sender) = state.senders.lock().await.remove(&connection_id) {
        sender.disconnect.notify_one();
    }
}

async fn dispatch(state: &AppState, deliveries: Vec<Delivery>) {
    for delivery in deliveries {
        let sender = state
            .senders
            .lock()
            .await
            .get(&delivery.connection_id)
            .cloned();
        if let Some(sender) = sender {
            let lossy = protocol_message_is_lossy(&delivery.message);
            match sender
                .tx
                .try_send(OutboundMessage::Protocol(delivery.message))
            {
                Ok(()) => {}
                Err(mpsc::error::TrySendError::Full(_)) if lossy => {}
                Err(_) => {
                    // Never continue a session after losing a control message.
                    // The connection loop removes the player and cancels its match.
                    warn!(connection_id=%delivery.connection_id, "disconnecting client after failed delivery");
                    disconnect_sender(state, delivery.connection_id).await;
                }
            }
        }
    }
}

fn protocol_message_is_lossy(message: &ServerMessage) -> bool {
    matches!(
        message,
        ServerMessage::Presence { .. } | ServerMessage::Pong { .. }
    )
}

async fn relay_binary(state: &AppState, connection_id: Uuid, envelope: Vec<u8>) -> Vec<Delivery> {
    const SESSION_ID_BYTES: usize = 16;
    if envelope.len() <= SESSION_ID_BYTES || envelope.len() > SESSION_ID_BYTES + MAX_RELAY_BYTES {
        return vec![protocol_error(
            connection_id,
            "binary relay frame has an invalid size",
        )];
    }
    let session_id = match Uuid::from_slice(&envelope[..SESSION_ID_BYTES]) {
        Ok(value) => value,
        Err(_) => {
            return vec![protocol_error(
                connection_id,
                "binary relay session id is invalid",
            )];
        }
    };
    let target = match state
        .hub
        .lock()
        .await
        .binary_relay_target(connection_id, session_id)
    {
        Ok(value) => value,
        Err(message) => return vec![protocol_error(connection_id, message)],
    };
    let sender = state.senders.lock().await.get(&target).cloned();
    if let Some(sender) = sender {
        if sender
            .tx
            .try_send(OutboundMessage::Binary(envelope))
            .is_err()
        {
            disconnect_sender(state, target).await;
            return vec![protocol_error(
                connection_id,
                "matched peer is not accepting relay frames",
            )];
        }
    } else {
        return vec![protocol_error(connection_id, "matched peer is offline")];
    }
    Vec::new()
}

fn protocol_error(connection_id: Uuid, message: impl Into<String>) -> Delivery {
    Delivery {
        connection_id,
        message: ServerMessage::Error {
            code: "invalid_request".into(),
            message: message.into(),
        },
    }
}

fn json_message(message: &ServerMessage) -> Message {
    Message::Text(
        serde_json::to_string(message)
            .expect("server messages serialize")
            .into(),
    )
}

fn authenticate(
    config: &Config,
    query_token: Option<&str>,
    headers: &HeaderMap,
) -> Result<Option<String>, ()> {
    let bearer = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    let token = query_token.or(bearer);
    if let Some(secret) = &config.auth_secret {
        return verify_user_token(secret, token.ok_or(())?).map(Some);
    }
    let Some(expected) = &config.auth_token else {
        return Ok(None);
    };
    token
        .filter(|actual| constant_time_equal(actual.as_bytes(), expected.as_bytes()))
        .map(|_| None)
        .ok_or(())
}

fn validate_public_auth(host: IpAddr, secret: Option<&str>, allow_anonymous: bool) -> Result<()> {
    if !host.is_loopback() && secret.is_none() && !allow_anonymous {
        bail!(
            "public listeners require CRYSTAL_AUTH_SECRET; use CRYSTAL_ALLOW_ANONYMOUS=true only for an explicitly unauthenticated test server"
        );
    }
    Ok(())
}

fn issue_user_token(secret: &str, user_id: &str, ttl_seconds: u64) -> Result<String> {
    if secret.len() < 32 {
        bail!("CRYSTAL_AUTH_SECRET must contain at least 32 bytes");
    }
    if ttl_seconds == 0 {
        bail!("token TTL must be greater than zero");
    }
    crystal_web_server::hub::validate_public_token("user id", user_id)
        .map_err(anyhow::Error::msg)?;
    let now = unix_time_seconds()?;
    let claims = AuthClaims {
        user_id: user_id.into(),
        expires_at: now
            .checked_add(ttl_seconds)
            .context("token expiry overflow")?,
    };
    let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims)?);
    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).context("initialize token signer")?;
    mac.update(payload.as_bytes());
    let signature = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
    Ok(format!("{payload}.{signature}"))
}

fn verify_user_token(secret: &str, token: &str) -> Result<String, ()> {
    let (payload, signature) = token.split_once('.').ok_or(())?;
    if payload.is_empty() || signature.is_empty() || signature.contains('.') {
        return Err(());
    }
    let signature = URL_SAFE_NO_PAD.decode(signature).map_err(|_| ())?;
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).map_err(|_| ())?;
    mac.update(payload.as_bytes());
    mac.verify_slice(&signature).map_err(|_| ())?;
    let claims: AuthClaims =
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload).map_err(|_| ())?)
            .map_err(|_| ())?;
    crystal_web_server::hub::validate_public_token("user id", &claims.user_id).map_err(|_| ())?;
    if claims.expires_at <= unix_time_seconds().map_err(|_| ())? {
        return Err(());
    }
    Ok(claims.user_id)
}

fn unix_time_seconds() -> Result<u64> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .context("system clock is before Unix epoch")?
        .as_secs())
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}

fn modpack_allowed(config: &Config, modpack: &ModpackIdentity) -> bool {
    config.allowed_modpacks.is_empty()
        || config
            .allowed_modpacks
            .get(&modpack.id)
            .is_some_and(|hash| hash == &modpack.content_hash)
}

fn parse_modpacks(value: &str) -> Result<HashMap<String, String>> {
    let mut output = HashMap::new();
    for entry in value
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
    {
        let (id, hash) = entry
            .split_once('=')
            .context("CRYSTAL_MODPACKS entries must be id=content_hash")?;
        if id.is_empty() || hash.is_empty() {
            bail!("CRYSTAL_MODPACKS contains an empty id or hash")
        }
        if output.insert(id.into(), hash.into()).is_some() {
            bail!("duplicate modpack id '{id}'")
        }
    }
    Ok(output)
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("install Ctrl-C handler")
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = terminate => {} }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_state() -> AppState {
        let (ratings_dirty, _) = mpsc::channel(1);
        AppState {
            hub: Arc::new(Mutex::new(Hub::default())),
            senders: Arc::new(Mutex::new(HashMap::new())),
            config: Arc::new(Config {
                root: PathBuf::new(),
                pack_dir: PathBuf::new(),
                data_dir: PathBuf::new(),
                host: IpAddr::V4(Ipv4Addr::LOCALHOST),
                port: 0,
                auth_token: None,
                auth_secret: None,
                allowed_modpacks: HashMap::new(),
                max_clients: 2,
            }),
            ratings_dirty,
            connection_slots: Arc::new(Semaphore::new(2)),
            shutdown: watch::channel(false).0,
        }
    }

    #[tokio::test]
    async fn browser_can_create_an_online_identity_without_an_invite() {
        let mut state = test_state();
        let secret = "0123456789abcdef0123456789abcdef";
        Arc::make_mut(&mut state.config).auth_secret = Some(secret.into());
        let mut identities = Vec::new();
        for _ in 0..2 {
            let response = create_session(State(state.clone())).await.unwrap().into_response();
            assert_eq!(response.headers()[CACHE_CONTROL], "no-store");
            let body = axum::body::to_bytes(response.into_body(), 4096).await.unwrap();
            let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
            let identity = verify_user_token(secret, body["token"].as_str().unwrap()).unwrap();
            assert!(identity.strip_prefix("player-").unwrap().parse::<u64>().unwrap() > 0);
            identities.push(identity);
        }
        assert_ne!(identities[0], identities[1]);
    }

    #[tokio::test]
    async fn stalled_critical_delivery_removes_connection_sender() {
        let state = test_state();
        let id = Uuid::new_v4();
        let (tx, _rx) = mpsc::channel(1);
        tx.try_send(OutboundMessage::Binary(vec![1])).unwrap();
        state.senders.lock().await.insert(
            id,
            ClientSender {
                tx,
                disconnect: Arc::new(Notify::new()),
            },
        );
        dispatch(
            &state,
            vec![Delivery {
                connection_id: id,
                message: ServerMessage::ResultPending {
                    session_id: Uuid::new_v4(),
                },
            }],
        )
        .await;
        assert!(!state.senders.lock().await.contains_key(&id));
    }

    #[tokio::test]
    async fn unresponsive_socket_releases_player_and_capacity() {
        let state = test_state();
        let app = Router::new()
            .route("/v1/ws", get(websocket))
            .with_state(state.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        let (mut socket, _) = tokio_tungstenite::connect_async(format!("ws://{address}/v1/ws"))
            .await
            .unwrap();
        let hello = ClientMessage::Hello {
            protocol_version: PROTOCOL_VERSION,
            identity: crystal_net::hosted::ClientIdentity {
                user_id: "player-1".into(),
                display_name: "CHRIS".into(),
                world: crystal_net::hosted::WorldIdentity {
                    world_id: "main".into(),
                    modpack: ModpackIdentity {
                        id: "core".into(),
                        content_hash: "hash".into(),
                    },
                },
            },
        };
        socket
            .send(tokio_tungstenite::tungstenite::Message::Text(
                serde_json::to_string(&hello).unwrap().into(),
            ))
            .await
            .unwrap();
        socket.next().await.unwrap().unwrap();
        assert_eq!(state.hub.lock().await.client_count(), 1);
        // Do not poll or pong: simulate a network connection that stopped responding.
        tokio::time::sleep(Duration::from_millis(500)).await;
        let count = state.hub.lock().await.client_count();
        let capacity = state.connection_slots.available_permits();
        drop(socket);
        server.abort();
        assert_eq!(count, 0);
        assert_eq!(capacity, 2);
    }

    #[tokio::test]
    async fn authenticated_socket_heartbeats_and_shutdown_release_identity() {
        use tokio_tungstenite::tungstenite::Message as WireMessage;
        let mut state = test_state();
        let secret = "0123456789abcdef0123456789abcdef";
        Arc::make_mut(&mut state.config).auth_secret = Some(secret.into());
        let app = Router::new()
            .route("/v1/ws", get(websocket))
            .with_state(state.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        assert!(
            tokio_tungstenite::connect_async(format!("ws://{address}/v1/ws"))
                .await
                .is_err()
        );
        let token = issue_user_token(secret, "player-1", 60).unwrap();
        let (mut socket, _) =
            tokio_tungstenite::connect_async(format!("ws://{address}/v1/ws?token={token}"))
                .await
                .unwrap();
        let mut hello = ClientMessage::Hello {
            protocol_version: PROTOCOL_VERSION,
            identity: crystal_net::hosted::ClientIdentity {
                user_id: "player-2".into(),
                display_name: "CHRIS".into(),
                world: crystal_net::hosted::WorldIdentity {
                    world_id: "main".into(),
                    modpack: ModpackIdentity {
                        id: "core".into(),
                        content_hash: "hash".into(),
                    },
                },
            },
        };
        socket
            .send(WireMessage::Text(
                serde_json::to_string(&hello).unwrap().into(),
            ))
            .await
            .unwrap();
        let rejected = socket.next().await.unwrap().unwrap();
        assert!(rejected.to_text().unwrap().contains("identity_mismatch"));
        drop(socket);
        let (mut socket, _) =
            tokio_tungstenite::connect_async(format!("ws://{address}/v1/ws?token={token}"))
                .await
                .unwrap();
        if let ClientMessage::Hello { identity, .. } = &mut hello {
            identity.user_id = "player-1".into();
        }
        socket
            .send(WireMessage::Text(
                serde_json::to_string(&hello).unwrap().into(),
            ))
            .await
            .unwrap();
        let deadline = tokio::time::sleep(IDLE_TIMEOUT * 2);
        tokio::pin!(deadline);
        let mut welcomed = false;
        let mut pings = 0;
        loop {
            tokio::select! {
                _ = &mut deadline => break,
                message = socket.next() => match message.unwrap().unwrap() {
                    WireMessage::Ping(payload) => { pings += 1; socket.send(WireMessage::Pong(payload)).await.unwrap(); }
                    WireMessage::Text(text) => { assert!(text.contains("welcome")); welcomed = true; }
                    other => panic!("unexpected heartbeat response: {other:?}"),
                }
            }
        }
        assert!(welcomed && pings > 1);
        assert_eq!(state.hub.lock().await.client_count(), 1);
        state.shutdown.send_replace(true);
        timeout(Duration::from_secs(1), async {
            while state.connection_slots.available_permits() != 2 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(state.hub.lock().await.client_count(), 0);
        assert!(state.senders.lock().await.is_empty());
        drop(socket);
        server.abort();
    }

    #[test]
    fn public_listener_requires_identity_authentication() {
        let public = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
        assert!(validate_public_auth(public, None, false).is_err());
        assert!(
            validate_public_auth(public, Some("0123456789abcdef0123456789abcdef"), false).is_ok()
        );
        assert!(validate_public_auth(IpAddr::V4(Ipv4Addr::LOCALHOST), None, false).is_ok());
        assert!(validate_public_auth(public, None, true).is_ok());
    }

    #[test]
    fn player_departure_must_not_be_dropped() {
        assert!(!protocol_message_is_lossy(&ServerMessage::PresenceLeft {
            user_id: "player-1".into()
        }));
    }

    #[test]
    fn parses_exact_modpack_allowlist() {
        let packs = parse_modpacks("core=abc,gen3=def").unwrap();
        assert_eq!(packs.get("gen3").map(String::as_str), Some("def"));
        assert!(parse_modpacks("broken").is_err());
    }

    #[test]
    fn token_comparison_requires_exact_bytes() {
        assert!(constant_time_equal(b"secret", b"secret"));
        assert!(!constant_time_equal(b"secret", b"Secret"));
        assert!(!constant_time_equal(b"secret", b"secret2"));
    }

    #[test]
    fn cache_policy_keeps_protocol_uncached_and_static_assets_revalidated() {
        assert_eq!(cache_policy("/v1/status"), "no-store");
        assert_eq!(
            cache_policy("/crystal-bevy_bg.wasm"),
            "public, max-age=0, must-revalidate"
        );
    }

    #[test]
    fn signed_user_tokens_bind_identity_and_reject_tampering() {
        let secret = "0123456789abcdef0123456789abcdef";
        let token = issue_user_token(secret, "player-42", 60).unwrap();
        assert_eq!(verify_user_token(secret, &token), Ok("player-42".into()));
        let mut tampered = token.into_bytes();
        let index = tampered.len() - 1;
        tampered[index] = if tampered[index] == b'A' { b'B' } else { b'A' };
        assert!(verify_user_token(secret, std::str::from_utf8(&tampered).unwrap()).is_err());
        assert!(issue_user_token("short", "player-42", 60).is_err());
        assert!(issue_user_token(secret, "bad user", 60).is_err());
    }
}
