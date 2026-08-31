use std::collections::HashMap;
use std::env;
use std::time::Instant;

use anyhow::{Context, Result, anyhow, ensure};
use crystal_web_server::hub::{
    ClientIdentity, ClientMessage, Hub, MatchMode, MatchOutcome, ModpackIdentity, PROTOCOL_VERSION,
    ServerMessage, WorldIdentity,
};
use futures_util::{SinkExt, StreamExt, stream::FuturesUnordered};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};
use uuid::Uuid;

type NetworkSocket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

#[tokio::main]
async fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    if args.next().as_deref() == Some("--network") {
        let url = args
            .next()
            .context("--network requires a ws:// or wss:// URL")?;
        let users = args
            .next()
            .map(|value| value.parse())
            .transpose()?
            .unwrap_or(1_000_usize);
        return network_load(&url, users).await;
    }
    let users = env::args()
        .nth(1)
        .map(|value| value.parse())
        .transpose()?
        .unwrap_or(10_000_usize);
    hub_load(users)
}

fn hub_load(users: usize) -> Result<()> {
    ensure_even_users(users)?;
    let started = Instant::now();
    let mut hub = Hub::default();
    let mut connections = Vec::with_capacity(users);
    for index in 0..users {
        let id = Uuid::new_v4();
        hub.connect(id, identity(index))
            .map_err(|error| anyhow!("connect user {index}: {error}"))?;
        hub.handle(
            id,
            ClientMessage::Presence {
                map: format!("MAP_{}", index % 100),
                tile_x: (index % 20) as i32,
                tile_y: ((index / 20) % 20) as i32,
                direction: "down".into(),
            },
        );
        connections.push(id);
    }
    let connected_at = started.elapsed();
    let mut sessions = Vec::with_capacity(users / 2);
    for (index, id) in connections.iter().copied().enumerate() {
        let deliveries = hub.handle(
            id,
            ClientMessage::QueueJoin {
                mode: MatchMode::Battle,
                rating: 1000 + (index % 20) as i32,
                rating_range: 100,
            },
        );
        if deliveries.len() == 2 {
            let ServerMessage::MatchFound { session_id, .. } = deliveries[0].message else {
                anyhow::bail!("unexpected match response")
            };
            sessions.push((
                session_id,
                deliveries[0].connection_id,
                deliveries[1].connection_id,
            ));
        }
    }
    let matched_at = started.elapsed();
    ensure!(
        hub.client_count() == users,
        "not every client remained connected"
    );
    ensure!(hub.queued_count() == 0, "queue did not drain");
    ensure!(hub.session_count() == users / 2, "incorrect session count");
    for (session_id, host, guest) in sessions {
        let relay = hub.handle(
            host,
            ClientMessage::Relay {
                session_id,
                payload: serde_json::json!({"frame": 1, "input": "a"}),
            },
        );
        ensure!(
            relay.len() == 1 && relay[0].connection_id == guest,
            "relay was not delivered to peer"
        );
        hub.handle(
            host,
            ClientMessage::Result {
                session_id,
                outcome: MatchOutcome::Local,
            },
        );
        let settled = hub.handle(
            guest,
            ClientMessage::Result {
                session_id,
                outcome: MatchOutcome::Remote,
            },
        );
        ensure!(
            settled.len() == 2,
            "session did not settle for both players"
        );
    }
    let total = started.elapsed();
    println!(
        "mode=hub users={users} sessions={} connect_ms={} match_ms={} settle_ms={} total_ms={} users_per_second={:.0}",
        users / 2,
        connected_at.as_millis(),
        (matched_at - connected_at).as_millis(),
        (total - matched_at).as_millis(),
        total.as_millis(),
        users as f64 / total.as_secs_f64(),
    );
    Ok(())
}

