use std::collections::{HashMap, HashSet, VecDeque};

use uuid::Uuid;

pub use crystal_net::hosted::{
    ClientIdentity, ClientMessage, MAX_RELAY_BYTES, MatchMode, MatchOutcome, ModpackIdentity,
    PROTOCOL_VERSION, ServerMessage, WorldIdentity,
};

pub const MAX_NAME_BYTES: usize = 32;
pub const MAX_TOKEN_BYTES: usize = 128;
pub const MAX_RATING_RANGE: u32 = 1_000;
const PRESENCE_RADIUS_X: u32 = 12;
const PRESENCE_RADIUS_Y: u32 = 10;
const PRESENCE_CELL_SIZE: i32 = 16;

#[derive(Debug, Clone, PartialEq)]
pub struct Delivery {
    pub connection_id: Uuid,
    pub message: ServerMessage,
}

#[derive(Debug, Clone)]
struct ClientRecord {
    identity: ClientIdentity,
    presence: Option<PresenceRecord>,
}

#[derive(Debug, Clone)]
struct PresenceRecord {
    map: String,
    tile_x: i32,
    tile_y: i32,
    direction: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PresenceCell {
    world: WorldIdentity,
    map: String,
    x: i32,
    y: i32,
}

#[derive(Debug, Clone)]
struct QueueEntry {
    connection_id: Uuid,
    mode: MatchMode,
    rating: i32,
    rating_range: u32,
}

#[derive(Debug, Clone)]
struct Session {
    players: [Uuid; 2],
    mode: MatchMode,
    ranked: bool,
    reports: HashMap<Uuid, MatchOutcome>,
    settled: bool,
}

#[derive(Debug, Clone)]
struct PendingInteraction {
    from: Uuid,
    target: Uuid,
    kind: MatchMode,
}

#[derive(Debug, Default)]
pub struct Hub {
    clients: HashMap<Uuid, ClientRecord>,
    users: HashMap<String, Uuid>,
    queues: HashMap<WorldIdentity, VecDeque<QueueEntry>>,
    sessions: HashMap<Uuid, Session>,
    interactions: HashMap<Uuid, PendingInteraction>,
    ratings: HashMap<String, i32>,
    presence_cells: HashMap<PresenceCell, HashSet<Uuid>>,
}

impl Hub {
    pub fn connect(
        &mut self,
        connection_id: Uuid,
        identity: ClientIdentity,
    ) -> Result<Vec<Delivery>, String> {
        validate_identity(&identity)?;
        if self.users.contains_key(&identity.user_id) {
            return Err("user already has an active connection".into());
        }
        self.users.insert(identity.user_id.clone(), connection_id);
        self.clients.insert(
            connection_id,
            ClientRecord {
                identity,
                presence: None,
            },
        );
        Ok(vec![deliver(
            connection_id,
            ServerMessage::Welcome {
                protocol_version: PROTOCOL_VERSION,
                connection_id,
            },
        )])
    }

    pub fn disconnect(&mut self, connection_id: Uuid) -> Vec<Delivery> {
        self.remove_from_queues(connection_id);
        let Some(client) = self.clients.remove(&connection_id) else {
            return Vec::new();
        };
        self.users.remove(&client.identity.user_id);
        self.interactions
            .retain(|_, request| request.from != connection_id && request.target != connection_id);
        let ended = self
            .sessions
            .iter()
            .filter(|(_, session)| session.players.contains(&connection_id))
            .map(|(session_id, session)| (*session_id, session.clone()))
            .collect::<Vec<_>>();
        self.sessions
            .retain(|_, session| !session.players.contains(&connection_id));
        let mut output = ended
            .into_iter()
            .filter_map(|(session_id, session)| {
                peer_for(&session, connection_id).ok().map(|peer| {
                    deliver(
                        peer,
                        ServerMessage::ResultSettled {
                            session_id,
                            winner_user_id: None,
                            ranked: session.ranked,
                        },
                    )
                })
            })
            .collect::<Vec<_>>();
        if let Some(presence) = client.presence {
            self.remove_presence_index(&client.identity.world, &presence, connection_id);
            output.extend(
                self.presence_candidates(
                    &client.identity.world,
                    Some(&presence),
                    &presence,
                    connection_id,
                )
                .into_iter()
                .filter(|id| {
                    self.clients
                        .get(id)
                        .and_then(|peer| peer.presence.as_ref())
                        .is_some_and(|value| presences_are_visible(&presence, value))
                })
                .map(|id| {
                    deliver(
                        id,
                        ServerMessage::PresenceLeft {
                            user_id: client.identity.user_id.clone(),
                        },
                    )
                }),
            );
        }
        output
    }