async fn network_load(url: &str, users: usize) -> Result<()> {
    ensure_even_users(users)?;
    ensure!(
        url.starts_with("ws://") || url.starts_with("wss://"),
        "network URL must use ws:// or wss://"
    );
    let started = Instant::now();
    let mut connecting = futures_util::stream::iter(0..users)
        .map(|index| connect_client(url.to_owned(), index))
        .buffer_unordered(512);
    let mut sockets = Vec::with_capacity(users);
    while let Some(result) = connecting.next().await {
        sockets.push(result?);
    }
    let connected_at = started.elapsed();

    for (index, socket) in sockets.iter_mut().enumerate() {
        send(
            socket,
            &ClientMessage::Presence {
                map: format!("LOAD_PAIR_{}", index / 2),
                tile_x: (index % 2) as i32,
                tile_y: 8,
                direction: if index % 2 == 0 { "right" } else { "left" }.into(),
            },
        )
        .await?;
    }
    let mut observing = sockets
        .into_iter()
        .enumerate()
        .map(|(index, mut socket)| async move {
            loop {
                match receive(&mut socket).await? {
                    ServerMessage::Presence {
                        user_id,
                        map,
                        tile_x,
                        ..
                    } => {
                        let peer = index ^ 1;
                        ensure!(user_id.starts_with("load-"), "wrong ghost identity");
                        ensure!(
                            map == format!("LOAD_PAIR_{}", index / 2),
                            "ghost crossed map boundary"
                        );
                        ensure!(tile_x == (peer % 2) as i32, "wrong ghost position");
                        return Ok::<_, anyhow::Error>(socket);
                    }
                    _ => {}
                }
            }
        })
        .collect::<FuturesUnordered<_>>();
    let mut sockets = Vec::with_capacity(users);
    while let Some(result) = observing.next().await {
        sockets.push(result?);
    }
    let presence_at = started.elapsed();

    for (index, socket) in sockets.iter_mut().enumerate() {
        send(
            socket,
            &ClientMessage::QueueJoin {
                mode: MatchMode::Battle,
                rating: 1000 + (index % 20) as i32,
                rating_range: 100,
            },
        )
        .await?;
    }
    let mut matching = sockets
        .into_iter()
        .map(wait_for_match)
        .collect::<FuturesUnordered<_>>();
    let mut matched = Vec::with_capacity(users);
    while let Some(result) = matching.next().await {
        matched.push(result?);
    }
    let matched_at = started.elapsed();

    let mut sessions = HashMap::<Uuid, Vec<(NetworkSocket, bool)>>::with_capacity(users / 2);
    for (socket, session_id, is_host) in matched {
        sessions
            .entry(session_id)
            .or_default()
            .push((socket, is_host));
    }
    let mut relayed = Vec::with_capacity(users);
    for (session_id, mut peers) in sessions {
        ensure!(peers.len() == 2, "match did not contain exactly two peers");
        let (mut second, second_is_host) = peers.pop().expect("two peers");
        let (mut first, first_is_host) = peers.pop().expect("two peers");
        let mut envelope = session_id.as_bytes().to_vec();
        envelope.extend_from_slice(b"crystal-link-load-frame");
        first.send(Message::Binary(envelope.clone().into())).await?;
        loop {
            match second
                .next()
                .await
                .context("server closed during binary relay")??
            {
                Message::Binary(received) => {
                    ensure!(
                        received.as_ref() == envelope,
                        "binary relay payload changed"
                    );
                    break;
                }
                Message::Ping(payload) => second.send(Message::Pong(payload)).await?,
                Message::Close(frame) => anyhow::bail!("server closed during relay: {frame:?}"),
                _ => {}
            }
        }
        send(
            &mut first,
            &ClientMessage::Result {
                session_id,
                outcome: if first_is_host {
                    MatchOutcome::Local
                } else {
                    MatchOutcome::Remote
                },
            },
        )
        .await?;
        send(
            &mut second,
            &ClientMessage::Result {
                session_id,
                outcome: if second_is_host {
                    MatchOutcome::Local
                } else {
                    MatchOutcome::Remote
                },
            },
        )
        .await?;
        wait_for_settlement(&mut first, session_id).await?;
        wait_for_settlement(&mut second, session_id).await?;
        relayed.push(first);
        relayed.push(second);
    }
    let settled_at = started.elapsed();

    let mut pinging = relayed
        .into_iter()
        .enumerate()
        .map(|(index, mut socket)| async move {
            send(
                &mut socket,
                &ClientMessage::Ping {
                    nonce: index as u64,
                },
            )
            .await?;
            loop {
                match receive(&mut socket).await? {
                    ServerMessage::Pong { nonce } if nonce == index as u64 => {
                        return Ok::<_, anyhow::Error>(socket);
                    }
                    _ => {}
                }
            }
        })
        .collect::<FuturesUnordered<_>>();
    let mut kept_open = Vec::with_capacity(users);
    while let Some(result) = pinging.next().await {
        kept_open.push(result?);
    }
    let total = started.elapsed();
    println!(
        "mode=network users={users} sessions={} connect_ms={} ghost_ms={} match_ms={} relay_settle_ms={} ping_ms={} total_ms={} users_per_second={:.0}",
        users / 2,
        connected_at.as_millis(),
        (presence_at - connected_at).as_millis(),
        (matched_at - presence_at).as_millis(),
        (settled_at - matched_at).as_millis(),
        (total - settled_at).as_millis(),
        total.as_millis(),
        users as f64 / total.as_secs_f64(),
    );
    let mut closing = futures_util::stream::iter(kept_open)
        .map(|mut socket| async move { socket.close(None).await })
        .buffer_unordered(512);
    while let Some(result) = closing.next().await {
        result.context("close load-test WebSocket")?;
    }
    Ok(())
}