    pub fn handle(&mut self, connection_id: Uuid, message: ClientMessage) -> Vec<Delivery> {
        match self.handle_checked(connection_id, message) {
            Ok(deliveries) => deliveries,
            Err(message) => vec![deliver(
                connection_id,
                ServerMessage::Error {
                    code: "invalid_request".into(),
                    message,
                },
            )],
        }
    }

    fn handle_checked(
        &mut self,
        connection_id: Uuid,
        message: ClientMessage,
    ) -> Result<Vec<Delivery>, String> {
        if !self.clients.contains_key(&connection_id) {
            return Err("connection is not registered".into());
        }
        match message {
            ClientMessage::Hello { .. } => Err("hello may only be sent once".into()),
            ClientMessage::Presence {
                map,
                tile_x,
                tile_y,
                direction,
            } => {
                validate_token("map", &map)?;
                if i16::try_from(tile_x).is_err() || i16::try_from(tile_y).is_err() {
                    return Err("presence tile is outside the supported map range".into());
                }
                if !matches!(direction.as_str(), "up" | "down" | "left" | "right") {
                    return Err("invalid direction".into());
                }
                let next_presence = PresenceRecord {
                    map: map.clone(),
                    tile_x,
                    tile_y,
                    direction: direction.clone(),
                };
                let (identity, old_presence) = {
                    let client = self.clients.get_mut(&connection_id).expect("checked");
                    let old_presence = client.presence.replace(next_presence.clone());
                    (client.identity.clone(), old_presence)
                };
                self.remove_presence_index(
                    &identity.world,
                    old_presence.as_ref().unwrap_or(&next_presence),
                    connection_id,
                );
                self.insert_presence_index(&identity.world, &next_presence, connection_id);
                let mut output = Vec::new();
                let candidates = self.presence_candidates(
                    &identity.world,
                    old_presence.as_ref(),
                    &next_presence,
                    connection_id,
                );
                for id in candidates {
                    let peer = self.clients.get(&id).expect("presence index client exists");
                    let Some(peer_presence) = peer.presence.as_ref() else {
                        continue;
                    };
                    let was_visible = old_presence
                        .as_ref()
                        .is_some_and(|old| presences_are_visible(old, peer_presence));
                    let is_visible = presences_are_visible(&next_presence, peer_presence);
                    match (was_visible, is_visible) {
                        (false, true) => {
                            output.push(deliver(
                                connection_id,
                                presence_message(&peer.identity, peer_presence),
                            ));
                            output.push(deliver(id, presence_message(&identity, &next_presence)));
                        }
                        (true, true) => {
                            output.push(deliver(id, presence_message(&identity, &next_presence)))
                        }
                        (true, false) => {
                            output.push(deliver(
                                connection_id,
                                ServerMessage::PresenceLeft {
                                    user_id: peer.identity.user_id.clone(),
                                },
                            ));
                            output.push(deliver(
                                id,
                                ServerMessage::PresenceLeft {
                                    user_id: identity.user_id.clone(),
                                },
                            ));
                        }
                        (false, false) => {}
                    }
                }
                Ok(output)
            }
            ClientMessage::QueueJoin {
                mode,
                rating,
                rating_range,
            } => self.join_queue(connection_id, mode, rating, rating_range),
            ClientMessage::QueueLeave => {
                self.remove_from_queues(connection_id);
                Ok(vec![deliver(connection_id, ServerMessage::QueueLeft)])
            }
            ClientMessage::Relay {
                session_id,
                payload,
            } => self.relay(connection_id, session_id, payload),
            ClientMessage::InteractionRequest {
                target_user_id,
                kind,
            } => self.interaction_request(connection_id, target_user_id, kind),
            ClientMessage::InteractionResponse {
                request_id,
                target_user_id,
                accepted,
            } => self.interaction_response(connection_id, request_id, target_user_id, accepted),
            ClientMessage::Result {
                session_id,
                outcome,
            } => self.report_result(connection_id, session_id, outcome),
            ClientMessage::Ping { nonce } => {
                Ok(vec![deliver(connection_id, ServerMessage::Pong { nonce })])
            }
        }
    }

    fn join_queue(
        &mut self,
        connection_id: Uuid,
        mode: MatchMode,
        _claimed_rating: i32,
        rating_range: u32,
    ) -> Result<Vec<Delivery>, String> {
        if self.active_session_for(connection_id).is_some() {
            return Err("client is already in an active session".into());
        }
        self.remove_from_queues(connection_id);
        let rating_range = rating_range.min(MAX_RATING_RANGE);
        let client = self.clients.get(&connection_id).expect("checked");
        let rating = *self.ratings.get(&client.identity.user_id).unwrap_or(&1000);
        let world = client.identity.world.clone();
        let queue = self.queues.entry(world).or_default();
        let compatible = queue.iter().position(|candidate| {
            candidate.mode == mode
                && rating.abs_diff(candidate.rating) <= rating_range.min(candidate.rating_range)
        });
        let Some(index) = compatible else {
            queue.push_back(QueueEntry {
                connection_id,
                mode,
                rating,
                rating_range,
            });
            return Ok(vec![deliver(
                connection_id,
                ServerMessage::QueueJoined { mode },
            )]);
        };
        let opponent = queue.remove(index).expect("queue position exists");
        let session_id = Uuid::new_v4();
        self.sessions.insert(
            session_id,
            Session {
                players: [opponent.connection_id, connection_id],
                mode,
                ranked: true,
                reports: HashMap::new(),
                settled: false,
            },
        );
        let opponent_identity = self
            .clients
            .get(&opponent.connection_id)
            .expect("queued client exists")
            .identity
            .clone();
        let local_identity = self
            .clients
            .get(&connection_id)
            .expect("checked")
            .identity
            .clone();
        Ok(vec![
            deliver(
                opponent.connection_id,
                ServerMessage::MatchFound {
                    session_id,
                    mode,
                    opponent_user_id: local_identity.user_id,
                    opponent_display_name: local_identity.display_name,
                    is_host: true,
                },
            ),
            deliver(
                connection_id,
                ServerMessage::MatchFound {
                    session_id,
                    mode,
                    opponent_user_id: opponent_identity.user_id,
                    opponent_display_name: opponent_identity.display_name,
                    is_host: false,
                },
            ),
        ])
    }

    fn relay(
        &self,
        connection_id: Uuid,
        session_id: Uuid,
        payload: serde_json::Value,
    ) -> Result<Vec<Delivery>, String> {
        let bytes = serde_json::to_vec(&payload).map_err(|error| error.to_string())?;
        if bytes.len() > MAX_RELAY_BYTES {
            return Err("relay payload is too large".into());
        }
        let session = self.sessions.get(&session_id).ok_or("unknown session")?;
        let peer = peer_for(session, connection_id)?;
        let user_id = self
            .clients
            .get(&connection_id)
            .expect("checked")
            .identity
            .user_id
            .clone();
        Ok(vec![deliver(
            peer,
            ServerMessage::Relay {
                session_id,
                from_user_id: user_id,
                payload,
            },
        )])
    }

    fn interaction_request(
        &mut self,
        connection_id: Uuid,
        target_user_id: String,
        kind: MatchMode,
    ) -> Result<Vec<Delivery>, String> {
        if self.active_session_for(connection_id).is_some() {
            return Err("requester is already in an active session".into());
        }
        let target = self
            .users
            .get(&target_user_id)
            .copied()
            .ok_or("target is offline")?;
        let from = self.clients.get(&connection_id).expect("checked");
        let target_client = self.clients.get(&target).expect("user index is valid");
        if from.identity.world != target_client.identity.world {
            return Err("target uses a different world or modpack".into());
        }
        if target == connection_id {
            return Err("cannot interact with yourself".into());
        }
        if self.active_session_for(target).is_some() {
            return Err("target is already in an active session".into());
        }
        let from_presence = from
            .presence
            .as_ref()
            .ok_or("requester has no map presence")?;
        let target_presence = target_client
            .presence
            .as_ref()
            .ok_or("target has no map presence")?;
        if !presences_are_visible(from_presence, target_presence) {
            return Err("target is outside presence range".into());
        }
        let request_id = Uuid::new_v4();
        self.interactions.insert(
            request_id,
            PendingInteraction {
                from: connection_id,
                target,
                kind,
            },
        );
        Ok(vec![deliver(
            target,
            ServerMessage::InteractionRequest {
                request_id,
                from_user_id: from.identity.user_id.clone(),
                from_display_name: from.identity.display_name.clone(),
                kind,
            },
        )])
    }