async fn connect_client(url: String, index: usize) -> Result<NetworkSocket> {
    let (mut socket, _) = connect_async(&url)
        .await
        .with_context(|| format!("connect user {index}"))?;
    send(
        &mut socket,
        &ClientMessage::Hello {
            protocol_version: PROTOCOL_VERSION,
            identity: identity(index),
        },
    )
    .await?;
    ensure!(
        matches!(receive(&mut socket).await?, ServerMessage::Welcome { .. }),
        "user {index} did not receive welcome"
    );
    Ok(socket)
}

async fn wait_for_match(mut socket: NetworkSocket) -> Result<(NetworkSocket, Uuid, bool)> {
    loop {
        if let ServerMessage::MatchFound {
            session_id,
            is_host,
            ..
        } = receive(&mut socket).await?
        {
            return Ok((socket, session_id, is_host));
        }
    }
}

async fn wait_for_settlement(socket: &mut NetworkSocket, expected_session: Uuid) -> Result<()> {
    loop {
        if let ServerMessage::ResultSettled { session_id, .. } = receive(socket).await? {
            ensure!(session_id == expected_session, "settled the wrong session");
            return Ok(());
        }
    }
}

async fn send(socket: &mut NetworkSocket, message: &ClientMessage) -> Result<()> {
    socket
        .send(Message::Text(serde_json::to_string(message)?.into()))
        .await?;
    Ok(())
}

async fn receive(socket: &mut NetworkSocket) -> Result<ServerMessage> {
    loop {
        match socket
            .next()
            .await
            .context("server closed the WebSocket")??
        {
            Message::Text(text) => return Ok(serde_json::from_str(&text)?),
            Message::Ping(payload) => socket.send(Message::Pong(payload)).await?,
            Message::Close(frame) => anyhow::bail!("server closed the WebSocket: {frame:?}"),
            _ => {}
        }
    }
}

fn ensure_even_users(users: usize) -> Result<()> {
    ensure!(
        users >= 2 && users % 2 == 0,
        "user count must be an even integer >= 2"
    );
    Ok(())
}

fn identity(index: usize) -> ClientIdentity {
    ClientIdentity {
        user_id: format!("load-{index}"),
        display_name: format!("Load {index}"),
        world: WorldIdentity {
            world_id: "load".into(),
            modpack: ModpackIdentity {
                id: "core-modular".into(),
                content_hash: "load.hash".into(),
            },
        },
    }
}