    fn interaction_response(
        &mut self,
        connection_id: Uuid,
        request_id: Uuid,
        target_user_id: String,
        accepted: bool,
    ) -> Result<Vec<Delivery>, String> {
        let request = self
            .interactions
            .get(&request_id)
            .cloned()
            .ok_or("unknown interaction request")?;
        if request.target != connection_id {
            return Err("only the requested player may respond".into());
        }
        let requester = self
            .clients
            .get(&request.from)
            .ok_or("requester is offline")?
            .identity
            .clone();
        if requester.user_id != target_user_id {
            return Err("response target does not match the requester".into());
        }
        let responder = self
            .clients
            .get(&connection_id)
            .expect("checked")
            .identity
            .clone();
        self.interactions.remove(&request_id);
        let mut output = vec![deliver(
            request.from,
            ServerMessage::InteractionResponse {
                request_id,
                from_user_id: responder.user_id.clone(),
                accepted,
            },
        )];
        if accepted {
            if self.active_session_for(request.from).is_some()
                || self.active_session_for(connection_id).is_some()
            {
                return Err("one of the players is already in an active session".into());
            }
            let requester_presence = self
                .clients
                .get(&request.from)
                .and_then(|client| client.presence.as_ref())
                .ok_or("requester has no map presence")?;
            let responder_presence = self
                .clients
                .get(&connection_id)
                .and_then(|client| client.presence.as_ref())
                .ok_or("responder has no map presence")?;
            if !presences_are_visible(requester_presence, responder_presence) {
                return Err("players moved outside presence range".into());
            }
            self.remove_from_queues(request.from);
            self.remove_from_queues(connection_id);
            let session_id = Uuid::new_v4();
            self.sessions.insert(
                session_id,
                Session {
                    players: [request.from, connection_id],
                    mode: request.kind,
                    ranked: false,
                    reports: HashMap::new(),
                    settled: false,
                },
            );
            output.push(deliver(
                request.from,
                ServerMessage::MatchFound {
                    session_id,
                    mode: request.kind,
                    opponent_user_id: responder.user_id,
                    opponent_display_name: responder.display_name,
                    is_host: true,
                },
            ));
            output.push(deliver(
                connection_id,
                ServerMessage::MatchFound {
                    session_id,
                    mode: request.kind,
                    opponent_user_id: requester.user_id,
                    opponent_display_name: requester.display_name,
                    is_host: false,
                },
            ));
        }
        Ok(output)
    }

    fn report_result(
        &mut self,
        connection_id: Uuid,
        session_id: Uuid,
        outcome: MatchOutcome,
    ) -> Result<Vec<Delivery>, String> {
        let (session, winner) = {
            let session = self
                .sessions
                .get_mut(&session_id)
                .ok_or("unknown session")?;
            peer_for(session, connection_id)?;
            if session.settled {
                return Err("session is already settled".into());
            }
            let recorded_outcome = *session.reports.entry(connection_id).or_insert(outcome);
            let peer = peer_for(session, connection_id)?;
            if recorded_outcome == MatchOutcome::Cancelled {
                session.settled = true;
                (session.clone(), None)
            } else {
                let Some(peer_outcome) = session.reports.get(&peer).copied() else {
                    return Ok(vec![deliver(
                        connection_id,
                        ServerMessage::ResultPending { session_id },
                    )]);
                };
                if !outcomes_agree(recorded_outcome, peer_outcome) {
                    return Err("participant result reports conflict".into());
                }
                let winner = match recorded_outcome {
                    MatchOutcome::Local => Some(connection_id),
                    MatchOutcome::Remote => Some(peer),
                    MatchOutcome::Draw | MatchOutcome::Cancelled => None,
                };
                session.settled = true;
                (session.clone(), winner)
            }
        };
        let winner_user_id = winner.and_then(|id| {
            self.clients
                .get(&id)
                .map(|client| client.identity.user_id.clone())
        });
        if session.ranked
            && session.mode == MatchMode::Battle
            && !session
                .reports
                .values()
                .any(|outcome| *outcome == MatchOutcome::Cancelled)
        {
            let first = self
                .clients
                .get(&session.players[0])
                .expect("session client")
                .identity
                .user_id
                .clone();
            let second = self
                .clients
                .get(&session.players[1])
                .expect("session client")
                .identity
                .user_id
                .clone();
            let first_rating = *self.ratings.get(&first).unwrap_or(&1000);
            let second_rating = *self.ratings.get(&second).unwrap_or(&1000);
            let first_score = if winner == Some(session.players[0]) {
                1.0
            } else if winner.is_none() {
                0.5
            } else {
                0.0
            };
            let expected = 1.0 / (1.0 + 10_f64.powf((second_rating - first_rating) as f64 / 400.0));
            let next_first = (first_rating as f64 + 32.0 * (first_score - expected))
                .round()
                .max(100.0) as i32;
            let next_second = (second_rating as f64
                + 32.0 * ((1.0 - first_score) - (1.0 - expected)))
                .round()
                .max(100.0) as i32;
            self.ratings.insert(first, next_first);
            self.ratings.insert(second, next_second);
        }
        self.sessions.remove(&session_id);
        Ok(session
            .players
            .into_iter()
            .map(|id| {
                deliver(
                    id,
                    ServerMessage::ResultSettled {
                        session_id,
                        winner_user_id: winner_user_id.clone(),
                        ranked: session.ranked,
                    },
                )
            })
            .collect())
    }

    fn active_session_for(&self, connection_id: Uuid) -> Option<Uuid> {
        self.sessions.iter().find_map(|(session_id, session)| {
            (!session.settled && session.players.contains(&connection_id)).then_some(*session_id)
        })
    }

    pub fn client_count(&self) -> usize {
        self.clients.len()
    }
    pub fn queued_count(&self) -> usize {
        self.queues.values().map(VecDeque::len).sum()
    }
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }
    pub fn ratings_snapshot(&self) -> HashMap<String, i32> {
        self.ratings.clone()
    }
    pub fn replace_ratings(&mut self, ratings: HashMap<String, i32>) -> Result<(), String> {
        for (user_id, rating) in &ratings {
            validate_token("rating user id", user_id)?;
            if !(100..=5000).contains(rating) {
                return Err(format!("rating for {user_id} is outside 100..=5000"));
            }
        }
        self.ratings = ratings;
        Ok(())
    }

    pub fn binary_relay_target(
        &self,
        connection_id: Uuid,
        session_id: Uuid,
    ) -> Result<Uuid, String> {
        let session = self.sessions.get(&session_id).ok_or("unknown session")?;
        peer_for(session, connection_id)
    }

    fn remove_from_queues(&mut self, connection_id: Uuid) {
        self.queues
            .values_mut()
            .for_each(|queue| queue.retain(|entry| entry.connection_id != connection_id));
        self.queues.retain(|_, queue| !queue.is_empty());
    }

    fn insert_presence_index(
        &mut self,
        world: &WorldIdentity,
        presence: &PresenceRecord,
        connection_id: Uuid,
    ) {
        self.presence_cells
            .entry(presence_cell(world, presence))
            .or_default()
            .insert(connection_id);
    }

    fn remove_presence_index(
        &mut self,
        world: &WorldIdentity,
        presence: &PresenceRecord,
        connection_id: Uuid,
    ) {
        let cell = presence_cell(world, presence);
        if let Some(clients) = self.presence_cells.get_mut(&cell) {
            clients.remove(&connection_id);
            if clients.is_empty() {
                self.presence_cells.remove(&cell);
            }
        }
    }

    fn presence_candidates(
        &self,
        world: &WorldIdentity,
        previous: Option<&PresenceRecord>,
        current: &PresenceRecord,
        exclude: Uuid,
    ) -> HashSet<Uuid> {
        let mut cells = Vec::with_capacity(18);
        if let Some(previous) = previous {
            cells.extend(neighboring_presence_cells(world, previous));
        }
        cells.extend(neighboring_presence_cells(world, current));
        cells
            .into_iter()
            .filter_map(|cell| self.presence_cells.get(&cell))
            .flat_map(|clients| clients.iter().copied())
            .filter(|id| *id != exclude)
            .collect()
    }
}

fn presence_cell(world: &WorldIdentity, presence: &PresenceRecord) -> PresenceCell {
    PresenceCell {
        world: world.clone(),
        map: presence.map.clone(),
        x: presence.tile_x.div_euclid(PRESENCE_CELL_SIZE),
        y: presence.tile_y.div_euclid(PRESENCE_CELL_SIZE),
    }
}

fn neighboring_presence_cells(
    world: &WorldIdentity,
    presence: &PresenceRecord,
) -> impl Iterator<Item = PresenceCell> {
    let center = presence_cell(world, presence);
    (-1..=1).flat_map(move |y| {
        let center = center.clone();
        (-1..=1).map(move |x| PresenceCell {
            world: center.world.clone(),
            map: center.map.clone(),
            x: center.x + x,
            y: center.y + y,
        })
    })
}

fn presences_are_visible(left: &PresenceRecord, right: &PresenceRecord) -> bool {
    left.map == right.map
        && left.tile_x.abs_diff(right.tile_x) <= PRESENCE_RADIUS_X
        && left.tile_y.abs_diff(right.tile_y) <= PRESENCE_RADIUS_Y
}

fn presence_message(identity: &ClientIdentity, presence: &PresenceRecord) -> ServerMessage {
    ServerMessage::Presence {
        user_id: identity.user_id.clone(),
        display_name: identity.display_name.clone(),
        map: presence.map.clone(),
        tile_x: presence.tile_x,
        tile_y: presence.tile_y,
        direction: presence.direction.clone(),
    }
}

fn peer_for(session: &Session, connection_id: Uuid) -> Result<Uuid, String> {
    if session.players[0] == connection_id {
        Ok(session.players[1])
    } else if session.players[1] == connection_id {
        Ok(session.players[0])
    } else {
        Err("connection is not a session participant".into())
    }
}

fn outcomes_agree(local: MatchOutcome, peer: MatchOutcome) -> bool {
    matches!(
        (local, peer),
        (MatchOutcome::Local, MatchOutcome::Remote)
            | (MatchOutcome::Remote, MatchOutcome::Local)
            | (MatchOutcome::Draw, MatchOutcome::Draw)
    )
}

fn deliver(connection_id: Uuid, message: ServerMessage) -> Delivery {
    Delivery {
        connection_id,
        message,
    }
}

fn validate_identity(identity: &ClientIdentity) -> Result<(), String> {
    validate_token("user id", &identity.user_id)?;
    if identity.display_name.is_empty() || identity.display_name.len() > MAX_NAME_BYTES {
        return Err("invalid display name".into());
    }
    validate_token("world id", &identity.world.world_id)?;
    validate_token("modpack id", &identity.world.modpack.id)?;
    validate_token("modpack content hash", &identity.world.modpack.content_hash)
}

fn validate_token(label: &str, value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > MAX_TOKEN_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(format!("invalid {label}"));
    }
    Ok(())
}

pub fn validate_public_token(label: &str, value: &str) -> Result<(), String> {
    validate_token(label, value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(user: &str, pack: &str) -> ClientIdentity {
        ClientIdentity {
            user_id: user.into(),
            display_name: user.into(),
            world: WorldIdentity {
                world_id: "main".into(),
                modpack: ModpackIdentity {
                    id: pack.into(),
                    content_hash: format!("{pack}.hash"),
                },
            },
        }
    }

    #[test]
    fn cancelling_ranked_match_preserves_unequal_ratings() {
        let mut hub = Hub::default();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        hub.connect(a, identity("a", "core")).unwrap();
        hub.connect(b, identity("b", "core")).unwrap();
        let ratings = HashMap::from([("a".into(), 1100), ("b".into(), 1000)]);
        hub.replace_ratings(ratings.clone()).unwrap();
        hub.handle(
            a,
            ClientMessage::QueueJoin {
                mode: MatchMode::Battle,
                rating: 1100,
                rating_range: 200,
            },
        );
        let found = hub.handle(
            b,
            ClientMessage::QueueJoin {
                mode: MatchMode::Battle,
                rating: 1000,
                rating_range: 200,
            },
        );
        let ServerMessage::MatchFound { session_id, .. } = found[0].message else {
            panic!("match");
        };
        let settled = hub.handle(
            a,
            ClientMessage::Result {
                session_id,
                outcome: MatchOutcome::Cancelled,
            },
        );
        assert_eq!(settled.len(), 2);
        assert_eq!(hub.session_count(), 0);
        assert_eq!(hub.ratings_snapshot(), ratings);
    }

    #[test]
    fn matchmaking_is_exactly_partitioned_by_modpack() {
        let mut hub = Hub::default();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        hub.connect(a, identity("a", "core")).unwrap();
        hub.connect(b, identity("b", "gen3")).unwrap();
        hub.connect(c, identity("c", "core")).unwrap();
        assert!(matches!(
            hub.handle(
                a,
                ClientMessage::QueueJoin {
                    mode: MatchMode::Battle,
                    rating: 1000,
                    rating_range: 100
                }
            )[0]
            .message,
            ServerMessage::QueueJoined { .. }
        ));
        assert!(matches!(
            hub.handle(
                b,
                ClientMessage::QueueJoin {
                    mode: MatchMode::Battle,
                    rating: 1000,
                    rating_range: 100
                }
            )[0]
            .message,
            ServerMessage::QueueJoined { .. }
        ));
        let matched = hub.handle(
            c,
            ClientMessage::QueueJoin {
                mode: MatchMode::Battle,
                rating: 1000,
                rating_range: 100,
            },
        );
        assert_eq!(matched.len(), 2);
        assert!(
            matched
                .iter()
                .all(|delivery| matches!(delivery.message, ServerMessage::MatchFound { .. }))
        );
        assert_eq!(hub.queued_count(), 1);
    }

    #[test]
    fn presence_ghosts_are_partitioned_by_world_modpack_and_map() {
        let mut hub = Hub::default();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let other_pack = Uuid::new_v4();
        let other_map = Uuid::new_v4();
        hub.connect(a, identity("a", "core")).unwrap();
        hub.connect(b, identity("b", "core")).unwrap();
        hub.connect(other_pack, identity("other-pack", "modded"))
            .unwrap();
        hub.connect(other_map, identity("other-map", "core"))
            .unwrap();

        let presence = |map: &str, tile_x| ClientMessage::Presence {
            map: map.into(),
            tile_x,
            tile_y: 8,
            direction: "down".into(),
        };
        assert!(hub.handle(a, presence("GoldenrodCity", 4)).is_empty());
        assert!(
            hub.handle(other_pack, presence("GoldenrodCity", 5))
                .is_empty()
        );
        assert!(hub.handle(other_map, presence("Route34", 6)).is_empty());

        let same_map = hub.handle(b, presence("GoldenrodCity", 7));
        assert_eq!(same_map.len(), 2);
        assert!(same_map.iter().any(|delivery| {
            delivery.connection_id == a
                && matches!(
                    &delivery.message,
                    ServerMessage::Presence { user_id, tile_x: 7, .. } if user_id == "b"
                )
        }));
        assert!(same_map.iter().any(|delivery| {
            delivery.connection_id == b
                && matches!(
                    &delivery.message,
                    ServerMessage::Presence { user_id, tile_x: 4, .. } if user_id == "a"
                )
        }));
        assert!(same_map.iter().all(|delivery| {
            delivery.connection_id != other_pack && delivery.connection_id != other_map
        }));

        let changed_map = hub.handle(a, presence("Route34", 9));
        assert!(changed_map.iter().any(|delivery| {
            delivery.connection_id == b
                && matches!(
                    &delivery.message,
                    ServerMessage::PresenceLeft { user_id } if user_id == "a"
                )
        }));
        assert!(changed_map.iter().any(|delivery| {
            delivery.connection_id == other_map
                && matches!(
                    &delivery.message,
                    ServerMessage::Presence { user_id, tile_x: 9, .. } if user_id == "a"
                )
        }));
    }

    #[test]
    fn presence_interest_is_spatial_and_emits_leave_at_the_boundary() {
        let mut hub = Hub::default();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        hub.connect(a, identity("a", "core")).unwrap();
        hub.connect(b, identity("b", "core")).unwrap();
        let at = |tile_x| ClientMessage::Presence {
            map: "GoldenrodCity".into(),
            tile_x,
            tile_y: 8,
            direction: "right".into(),
        };
        assert!(hub.handle(a, at(0)).is_empty());
        assert!(hub.handle(b, at(100)).is_empty());
        let entered = hub.handle(b, at(PRESENCE_RADIUS_X as i32));
        assert_eq!(entered.len(), 2);
        let left = hub.handle(b, at(PRESENCE_RADIUS_X as i32 + 1));
        assert_eq!(left.len(), 2);
        assert!(
            left.iter()
                .all(|delivery| { matches!(delivery.message, ServerMessage::PresenceLeft { .. }) })
        );
    }

    #[test]
    fn relay_and_two_party_settlement_require_session_participants() {
        let mut hub = Hub::default();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let outsider = Uuid::new_v4();
        for (id, name) in [(a, "a"), (b, "b"), (outsider, "outsider")] {
            hub.connect(id, identity(name, "core")).unwrap();
        }
        hub.handle(
            a,
            ClientMessage::QueueJoin {
                mode: MatchMode::Battle,
                rating: 1000,
                rating_range: 100,
            },
        );
        let found = hub.handle(
            b,
            ClientMessage::QueueJoin {
                mode: MatchMode::Battle,
                rating: 1000,
                rating_range: 100,
            },
        );
        let ServerMessage::MatchFound { session_id, .. } = found[0].message else {
            panic!("match")
        };
        assert!(matches!(
            hub.handle(
                outsider,
                ClientMessage::Relay {
                    session_id,
                    payload: serde_json::json!({"x": 1})
                }
            )[0]
            .message,
            ServerMessage::Error { .. }
        ));
        assert!(matches!(
            hub.handle(
                a,
                ClientMessage::Result {
                    session_id,
                    outcome: MatchOutcome::Local
                }
            )[0]
            .message,
            ServerMessage::ResultPending { .. }
        ));
        let settled = hub.handle(
            b,
            ClientMessage::Result {
                session_id,
                outcome: MatchOutcome::Remote,
            },
        );
        assert_eq!(settled.len(), 2);
        assert!(
            settled
                .iter()
                .all(|delivery| matches!(delivery.message, ServerMessage::ResultSettled { .. }))
        );
        assert_eq!(hub.session_count(), 0);
    }

    #[test]
    fn direct_interaction_acceptance_creates_an_unranked_session() {
        let mut hub = Hub::default();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        hub.connect(a, identity("a", "core")).unwrap();
        hub.connect(b, identity("b", "core")).unwrap();
        for id in [a, b] {
            hub.handle(
                id,
                ClientMessage::Presence {
                    map: "GoldenrodCity".into(),
                    tile_x: 4,
                    tile_y: 8,
                    direction: "down".into(),
                },
            );
        }
        let request = hub.handle(
            a,
            ClientMessage::InteractionRequest {
                target_user_id: "b".into(),
                kind: MatchMode::Trade,
            },
        );
        let ServerMessage::InteractionRequest { request_id, .. } = request[0].message else {
            panic!("request")
        };
        let accepted = hub.handle(
            b,
            ClientMessage::InteractionResponse {
                request_id,
                target_user_id: "a".into(),
                accepted: true,
            },
        );
        assert_eq!(accepted.len(), 3);
        let sessions = accepted
            .iter()
            .filter_map(|delivery| match delivery.message {
                ServerMessage::MatchFound { session_id, .. } => Some(session_id),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0], sessions[1]);
        assert_eq!(hub.session_count(), 1);
    }

    #[test]
    fn direct_interactions_require_same_map_and_idle_players() {
        let mut hub = Hub::default();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        hub.connect(a, identity("a", "core")).unwrap();
        hub.connect(b, identity("b", "core")).unwrap();
        for (id, map) in [(a, "GoldenrodCity"), (b, "Route34")] {
            hub.handle(
                id,
                ClientMessage::Presence {
                    map: map.into(),
                    tile_x: 4,
                    tile_y: 8,
                    direction: "down".into(),
                },
            );
        }
        assert!(matches!(
            hub.handle(
                a,
                ClientMessage::InteractionRequest {
                    target_user_id: "b".into(),
                    kind: MatchMode::TimeCapsule,
                }
            )[0]
            .message,
            ServerMessage::Error { .. }
        ));
    }

    #[test]
    fn matchmaking_requires_both_players_rating_ranges() {
        let mut hub = Hub::default();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        hub.connect(a, identity("a", "core")).unwrap();
        hub.connect(b, identity("b", "core")).unwrap();
        hub.replace_ratings(HashMap::from([("a".into(), 1000), ("b".into(), 1080)]))
            .unwrap();
        hub.handle(
            a,
            ClientMessage::QueueJoin {
                mode: MatchMode::Battle,
                rating: 1000,
                rating_range: 25,
            },
        );
        let result = hub.handle(
            b,
            ClientMessage::QueueJoin {
                mode: MatchMode::Battle,
                rating: 1080,
                rating_range: 100,
            },
        );
        assert!(matches!(
            result[0].message,
            ServerMessage::QueueJoined { .. }
        ));
        assert_eq!(hub.queued_count(), 2);
    }

    #[test]
    fn disconnect_cancels_the_peer_session() {
        let mut hub = Hub::default();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        hub.connect(a, identity("a", "core")).unwrap();
        hub.connect(b, identity("b", "core")).unwrap();
        hub.handle(
            a,
            ClientMessage::QueueJoin {
                mode: MatchMode::Battle,
                rating: 1000,
                rating_range: 100,
            },
        );
        hub.handle(
            b,
            ClientMessage::QueueJoin {
                mode: MatchMode::Battle,
                rating: 1000,
                rating_range: 100,
            },
        );
        let disconnected = hub.disconnect(a);
        assert!(
            disconnected
                .iter()
                .any(|delivery| delivery.connection_id == b
                    && matches!(
                        delivery.message,
                        ServerMessage::ResultSettled {
                            winner_user_id: None,
                            ..
                        }
                    ))
        );
        assert_eq!(hub.session_count(), 0);
    }
}
