struct MultiplayerRuntime {
    connection: Option<crystal_net::hosted::HostedConnection>,
    session: Option<crystal_net::hosted::HostedLinkSession>,
    config: BevyMultiplayerConfig,
    queued_mode: Option<crystal_net::hosted::MatchMode>,
    match_mode: Option<crystal_net::hosted::MatchMode>,
    result_reported: bool,
    session_settled: bool,
    direct_mode: Option<crystal_net::hosted::MatchMode>,
    direct_session: bool,
    pending_interaction: Option<IncomingInteraction>,
    last_presence: Option<(String, i16, i16, crate::core::world::map::Direction)>,
    presence_frames_since_send: u16,
    remote_presences: HashMap<String, RemotePresence>,
    player_id: u64,
    peer_player_id: Option<u64>,
    peer_player_name: Option<String>,
    trade_id_prefix: String,
    trade_sequence: u64,
    owns_internal_clock: bool,
    last_sent_link_room: Option<u8>,
    remote_link_room: Option<u8>,
    game_link_ready: bool,
    party_sent: bool,
    remote_party: Option<(u64, crate::core::models::Party)>,
    active_trade: Option<TradeSyncBuffer>,
    peer_trade_offers: VecDeque<TradeOffer>,
    peer_trade_confirmations: VecDeque<TradeConfirmation>,
    link_battle_random: Option<crate::core::random::LinkBattleRandomState>,
    link_battle_random_sent: bool,
    link_battle_started: bool,
    failed: bool,
    reconnect_frames: u16,
    reconnect_attempt: u8,
    sent_input_count: usize,
    sent_battle_action_count: usize,
    sent_menu_result_count: usize,
    peer_inputs: VecDeque<PlayerInputFrame>,
    peer_battle_actions: VecDeque<BattleActionFrame>,
    peer_menu_results: VecDeque<MenuChoiceResultFrame>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RemotePresence {
    display_name: String,
    map: String,
    tile_x: i16,
    tile_y: i16,
    direction: String,
}

#[derive(Debug, Clone)]
struct IncomingInteraction {
    request_id: uuid::Uuid,
    from_user_id: String,
    from_display_name: String,
    kind: crystal_net::hosted::MatchMode,
}

impl MultiplayerRuntime {
    fn new(runtime_shell: &BevyRuntimeShell, config: BevyMultiplayerConfig) -> Result<Self> {
        let connection = Self::connect(runtime_shell, &config)?;
        Ok(Self {
            connection: Some(connection),
            session: None,
            config: config.clone(),
            queued_mode: None,
            match_mode: None,
            result_reported: false,
            session_settled: false,
            direct_mode: None,
            direct_session: false,
            pending_interaction: None,
            last_presence: None,
            presence_frames_since_send: 0,
            remote_presences: HashMap::new(),
            player_id: config.player_id,
            peer_player_id: None,
            peer_player_name: None,
            trade_id_prefix: "pending".into(),
            trade_sequence: 1,
            owns_internal_clock: false,
            last_sent_link_room: None,
            remote_link_room: None,
            game_link_ready: false,
            party_sent: false,
            remote_party: None,
            active_trade: None,
            peer_trade_offers: VecDeque::new(),
            peer_trade_confirmations: VecDeque::new(),
            link_battle_random: None,
            link_battle_random_sent: false,
            link_battle_started: false,
            failed: false,
            reconnect_frames: 0,
            reconnect_attempt: 0,
            sent_input_count: 0,
            sent_battle_action_count: 0,
            sent_menu_result_count: 0,
            peer_inputs: VecDeque::new(),
            peer_battle_actions: VecDeque::new(),
            peer_menu_results: VecDeque::new(),
        })
    }

    fn connect(
        runtime_shell: &BevyRuntimeShell,
        config: &BevyMultiplayerConfig,
    ) -> Result<crystal_net::hosted::HostedConnection> {
        let pack = runtime_shell.shell.runtime().pack_identity();
        let identity = crystal_net::hosted::ClientIdentity {
            user_id: format!("player-{}", config.player_id),
            display_name: config.display_name.clone(),
            world: crystal_net::hosted::WorldIdentity {
                world_id: config.world_id.clone(),
                modpack: crystal_net::hosted::ModpackIdentity {
                    id: pack.runtime_modpack_id.clone(),
                    content_hash: pack.content_hash.clone(),
                },
            },
        };
        crystal_net::hosted::HostedConnection::connect(
            config.server_url.clone(),
            config.server_token.as_deref(),
            identity,
        )
        .context("connect to hosted multiplayer server")
    }

    fn poll(
        &mut self,
        runtime_shell: &mut BevyRuntimeShell,
        keys: &mut ButtonInput<KeyCode>,
    ) -> Result<()> {
        if self.failed {
            return self.poll_reconnect(runtime_shell);
        }
        self.publish_presence(runtime_shell)?;
        self.handle_interaction_input(runtime_shell, keys)?;
        let mut matched = None;
        let mut lobby_messages = Vec::new();
        if let Some(connection) = self.connection.as_mut() {
            let requested_mode = requested_match_mode(
                runtime_shell.shell.session().state(),
                runtime_shell.pending_link_room_selection,
            );
            if requested_mode != self.queued_mode {
                match requested_mode {
                    Some(mode) => connection
                        .join_queue(mode, self.config.rating, self.config.rating_range)
                        .context("join hosted multiplayer queue")?,
                    None => connection
                        .send(crystal_net::hosted::ClientMessage::QueueLeave)
                        .context("leave hosted multiplayer queue")?,
                }
                self.queued_mode = requested_mode;
            }
            lobby_messages = connection.poll().context("poll hosted multiplayer lobby")?;
        }
        for message in lobby_messages {
            match message {
                crystal_net::hosted::ServerMessage::Welcome { .. } => {
                    runtime_shell.last_action_status =
                        Some("Connected to multiplayer server".into());
                }
                crystal_net::hosted::ServerMessage::QueueJoined { mode } => {
                    runtime_shell.last_action_status =
                        Some(format!("Waiting for {mode:?} partner"));
                }
                crystal_net::hosted::ServerMessage::MatchFound {
                    session_id,
                    mode,
                    opponent_display_name,
                    is_host,
                    ..
                } => {
                    if Some(mode) != self.queued_mode && Some(mode) != self.direct_mode {
                        anyhow::bail!("hosted server returned a different match mode");
                    }
                    matched = Some((session_id, mode, opponent_display_name, is_host));
                }
                message => self.handle_server_message(message)?,
            }
        }
        if let Some((session_id, mode, opponent_display_name, is_host)) = matched {
            let direct_session = self.direct_mode == Some(mode) && self.queued_mode != Some(mode);
            let descriptor = runtime_shell
                .shell
                .link_session_descriptor(
                    session_id.to_string(),
                    self.config.player_id,
                    self.config.display_name.clone(),
                )
                .context("build hosted Bevy multiplayer session descriptor")?;
            let connection = self
                .connection
                .take()
                .context("matched hosted connection is missing")?;
            self.session = Some(
                crystal_net::hosted::HostedLinkSession::new(
                    connection,
                    session_id,
                    descriptor.hello,
                    descriptor.save_checkpoint,
                )
                .context("start hosted deterministic link session")?,
            );
            self.trade_id_prefix = session_id.to_string();
            self.match_mode = Some(mode);
            self.result_reported = false;
            self.session_settled = false;
            self.direct_session = direct_session;
            self.direct_mode = None;
            self.pending_interaction = None;
            self.owns_internal_clock = is_host;
            runtime_shell.last_action_status =
                Some(format!("Matched with {opponent_display_name}"));
        }
        if self.session.is_some() {
            let (events, messages) = {
                let session = self
                    .session
                    .as_mut()
                    .expect("checked hosted multiplayer session disappeared");
                let events = session
                    .poll()
                    .context("poll hosted Bevy multiplayer session")?;
                let messages = session.drain_server_messages();
                (events, messages)
            };
            for event in events {
                self.handle_event(runtime_shell, event)?;
            }
            for message in messages {
                self.handle_server_message(message)?;
            }
        }
        if self.session_settled {
            if !self.result_reported {
                self.cancel_interrupted_gameplay(runtime_shell)?;
            }
            self.return_to_lobby(runtime_shell)?;
            return Ok(());
        }
        if self
            .session
            .as_ref()
            .is_some_and(crystal_net::hosted::HostedLinkSession::is_ready_for_gameplay)
        {
            self.send_local_party(runtime_shell)?;
            self.send_link_room_selection(runtime_shell)?;
            self.synchronize_game_link_state(runtime_shell)?;
            self.advance_trade_center(runtime_shell)?;
            self.advance_colosseum(runtime_shell)?;
            self.advance_colosseum_turn(runtime_shell)?;
            self.advance_colosseum_replacement(runtime_shell)?;
            self.finish_colosseum_if_terminal(runtime_shell)?;
            self.send_pending_inputs(runtime_shell)?;
            self.send_pending_battle_actions(runtime_shell)?;
            self.send_pending_menu_results(runtime_shell)?;
        }
        Ok(())
    }

    fn publish_presence(&mut self, runtime_shell: &BevyRuntimeShell) -> Result<()> {
        const PRESENCE_HEARTBEAT_FRAMES: u16 = 30;
        let snapshot = runtime_shell.shell.session().snapshot();
        let presence = (
            snapshot.map_name.clone(),
            snapshot.tile.x,
            snapshot.tile.y,
            snapshot.facing,
        );
        self.presence_frames_since_send = self.presence_frames_since_send.saturating_add(1);
        if self.last_presence.as_ref() == Some(&presence)
            && self.presence_frames_since_send < PRESENCE_HEARTBEAT_FRAMES
        {
            return Ok(());
        }
        let direction = hosted_direction(snapshot.facing);
        if let Some(connection) = self.connection.as_mut() {
            connection
                .update_presence(
                    snapshot.map_name,
                    i32::from(snapshot.tile.x),
                    i32::from(snapshot.tile.y),
                    direction,
                )
                .context("publish hosted overworld presence")?;
        } else if let Some(session) = self.session.as_mut() {
            session
                .update_presence(
                    snapshot.map_name,
                    i32::from(snapshot.tile.x),
                    i32::from(snapshot.tile.y),
                    direction,
                )
                .context("publish hosted overworld presence during link session")?;
        } else {
            return Ok(());
        }
        self.last_presence = Some(presence);
        self.presence_frames_since_send = 0;
        Ok(())
    }

    fn handle_server_message(&mut self, message: crystal_net::hosted::ServerMessage) -> Result<()> {
        match message {
            crystal_net::hosted::ServerMessage::Presence {
                user_id,
                display_name,
                map,
                tile_x,
                tile_y,
                direction,
            } => {
                let tile_x = i16::try_from(tile_x).context("remote presence tile_x is invalid")?;
                let tile_y = i16::try_from(tile_y).context("remote presence tile_y is invalid")?;
                self.remote_presences.insert(
                    user_id,
                    RemotePresence {
                        display_name,
                        map,
                        tile_x,
                        tile_y,
                        direction,
                    },
                );
            }
            crystal_net::hosted::ServerMessage::PresenceLeft { user_id } => {
                self.remote_presences.remove(&user_id);
            }
            crystal_net::hosted::ServerMessage::InteractionRequest {
                request_id,
                from_user_id,
                from_display_name,
                kind,
            } => {
                self.pending_interaction = Some(IncomingInteraction {
                    request_id,
                    from_user_id,
                    from_display_name,
                    kind,
                });
            }
            crystal_net::hosted::ServerMessage::InteractionResponse { accepted, .. } => {
                if !accepted {
                    self.direct_mode = None;
                }
            }
            crystal_net::hosted::ServerMessage::ResultSettled { .. } => {
                self.session_settled = true;
            }
            crystal_net::hosted::ServerMessage::Error { code, message } => {
                anyhow::bail!("hosted multiplayer server error {code}: {message}");
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_interaction_input(
        &mut self,
        runtime_shell: &mut BevyRuntimeShell,
        keys: &mut ButtonInput<KeyCode>,
    ) -> Result<()> {
        if let Some(request) = self.pending_interaction.clone() {
            let accepted = if keys.just_pressed(KeyCode::KeyZ) {
                Some(true)
            } else if keys.just_pressed(KeyCode::KeyX) {
                Some(false)
            } else {
                runtime_shell.last_action_status = Some(format!(
                    "{} requests {:?}: Z accept / X decline",
                    request.from_display_name, request.kind
                ));
                return Ok(());
            };
            let accepted = accepted.expect("interaction response key was selected");
            self.connection
                .as_mut()
                .context("incoming interaction has no lobby connection")?
                .send(crystal_net::hosted::ClientMessage::InteractionResponse {
                    request_id: request.request_id,
                    target_user_id: request.from_user_id,
                    accepted,
                })
                .context("respond to hosted player interaction")?;
            self.direct_mode = accepted.then_some(request.kind);
            self.pending_interaction = None;
            keys.clear_just_pressed(KeyCode::KeyZ);
            keys.clear_just_pressed(KeyCode::KeyX);
            runtime_shell.last_action_status = Some(if accepted {
                "Challenge accepted".into()
            } else {
                "Challenge declined".into()
            });
            return Ok(());
        }
        if self.session.is_some() || self.queued_mode.is_some() || self.direct_mode.is_some() {
            return Ok(());
        }
        let mode = if keys.just_pressed(KeyCode::KeyC) {
            Some(crystal_net::hosted::MatchMode::Battle)
        } else if keys.just_pressed(KeyCode::KeyV) {
            Some(crystal_net::hosted::MatchMode::Trade)
        } else if keys.just_pressed(KeyCode::KeyT) {
            Some(crystal_net::hosted::MatchMode::TimeCapsule)
        } else {
            None
        };
        let Some(mode) = mode else {
            return Ok(());
        };
        let snapshot = runtime_shell.shell.session().snapshot();
        let target_tile = snapshot.tile.moved(snapshot.facing);
        let target = self.remote_presences.iter().find(|(_, presence)| {
            presence.map == snapshot.map_name
                && presence.tile_x == target_tile.x
                && presence.tile_y == target_tile.y
        });
        let Some((target_user_id, target)) = target else {
            runtime_shell.last_action_status = Some("No player ghost is directly ahead".into());
            return Ok(());
        };
        self.connection
            .as_mut()
            .context("player interaction has no lobby connection")?
            .send(crystal_net::hosted::ClientMessage::InteractionRequest {
                target_user_id: target_user_id.clone(),
                kind: mode,
            })
            .context("request hosted player interaction")?;
        self.direct_mode = Some(mode);
        runtime_shell.last_action_status =
            Some(format!("Sent {mode:?} request to {}", target.display_name));
        Ok(())
    }

    fn report_result(&mut self, outcome: crystal_net::hosted::MatchOutcome) -> Result<()> {
        if self.result_reported {
            return Ok(());
        }
        self.session
            .as_mut()
            .context("cannot report a result without a hosted session")?
            .report_result(outcome)
            .context("report hosted multiplayer result")?;
        self.result_reported = true;
        Ok(())
    }

    fn return_to_lobby(&mut self, runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
        if let Some(session) = self.session.as_mut() {
            session.disconnect();
        }
        mark_link_disconnected(runtime_shell.shell.session_mut().state_mut());
        self.connection = Some(Self::connect(runtime_shell, &self.config)?);
        self.session = None;
        self.queued_mode = None;
        self.match_mode = None;
        self.result_reported = false;
        self.session_settled = false;
        self.direct_mode = None;
        self.direct_session = false;
        self.pending_interaction = None;
        self.last_presence = None;
        self.remote_presences.clear();
        self.peer_player_id = None;
        self.peer_player_name = None;
        self.last_sent_link_room = None;
        self.remote_link_room = None;
        self.game_link_ready = false;
        self.party_sent = false;
        self.remote_party = None;
        self.active_trade = None;
        self.peer_trade_offers.clear();
        self.peer_trade_confirmations.clear();
        self.link_battle_random = None;
        self.link_battle_random_sent = false;
        self.link_battle_started = false;
        self.sent_input_count = 0;
        self.sent_battle_action_count = 0;
        self.sent_menu_result_count = 0;
        self.peer_inputs.clear();
        self.peer_battle_actions.clear();
        self.peer_menu_results.clear();
        self.failed = false;
        self.reconnect_frames = 0;
        self.reconnect_attempt = 0;
        runtime_shell.last_action_status = Some("Multiplayer session complete".into());
        mark_runtime_snapshot_dirty(runtime_shell);
        Ok(())
    }

    fn handle_event(
        &mut self,
        runtime_shell: &mut BevyRuntimeShell,
        event: crystal_net::hosted::HostedLinkSessionEvent,
    ) -> Result<()> {
        match event {
            crystal_net::hosted::HostedLinkSessionEvent::GameplayReady => {
                runtime_shell.last_action_status = Some("Multiplayer ready".to_string());
                runtime_shell
                    .last_audio_events
                    .push("multiplayer gameplay ready".to_string());
                trim_event_log(&mut runtime_shell.last_audio_events);
            }
            crystal_net::hosted::HostedLinkSessionEvent::Endpoint(event) => {
                self.handle_endpoint_event(runtime_shell, event)?;
            }
        }
        Ok(())
    }

    fn handle_endpoint_event(
        &mut self,
        runtime_shell: &mut BevyRuntimeShell,
        event: crystal_net::LinkEndpointEvent,
    ) -> Result<()> {
        match event {
            crystal_net::LinkEndpointEvent::PeerHello(hello) => {
                self.peer_player_id = Some(hello.player().id());
                self.peer_player_name = Some(hello.player().display_name().to_string());
                runtime_shell.last_action_status = Some(format!(
                    "Multiplayer peer: {}",
                    hello.player().display_name()
                ));
            }
            crystal_net::LinkEndpointEvent::PeerSaveCheckpoint { player_id, .. } => {
                runtime_shell.last_audio_events.push(format!(
                    "multiplayer checkpoint received player={player_id}"
                ));
                trim_event_log(&mut runtime_shell.last_audio_events);
            }
            crystal_net::LinkEndpointEvent::PeerMenuChoice(choice) => {
                if choice.menu_id() == "cable_club_room" {
                    let room = u8::try_from(choice.option_index())
                        .context("peer Cable Club room does not fit the link mode byte")?;
                    self.remote_link_room = Some(room);
                }
                runtime_shell.last_action_status = Some(format!(
                    "Peer selected {} option {}",
                    choice.menu_id(),
                    choice.option_index()
                ));
            }
            crystal_net::LinkEndpointEvent::PeerMenuChoiceResult(result) => {
                self.peer_menu_results.push_back(result);
                trim_multiplayer_queue(&mut self.peer_menu_results);
            }
            crystal_net::LinkEndpointEvent::PeerParty(party) => {
                self.receive_peer_party(party)?;
            }
            crystal_net::LinkEndpointEvent::PeerBattleRng(frame) => {
                self.receive_link_battle_random(frame)?;
            }
            crystal_net::LinkEndpointEvent::Message(message) => match message {
                LinkMessage::Input(input) => {
                    self.peer_inputs.push_back(input);
                    trim_multiplayer_queue(&mut self.peer_inputs);
                }
                LinkMessage::SessionInput(input) => {
                    self.peer_inputs.push_back(input.input().clone());
                    trim_multiplayer_queue(&mut self.peer_inputs);
                }
                LinkMessage::BattleAction(action) => {
                    self.peer_battle_actions.push_back(action);
                    trim_multiplayer_queue(&mut self.peer_battle_actions);
                }
                LinkMessage::SessionBattleAction(action) => {
                    self.peer_battle_actions.push_back(action.action().clone());
                    trim_multiplayer_queue(&mut self.peer_battle_actions);
                }
                LinkMessage::TradeOffer(offer) => {
                    self.peer_trade_offers.push_back(offer);
                }
                LinkMessage::SessionTradeOffer(offer) => {
                    self.peer_trade_offers.push_back(offer.offer().clone());
                }
                LinkMessage::TradeConfirmation(confirmation) => {
                    self.peer_trade_confirmations.push_back(confirmation);
                }
                LinkMessage::SessionTradeConfirmation(confirmation) => {
                    self.peer_trade_confirmations
                        .push_back(confirmation.confirmation().clone());
                }
                LinkMessage::Party(party) => self.receive_peer_party(party)?,
                LinkMessage::SessionParty(party) => {
                    self.receive_peer_party(party.party().clone())?
                }
                LinkMessage::LinkBattleRngInit(frame) => self.receive_link_battle_random(frame)?,
                LinkMessage::MenuChoiceResult(result) => {
                    self.peer_menu_results.push_back(result);
                    trim_multiplayer_queue(&mut self.peer_menu_results);
                }
                LinkMessage::SessionMenuChoiceResult(result) => {
                    self.peer_menu_results.push_back(result.result().clone());
                    trim_multiplayer_queue(&mut self.peer_menu_results);
                }
                LinkMessage::Disconnect { player_id, reason } => {
                    mark_link_disconnected(runtime_shell.shell.session_mut().state_mut());
                    self.game_link_ready = false;
                    self.session_settled = true;
                    runtime_shell.last_action_status =
                        Some(format!("Player {player_id} disconnected: {reason}"));
                }
                LinkMessage::SessionDisconnect(disconnect) => {
                    mark_link_disconnected(runtime_shell.shell.session_mut().state_mut());
                    self.game_link_ready = false;
                    self.session_settled = true;
                    runtime_shell.last_action_status = Some(format!(
                        "Player {} disconnected: {}",
                        disconnect.player_id(),
                        disconnect.reason()
                    ));
                }
                _ => {}
            },
        }
        Ok(())
    }

    fn cancel_interrupted_gameplay(&mut self, runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
        let active_link_battle = matches!(
            &runtime_shell.shell.session().state().battle,
            crate::core::state::BattleMemory::Trainer { battle_type, .. }
                if battle_type == "BATTLETYPE_LINK"
        );
        if active_link_battle {
            runtime_shell
                .shell
                .finish_link_battle(RuntimeLinkBattleResult::Draw)
                .context("close interrupted link battle")?;
            reset_visible_battle_exit_state(runtime_shell);
        }
        mark_link_disconnected(runtime_shell.shell.session_mut().state_mut());
        cancel_visible_online_link_flow(runtime_shell)
    }

    fn mark_disconnected(&mut self, runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
        if let Some(session) = self.session.as_mut() {
            session.disconnect();
        }
        self.connection = None;
        self.session = None;
        self.queued_mode = None;
        self.match_mode = None;
        self.result_reported = false;
        self.session_settled = false;
        self.direct_mode = None;
        self.direct_session = false;
        self.pending_interaction = None;
        self.remote_presences.clear();
        self.last_presence = None;
        self.peer_player_id = None;
        self.peer_player_name = None;
        self.last_sent_link_room = None;
        self.remote_link_room = None;
        self.party_sent = false;
        self.remote_party = None;
        self.active_trade = None;
        self.peer_trade_offers.clear();
        self.peer_trade_confirmations.clear();
        self.link_battle_random = None;
        self.link_battle_random_sent = false;
        self.link_battle_started = false;
        self.peer_inputs.clear();
        self.peer_battle_actions.clear();
        self.peer_menu_results.clear();
        self.sent_input_count = 0;
        self.sent_battle_action_count = 0;
        self.sent_menu_result_count = 0;
        let cleanup_result = self.cancel_interrupted_gameplay(runtime_shell);
        self.game_link_ready = false;
        self.failed = true;
        self.reconnect_frames = 60;
        self.reconnect_attempt = 0;
        runtime_shell.last_action_status = Some("Multiplayer disconnected; reconnecting".into());
        mark_runtime_snapshot_dirty(runtime_shell);
        cleanup_result
    }

    fn poll_reconnect(&mut self, runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
        if self.reconnect_frames > 0 {
            self.reconnect_frames -= 1;
            return Ok(());
        }
        match Self::connect(runtime_shell, &self.config) {
            Ok(connection) => {
                self.connection = Some(connection);
                self.failed = false;
                self.reconnect_attempt = 0;
                self.last_presence = None;
                runtime_shell.last_action_status = Some("Reconnected to multiplayer".into());
            }
            Err(error) => {
                self.reconnect_attempt = self.reconnect_attempt.saturating_add(1).min(5);
                self.reconnect_frames = 60_u16.saturating_mul(
                    1_u16
                        .checked_shl(u32::from(self.reconnect_attempt))
                        .unwrap_or(32),
                );
                runtime_shell.last_action_status =
                    Some(format!("Multiplayer reconnect failed: {error}; retrying"));
            }
        }
        Ok(())
    }

    fn send_link_room_selection(&mut self, runtime_shell: &BevyRuntimeShell) -> Result<()> {
        let state = runtime_shell.shell.session().state();
        let selected_room =
            selected_cable_club_room(state, runtime_shell.pending_link_room_selection).or_else(
                || {
                    self.direct_session
                        .then(|| self.match_mode.map(match_mode_room))
                        .flatten()
                },
            );
        let Some(room) = selected_room else {
            return Ok(());
        };
        if self.last_sent_link_room == Some(room) {
            return Ok(());
        }
        let frame = state.frame_counter.max(1);
        let choice = MenuChoiceFrame::new(
            self.player_id,
            Frame(frame),
            "cable_club_room",
            usize::from(room),
            0,
        )
        .context("build Cable Club room synchronization frame")?;
        self.session
            .as_mut()
            .context("hosted multiplayer session is not matched")?
            .send(LinkMessage::MenuChoice(choice))
            .context("send Cable Club room synchronization frame")?;
        self.last_sent_link_room = Some(room);
        Ok(())
    }

    fn send_local_party(&mut self, runtime_shell: &BevyRuntimeShell) -> Result<()> {
        if self.party_sent {
            return Ok(());
        }
        let state = runtime_shell.shell.session().state();
        let party = LinkPartyFrame::new(
            self.player_id,
            state.frame_counter.max(1),
            state.storage.party.clone(),
        )
        .context("build local multiplayer party snapshot")?;
        self.session
            .as_mut()
            .context("hosted multiplayer session is not matched")?
            .send(LinkMessage::Party(party))
            .context("send local multiplayer party snapshot")?;
        self.party_sent = true;
        Ok(())
    }

    fn receive_peer_party(&mut self, party: LinkPartyFrame) -> Result<()> {
        if party.player_id() == self.player_id {
            anyhow::bail!(
                "multiplayer peer party echoed local player {}",
                self.player_id
            );
        }
        match &self.remote_party {
            Some((revision, existing)) if *revision == party.revision() => {
                if existing != party.party() {
                    anyhow::bail!("multiplayer peer sent a conflicting party revision")
                }
            }
            Some((revision, _)) if *revision > party.revision() => {
                anyhow::bail!("multiplayer peer party revision regressed")
            }
            _ => self.remote_party = Some((party.revision(), party.into_party())),
        }
        Ok(())
    }

    fn receive_link_battle_random(&mut self, frame: LinkBattleRngFrame) -> Result<()> {
        if self.owns_internal_clock {
            anyhow::bail!("external-clock peer attempted to own Colosseum random seeds");
        }
        if Some(frame.player_id()) != self.peer_player_id {
            anyhow::bail!("Colosseum random seeds came from an unexpected player");
        }
        match &self.link_battle_random {
            Some(existing) if existing != frame.state() => {
                anyhow::bail!("peer sent conflicting Colosseum random seeds")
            }
            Some(_) => {}
            None => self.link_battle_random = Some(frame.into_state()),
        }
        Ok(())
    }

    fn synchronize_game_link_state(&mut self, runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
        let (Some(local_room), Some(remote_room)) =
            (self.last_sent_link_room, self.remote_link_room)
        else {
            return Ok(());
        };
        {
            let state = runtime_shell.shell.session_mut().state_mut();
            apply_connected_link_state(state, self.owns_internal_clock, remote_room);
            if self.direct_session && local_room == remote_room {
                activate_direct_link_room(state, local_room)?;
            }
        }
        if runtime_shell.pending_linked_friend_wait {
            complete_visible_linked_friend_wait(runtime_shell)?;
        }
        if !self.game_link_ready {
            runtime_shell.last_action_status = if local_room == remote_room {
                Some("Cable Club partner ready".to_string())
            } else {
                Some("Cable Club room selection differs from peer".to_string())
            };
            self.game_link_ready = true;
            mark_runtime_snapshot_dirty(runtime_shell);
        }
        Ok(())
    }

    fn current_trade_id(&self) -> String {
        hosted_trade_id(&self.trade_id_prefix, self.trade_sequence)
    }

    fn advance_trade_center(&mut self, runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
        if !self.game_link_ready
            || !matches!(
                runtime_shell.shell.session().state().link_session.link_mode,
                1 | 2
            )
        {
            return Ok(());
        }
        if runtime_shell.pending_link_trade_save {
            if runtime_shell
                .shell
                .session()
                .state()
                .pending_move_learn
                .is_some()
            {
                return Ok(());
            }
            anyhow::ensure!(
                quick_save_with_policy(runtime_shell, true, false, false)?,
                "SaveAfterLinkTrade did not persist the completed trade"
            );
            runtime_shell.pending_link_trade_save = false;
            mark_runtime_snapshot_dirty(runtime_shell);
        }
        let peer_player_id = self
            .peer_player_id
            .context("Trade Center has no connected peer identity")?;
        if self.remote_party.is_none() {
            return Ok(());
        }
        if self.active_trade.is_none() {
            let participants = crate::core::multiplayer::TradeParticipants::new(
                self.current_trade_id(),
                self.player_id,
                peer_player_id,
            )
            .context("create Trade Center participants")?;
            self.active_trade = Some(TradeSyncBuffer::new(participants));
        }

        let expected_trade_id = self.current_trade_id();
        while let Some(offer) = self.peer_trade_offers.pop_front() {
            if offer.trade_id() != expected_trade_id {
                anyhow::bail!(
                    "peer trade offer {} does not match active trade {}",
                    offer.trade_id(),
                    expected_trade_id
                );
            }
            let result = self
                .active_trade
                .as_mut()
                .expect("active trade was initialized")
                .insert_offer(offer)
                .context("insert peer Trade Center offer")?;
            if matches!(
                result,
                crate::core::multiplayer::InsertTradeFrameResult::Conflict
            ) {
                anyhow::bail!("peer sent a conflicting Trade Center offer");
            }
        }
        while let Some(confirmation) = self.peer_trade_confirmations.pop_front() {
            if confirmation.trade_id() != expected_trade_id {
                anyhow::bail!(
                    "peer trade confirmation {} does not match active trade {}",
                    confirmation.trade_id(),
                    expected_trade_id
                );
            }
            let result = self
                .active_trade
                .as_mut()
                .expect("active trade was initialized")
                .insert_confirmation(confirmation)
                .context("insert peer Trade Center confirmation")?;
            if matches!(
                result,
                crate::core::multiplayer::InsertTradeFrameResult::Conflict
            ) {
                anyhow::bail!("peer sent a conflicting Trade Center confirmation");
            }
        }

        if let Some(selection) = runtime_shell.pending_link_trade_party_slot.take() {
            match selection {
                Some(party_slot) => {
                    let offer = TradeOffer::from_party(
                        expected_trade_id.clone(),
                        self.player_id,
                        &runtime_shell.shell.session().state().storage.party,
                        party_slot,
                    )
                    .context("build local Trade Center offer")?;
                    let result = self
                        .active_trade
                        .as_mut()
                        .expect("active trade was initialized")
                        .insert_offer(offer.clone())
                        .context("insert local Trade Center offer")?;
                    if matches!(
                        result,
                        crate::core::multiplayer::InsertTradeFrameResult::Conflict
                    ) {
                        anyhow::bail!("local Trade Center offer conflicts with its retained offer");
                    }
                    self.session
                        .as_mut()
                        .context("hosted multiplayer session is not matched")?
                        .send(LinkMessage::TradeOffer(offer))
                        .context("send local Trade Center offer")?;
                    runtime_shell.last_action_status =
                        Some("Waiting for the other player's Pokemon".to_string());
                }
                None => {
                    self.submit_local_trade_confirmation(&expected_trade_id, false)?;
                    runtime_shell.last_action_status =
                        Some("Waiting for the other player to leave".to_string());
                }
            }
        }

        let peer_requested_exit = {
            let trade = self
                .active_trade
                .as_ref()
                .expect("active trade was initialized");
            trade.offer(peer_player_id).is_none()
                && trade.confirmation(peer_player_id) == Some(false)
                && trade.confirmation(self.player_id).is_none()
        };
        if peer_requested_exit {
            self.submit_local_trade_confirmation(&expected_trade_id, false)?;
        }

        if hosted_trade_state(
            self.active_trade
                .as_ref()
                .expect("active trade was initialized"),
        ) == HostedTradeState::ExitRoom
        {
            self.report_result(crystal_net::hosted::MatchOutcome::Cancelled)?;
            self.active_trade = None;
            self.trade_sequence = self.trade_sequence.saturating_add(1);
            runtime_shell.pending_link_trade_confirmation = None;
            complete_visible_link_room_session(runtime_shell)?;
            runtime_shell.last_action_status = Some("Left the Trade Center".to_string());
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(());
        }

        let trade = self
            .active_trade
            .as_ref()
            .expect("active trade was initialized");
        if trade.offer(self.player_id).is_none()
            && trade.confirmation(self.player_id).is_none()
            && runtime_shell
                .shell
                .session()
                .state()
                .pending_move_learn
                .is_none()
            && runtime_shell.pending_script_party_selection.is_none()
            && !runtime_shell.party_menu_open
            && runtime_shell.pc_confirmation.is_none()
        {
            runtime_shell.pending_script_party_selection =
                Some(PendingScriptPartySelection::LinkTrade);
            open_visible_party_menu(runtime_shell)?;
            set_shell_action_status(runtime_shell, "OFFER WHICH POKEMON?");
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(());
        }

        if let Some(confirm) = runtime_shell.pending_link_trade_confirmation.take() {
            self.submit_local_trade_confirmation(&expected_trade_id, confirm)?;
        }

        let trade = self
            .active_trade
            .as_ref()
            .expect("active trade was initialized");
        let local_cannot_battle_after_trade = if let (Some(local), Some(remote)) =
            (trade.offer(self.player_id), trade.offer(peer_player_id))
        {
            !link_trade_leaves_battle_ready(
                &runtime_shell.shell.session().state().storage.party,
                local.party_slot(),
                remote.pokemon(),
            )
        } else {
            false
        };
        if local_cannot_battle_after_trade && trade.confirmation(self.player_id).is_none() {
            self.submit_local_trade_confirmation(&expected_trade_id, false)?;
            runtime_shell.last_action_status =
                Some("That trade would leave no battle-ready Pokemon".to_string());
        }

        let trade = self
            .active_trade
            .as_ref()
            .expect("active trade was initialized");
        if trade.offer(self.player_id).is_some()
            && trade.offer(peer_player_id).is_some()
            && trade.confirmation(self.player_id).is_none()
            && runtime_shell.pc_confirmation.is_none()
        {
            let remote = trade
                .offer(peer_player_id)
                .expect("checked peer trade offer");
            runtime_shell.pc_notice = Some(format!("Trade for {}?", remote.pokemon().nickname));
            runtime_shell.pc_confirmation = Some(VisiblePcConfirmation::LinkTrade);
            runtime_shell.yes_no_cursor = Some(MenuCursor {
                surface_id: "pc:confirmation".to_string(),
                option_index: 0,
            });
            set_shell_action_status(runtime_shell, "TRADE?");
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(());
        }

        if !self
            .active_trade
            .as_ref()
            .expect("active trade was initialized")
            .is_ready()
        {
            return Ok(());
        }
        let outcome = self
            .active_trade
            .as_ref()
            .expect("active trade was initialized")
            .outcome()
            .context("resolve Trade Center outcome")?;
        if !outcome.cancelled() {
            let transfer_mode =
                if self.match_mode == Some(crystal_net::hosted::MatchMode::TimeCapsule) {
                    crate::core::multiplayer::LinkTradeTransferMode::TimeCapsule
                } else {
                    crate::core::multiplayer::LinkTradeTransferMode::TradeCenter
                };
            let applied = runtime_shell
                .shell
                .apply_link_trade_outcome(&outcome, self.player_id, transfer_mode)
                .context("apply and evolve Trade Center party replacement")?;
            runtime_shell
                .shell
                .snapshot()
                .context("validate runtime after Trade Center party replacement")?;
            self.party_sent = false;
            runtime_shell.pending_link_trade_save = true;
            runtime_shell.last_action_status =
                Some(applied.evolution.target_species.as_ref().map_or_else(
                    || "Trade completed".to_string(),
                    |species| format!("Trade completed; received Pokemon evolved into {species}"),
                ));
            runtime_shell.last_audio_events.push(format!(
                "link trade completed sent={} received_slot={} evolution={:?}",
                applied.sent.species.id,
                applied.received_party_index,
                applied.evolution.target_species
            ));
            trim_event_log(&mut runtime_shell.last_audio_events);
        } else {
            runtime_shell.last_action_status = Some("Trade cancelled".to_string());
        }
        self.active_trade = None;
        self.trade_sequence = self.trade_sequence.saturating_add(1);
        mark_runtime_snapshot_dirty(runtime_shell);
        Ok(())
    }

    fn submit_local_trade_confirmation(&mut self, trade_id: &str, confirm: bool) -> Result<()> {
        let confirmation = TradeConfirmation::new(trade_id, self.player_id, confirm)
            .context("build local Trade Center confirmation")?;
        let result = self
            .active_trade
            .as_mut()
            .context("active Trade Center exchange is missing")?
            .insert_confirmation(confirmation.clone())
            .context("insert local Trade Center confirmation")?;
        if matches!(
            result,
            crate::core::multiplayer::InsertTradeFrameResult::Conflict
        ) {
            anyhow::bail!("local Trade Center confirmation conflicts with retained choice");
        }
        self.session
            .as_mut()
            .context("hosted multiplayer session is not matched")?
            .send(LinkMessage::TradeConfirmation(confirmation))
            .context("send local Trade Center confirmation")
    }

    fn advance_colosseum(&mut self, runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
        if self.link_battle_started
            || runtime_shell.shell.session().state().link_session.link_mode != 3
        {
            return Ok(());
        }
        let Some((_, remote_party)) = &self.remote_party else {
            return Ok(());
        };
        if self.owns_internal_clock && self.link_battle_random.is_none() {
            let random = runtime_shell
                .shell
                .generate_link_battle_random_state()
                .context("generate internal-clock Colosseum random seeds")?;
            self.link_battle_random = Some(random);
        }
        if self.owns_internal_clock && !self.link_battle_random_sent {
            let frame = LinkBattleRngFrame::new(
                self.player_id,
                self.link_battle_random
                    .clone()
                    .context("internal-clock Colosseum random seeds are missing")?,
            )
            .context("build Colosseum random frame")?;
            self.session
                .as_mut()
                .context("Colosseum random seeds have no matched session")?
                .send(LinkMessage::LinkBattleRngInit(frame))
                .context("send Colosseum random seeds")?;
            self.link_battle_random_sent = true;
        }
        let Some(random_state) = self.link_battle_random.clone() else {
            runtime_shell.last_action_status =
                Some("Waiting for Colosseum random seeds".to_string());
            return Ok(());
        };
        let enemy_party = remote_party
            .pokemon
            .iter()
            .flatten()
            .cloned()
            .collect::<Vec<_>>();
        let start = LinkBattleStart {
            opponent_player_id: self
                .peer_player_id
                .context("Colosseum has no connected peer identity")?,
            opponent_name: self
                .peer_player_name
                .clone()
                .context("Colosseum has no connected peer name")?,
            enemy_party,
            random_state,
        };
        runtime_shell
            .shell
            .start_link_battle(&start)
            .context("start live Colosseum battle")?;
        prepare_visible_battle_entry(runtime_shell)?;
        self.link_battle_started = true;
        set_shell_action_status(runtime_shell, "COLOSSEUM BATTLE");
        mark_runtime_snapshot_dirty(runtime_shell);
        Ok(())
    }

    fn advance_colosseum_turn(&mut self, runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
        let Some(local_action) = runtime_shell.pending_link_battle_action.clone() else {
            return Ok(());
        };
        let combat = active_battle_combat_state(runtime_shell.shell.session().state())?;
        let expected_turn = u64::from(combat.turn).saturating_add(1);
        let peer_player_id = self
            .peer_player_id
            .context("Colosseum turn has no peer identity")?;
        let Some(position) = self
            .peer_battle_actions
            .iter()
            .position(|frame| frame.player_id() == peer_player_id && frame.turn() == expected_turn)
        else {
            return Ok(());
        };
        let peer_frame = self
            .peer_battle_actions
            .remove(position)
            .expect("located Colosseum peer action disappeared");
        if self
            .peer_battle_actions
            .iter()
            .any(|frame| frame.player_id() == peer_player_id && frame.turn() == expected_turn)
        {
            anyhow::bail!("peer sent duplicate Colosseum actions for turn {expected_turn}");
        }
        let snapshot = runtime_shell.shell.snapshot()?;
        let battle_before = snapshot
            .battle
            .clone()
            .context("Colosseum action has no active battle")?;
        anyhow::ensure!(
            battle_before.battle_type == "BATTLETYPE_LINK",
            "queued Colosseum action belongs to a non-link battle"
        );
        let peer_action = peer_frame.action().clone();
        record_visible_runtime_action(
            runtime_shell,
            format!("battle:link:turn:{expected_turn}:local:{local_action:?}:peer:{peer_action:?}"),
        )?;
        let turn = runtime_shell
            .shell
            .resolve_active_battle_turn(local_action, peer_action)
            .context("resolve synchronized Colosseum turn")?;
        runtime_shell.pending_link_battle_action = None;
        stage_visible_battle_messages(runtime_shell, &snapshot, &turn.outcome.events);
        runtime_shell.last_audio_events.push(format!(
            "link battle turn={} {} events={} checksum={:?}",
            expected_turn,
            format_battle_turn_summary(&turn.outcome),
            format_battle_turn_events(&turn.outcome.events),
            turn.state_checksum
        ));
        set_shell_action_status(
            runtime_shell,
            format!("LINK BATTLE {}", format_battle_turn_summary(&turn.outcome)),
        );
        trim_event_log(&mut runtime_shell.last_audio_events);
        settle_visible_resolved_battle_turn(runtime_shell, &battle_before)?;
        mark_runtime_snapshot_dirty(runtime_shell);
        Ok(())
    }

    fn finish_colosseum_if_terminal(&mut self, runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
        if self.match_mode != Some(crystal_net::hosted::MatchMode::Battle) || self.result_reported {
            return Ok(());
        }
        let snapshot = runtime_shell.shell.snapshot()?;
        let Some(battle) = snapshot
            .battle
            .as_ref()
            .filter(|battle| battle.battle_type == "BATTLETYPE_LINK")
        else {
            return Ok(());
        };
        let local_usable = snapshot
            .party
            .slots
            .iter()
            .any(|slot| !slot.pokemon.is_egg && slot.pokemon.hp > 0);
        let remote_usable = battle
            .enemy_party
            .iter()
            .any(|pokemon| !pokemon.is_egg && pokemon.hp > 0);
        if local_usable && remote_usable {
            return Ok(());
        }
        let Some((game_result, hosted_result)) = terminal_link_results(local_usable, remote_usable)
        else {
            return Ok(());
        };
        runtime_shell
            .shell
            .record_link_battle_result(game_result)
            .context("record completed Colosseum result")?;
        runtime_shell
            .shell
            .finish_link_battle(game_result)
            .context("finish completed Colosseum battle")?;
        reset_visible_battle_exit_state(runtime_shell);
        self.report_result(hosted_result)?;
        complete_visible_link_room_session(runtime_shell)?;
        runtime_shell.last_action_status = Some(
            match game_result {
                RuntimeLinkBattleResult::Win => "LINK BATTLE WON",
                RuntimeLinkBattleResult::Loss => "LINK BATTLE LOST",
                RuntimeLinkBattleResult::Draw => "LINK BATTLE DRAW",
            }
            .into(),
        );
        mark_runtime_snapshot_dirty(runtime_shell);
        Ok(())
    }

    fn advance_colosseum_replacement(
        &mut self,
        runtime_shell: &mut BevyRuntimeShell,
    ) -> Result<()> {
        let snapshot = runtime_shell.shell.snapshot()?;
        let Some(battle) = snapshot.battle.as_ref() else {
            runtime_shell.pending_link_battle_replacement = None;
            return Ok(());
        };
        if battle.battle_type != "BATTLETYPE_LINK" {
            runtime_shell.pending_link_battle_replacement = None;
            return Ok(());
        }
        // The local replacement frame is already retained in the deterministic
        // outbound journal. Clearing this latch lets a Spikes KO select and
        // queue another replacement for the same completed battle turn.
        runtime_shell.pending_link_battle_replacement = None;
        if battle.enemy_pokemon.hp != 0 || battle.enemy_spikes_zero_hp_unchecked {
            return Ok(());
        }
        let turn =
            u64::from(active_battle_combat_state(runtime_shell.shell.session().state())?.turn);
        let peer_player_id = self
            .peer_player_id
            .context("Colosseum replacement has no peer identity")?;
        let Some(position) = self.peer_battle_actions.iter().position(|frame| {
            frame.player_id() == peer_player_id
                && frame.turn() == turn
                && matches!(frame.action(), BattleAction::Switch { .. })
        }) else {
            set_shell_action_status(runtime_shell, "WAITING FOR LINK REPLACEMENT");
            return Ok(());
        };
        let frame = self
            .peer_battle_actions
            .remove(position)
            .expect("located Colosseum replacement disappeared");
        let BattleAction::Switch { party_index } = frame.action() else {
            unreachable!("Colosseum replacement predicate accepted a non-switch action");
        };
        let switched = runtime_shell
            .shell
            .switch_link_battle_enemy_party(*party_index)
            .context("apply synchronized peer Colosseum replacement")?;
        let replacement = runtime_shell.shell.snapshot()?;
        let replacement_battle = replacement
            .battle
            .as_ref()
            .context("peer Colosseum replacement removed the active battle")?;
        let message = format!(
            "{}\nsent out\n{}!",
            self.peer_player_name.as_deref().unwrap_or("LINK"),
            replacement_battle.enemy_pokemon.nickname
        );
        runtime_shell.battle_messages.push_back(message.clone());
        if let Some(spikes_message) = visible_direct_spikes_message(
            &replacement,
            crate::core::battle::turn::BattleSide::Enemy,
            &switched.spikes,
        ) {
            runtime_shell.battle_messages.push_back(spikes_message);
        }
        runtime_shell.battle_enemy_send_out_pending = true;
        defer_visible_battle_cry_after_message(
            runtime_shell,
            replacement_battle.enemy_pokemon.species.id.clone(),
            "link_replacement",
            message,
        );
        runtime_shell.battle_message_scene = Some(Box::new(replacement));
        set_shell_action_status(runtime_shell, "LINK OPPONENT SENT A POKEMON");
        mark_runtime_snapshot_dirty(runtime_shell);
        Ok(())
    }

    fn send_pending_inputs(&mut self, runtime_shell: &BevyRuntimeShell) -> Result<()> {
        if self.sent_input_count > runtime_shell.deterministic_input_frames.len() {
            self.sent_input_count = 0;
        }
        for input in runtime_shell
            .deterministic_input_frames
            .iter()
            .skip(self.sent_input_count)
        {
            let input =
                PlayerInputFrame::new(self.player_id, Frame(input.frame()), input.joypad_mask())
                    .context("remap local Bevy input to multiplayer player")?;
            self.session
                .as_mut()
                .context("hosted multiplayer session is not matched")?
                .send(LinkMessage::Input(input))
                .context("send Bevy multiplayer input")?;
            self.sent_input_count += 1;
        }
        Ok(())
    }

    fn send_pending_battle_actions(&mut self, runtime_shell: &BevyRuntimeShell) -> Result<()> {
        if self.sent_battle_action_count > runtime_shell.deterministic_battle_actions.len() {
            self.sent_battle_action_count = 0;
        }
        for action in runtime_shell
            .deterministic_battle_actions
            .iter()
            .skip(self.sent_battle_action_count)
        {
            let action = BattleActionFrame::new(
                self.player_id,
                action.turn(),
                action.action().clone(),
                action.state_hash(),
            )
            .context("remap local Bevy battle action to multiplayer player")?;
            self.session
                .as_mut()
                .context("hosted multiplayer session is not matched")?
                .send(LinkMessage::BattleAction(action))
                .context("send Bevy multiplayer battle action")?;
            self.sent_battle_action_count += 1;
        }
        Ok(())
    }

    fn send_pending_menu_results(&mut self, runtime_shell: &BevyRuntimeShell) -> Result<()> {
        if self.sent_menu_result_count > runtime_shell.deterministic_menu_results.len() {
            self.sent_menu_result_count = 0;
        }
        for result in runtime_shell
            .deterministic_menu_results
            .iter()
            .skip(self.sent_menu_result_count)
        {
            let choice = result.choice();
            let choice = MenuChoiceFrame::new(
                self.player_id,
                Frame(choice.frame()),
                choice.menu_id(),
                choice.option_index(),
                choice.verticalmenu_command_index(),
            )
            .context("remap local Bevy menu choice to multiplayer player")?;
            let checksum = StateChecksumFrame::new(
                self.player_id,
                Frame(result.checksum().frame()),
                result.checksum().hash(),
            );
            let result = MenuChoiceResultFrame::new(choice, checksum, result.script_value())
                .context("remap local Bevy menu result to multiplayer player")?;
            self.session
                .as_mut()
                .context("hosted multiplayer session is not matched")?
                .send(LinkMessage::MenuChoiceResult(result))
                .context("send Bevy multiplayer menu result")?;
            self.sent_menu_result_count += 1;
        }
        Ok(())
    }
}

fn selected_cable_club_room(
    state: &crate::core::state::GameState,
    pending_room: Option<u8>,
) -> Option<u8> {
    pending_room.filter(|room| {
        *room <= 2
            && *room == state.link_session.chosen_cable_club_room
            && *room == state.link_session.player_link_action
    })
}

fn requested_match_mode(
    state: &crate::core::state::GameState,
    pending_room: Option<u8>,
) -> Option<crystal_net::hosted::MatchMode> {
    match selected_cable_club_room(state, pending_room)? {
        0 => Some(crystal_net::hosted::MatchMode::TimeCapsule),
        1 => Some(crystal_net::hosted::MatchMode::Trade),
        2 => Some(crystal_net::hosted::MatchMode::Battle),
        _ => None,
    }
}

fn match_mode_room(mode: crystal_net::hosted::MatchMode) -> u8 {
    match mode {
        crystal_net::hosted::MatchMode::TimeCapsule => 0,
        crystal_net::hosted::MatchMode::Trade => 1,
        crystal_net::hosted::MatchMode::Battle => 2,
    }
}

fn hosted_trade_id(session_id: &str, sequence: u64) -> String {
    format!("{session_id}-trade-{sequence}")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HostedTradeState {
    Pending,
    ExchangeReady,
    ExitRoom,
}

fn hosted_trade_state(trade: &TradeSyncBuffer) -> HostedTradeState {
    let players = trade.participants().players();
    let both_cancelled = players
        .iter()
        .all(|player_id| trade.confirmation(*player_id) == Some(false));
    let selection_cancelled = players
        .iter()
        .any(|player_id| trade.offer(*player_id).is_none());
    if both_cancelled && selection_cancelled {
        HostedTradeState::ExitRoom
    } else if trade.is_ready() {
        HostedTradeState::ExchangeReady
    } else {
        HostedTradeState::Pending
    }
}

fn link_trade_leaves_battle_ready(
    party: &crate::core::models::Party,
    offered_party_slot: usize,
    received: &crate::core::models::Pokemon,
) -> bool {
    received.hp > 0
        || party.pokemon.iter().enumerate().any(|(index, pokemon)| {
            index != offered_party_slot && pokemon.as_ref().is_some_and(|pokemon| pokemon.hp > 0)
        })
}

fn activate_direct_link_room(state: &mut crate::core::state::GameState, room: u8) -> Result<()> {
    state.link_session.link_mode = room.saturating_add(1);
    anyhow::ensure!(room <= 2, "direct multiplayer room is invalid");
    if room == 2 {
        // A directly activated Colosseum is saveable before peer seed
        // negotiation. The negotiated stream replaces this untouched
        // cartridge-shaped stream before the first battle turn.
        state.link_session.battle_random.get_or_insert_with(|| {
            crate::core::random::LinkBattleRandomState {
                seeds: [0; crate::core::random::LINK_BATTLE_RANDOM_SEED_COUNT],
                count: 0,
            }
        });
    }
    Ok(())
}

fn terminal_link_results(
    local_usable: bool,
    remote_usable: bool,
) -> Option<(RuntimeLinkBattleResult, crystal_net::hosted::MatchOutcome)> {
    match (local_usable, remote_usable) {
        (true, true) => None,
        (true, false) => Some((
            RuntimeLinkBattleResult::Win,
            crystal_net::hosted::MatchOutcome::Local,
        )),
        (false, true) => Some((
            RuntimeLinkBattleResult::Loss,
            crystal_net::hosted::MatchOutcome::Remote,
        )),
        (false, false) => Some((
            RuntimeLinkBattleResult::Draw,
            crystal_net::hosted::MatchOutcome::Draw,
        )),
    }
}

fn apply_connected_link_state(
    state: &mut crate::core::state::GameState,
    owns_internal_clock: bool,
    remote_room: u8,
) {
    state.link_session.serial_connection_status = if owns_internal_clock {
        crate::core::state::LinkSerialConnectionStatus::UsingInternalClock
    } else {
        crate::core::state::LinkSerialConnectionStatus::UsingExternalClock
    };
    state
        .script_runtime
        .variables
        .insert("_other_player_room".to_string(), remote_room.to_string());
    state.script_runtime.variables.insert(
        "_other_player_link_mode".to_string(),
        remote_room.saturating_add(1).to_string(),
    );
}

fn mark_link_disconnected(state: &mut crate::core::state::GameState) {
    state.link_session.serial_connection_status =
        crate::core::state::LinkSerialConnectionStatus::NotEstablished;
}

fn sync_multiplayer_ghosts(
    mut commands: Commands,
    multiplayer: Option<NonSend<MultiplayerRuntime>>,
    runtime_shell: Res<BevyRuntimeShell>,
    rendered: Res<RenderedViewport>,
    time: Res<Time>,
    mut tileset_art: ResMut<RenderedTilesetArt>,
    mut images: ResMut<Assets<Image>>,
    player: Query<&Sprite, (With<PlayerMarker>, Without<MultiplayerGhost>)>,
    mut ghosts: Query<
        (
            Entity,
            &mut MultiplayerGhost,
            &mut Handle<Image>,
            &mut Sprite,
            &mut Transform,
        ),
        Without<PlayerMarker>,
    >,
) {
    let Some(multiplayer) = multiplayer else {
        for (entity, ..) in &mut ghosts {
            commands.entity(entity).despawn_recursive();
        }
        return;
    };
    let Ok(player_sprite) = player.get_single() else {
        return;
    };
    let Some((start_x, start_y)) = rendered.viewport_origin else {
        return;
    };
    let Ok(snapshot) = runtime_shell.shell.snapshot() else {
        return;
    };
    let camera_offset = visible_overworld_camera_offset(&rendered, &runtime_shell, 1.0);
    let mut retained = HashSet::new();

    for (entity, mut ghost, mut texture, mut sprite, mut transform) in &mut ghosts {
        let Some(presence) = multiplayer.remote_presences.get(&ghost.user_id) else {
            commands.entity(entity).despawn_recursive();
            continue;
        };
        let target_tile = TilePosition::new(presence.tile_x, presence.tile_y);
        let Some(_) = (presence.map == snapshot.overworld.map_name)
            .then(|| runtime_tile_playfield_position(target_tile, start_x, start_y))
            .flatten()
        else {
            commands.entity(entity).despawn_recursive();
            continue;
        };
        let target = Vec2::new(f32::from(presence.tile_x), f32::from(presence.tile_y));
        let distance = ghost.display_tile.distance(target);
        if distance > 2.0 {
            ghost.display_tile = target;
        } else {
            ghost.display_tile = move_toward(
                ghost.display_tile,
                target,
                3.75 * time.delta_seconds().min(0.1),
            );
        }
        let walking = ghost.display_tile.distance_squared(target) > 0.0001
            && (time.elapsed_seconds() * 8.0) as u64 & 1 == 1;
        let Some(frame) = multiplayer_ghost_frame(
            &snapshot,
            &runtime_shell,
            presence,
            walking,
            &mut tileset_art,
            &mut images,
        ) else {
            commands.entity(entity).despawn_recursive();
            continue;
        };
        let size = frame.size;
        let (base_x, base_y) = remote_tile_playfield_position(ghost.display_tile, start_x, start_y);
        let Some(_) = player_sprite.custom_size else {
            commands.entity(entity).despawn_recursive();
            continue;
        };
        let (x, y) = overworld_sprite_position_from_base(base_x, base_y, size);
        *texture = frame.handle;
        sprite.custom_size = Some(size);
        sprite.color = multiplayer_ghost_color();
        sprite.flip_x = false;
        transform.translation = Vec3::new(
            x + camera_offset.x,
            y + camera_offset.y,
            overworld_entity_depth(target_tile, None, (start_x, start_y)) + 0.000_000_1,
        );
        retained.insert(ghost.user_id.clone());
    }

    let Some(_) = player_sprite.custom_size else {
        return;
    };
    for (user_id, presence) in &multiplayer.remote_presences {
        if retained.contains(user_id) || presence.map != snapshot.overworld.map_name {
            continue;
        }
        let tile = TilePosition::new(presence.tile_x, presence.tile_y);
        let Some((base_x, base_y)) = runtime_tile_playfield_position(tile, start_x, start_y) else {
            continue;
        };
        let Some(frame) = multiplayer_ghost_frame(
            &snapshot,
            &runtime_shell,
            presence,
            false,
            &mut tileset_art,
            &mut images,
        ) else {
            continue;
        };
        let (x, y) = overworld_sprite_position_from_base(base_x, base_y, frame.size);
        commands.spawn((
            SpriteBundle {
                texture: frame.handle,
                sprite: Sprite {
                    custom_size: Some(frame.size),
                    color: multiplayer_ghost_color(),
                    flip_x: false,
                    ..default()
                },
                transform: Transform::from_xyz(
                    x + camera_offset.x,
                    y + camera_offset.y,
                    overworld_entity_depth(tile, None, (start_x, start_y)) + 0.000_000_1,
                ),
                ..default()
            },
            MultiplayerGhost {
                user_id: user_id.clone(),
                display_tile: Vec2::new(f32::from(presence.tile_x), f32::from(presence.tile_y)),
            },
            Name::new(format!("Multiplayer ghost: {}", presence.display_name)),
        ));
    }
}

fn move_toward(current: Vec2, target: Vec2, max_distance: f32) -> Vec2 {
    let delta = target - current;
    let distance = delta.length();
    if distance <= max_distance || distance == 0.0 {
        target
    } else {
        current + delta * (max_distance / distance)
    }
}

fn remote_tile_playfield_position(tile: Vec2, start_x: i16, start_y: i16) -> (f32, f32) {
    (
        PLAYFIELD_LEFT + (tile.x * 2.0 - f32::from(start_x) + 0.5) * TILE_SIZE,
        PLAYFIELD_TOP - (tile.y * 2.0 - f32::from(start_y) + 0.5) * TILE_SIZE,
    )
}

fn multiplayer_ghost_frame(
    snapshot: &crate::RuntimeShellSnapshot,
    runtime_shell: &BevyRuntimeShell,
    presence: &RemotePresence,
    walking: bool,
    tileset_art: &mut RenderedTilesetArt,
    images: &mut Assets<Image>,
) -> Option<SpriteFrame> {
    let direction = match presence.direction.as_str() {
        "up" => Direction::Up,
        "down" => Direction::Down,
        "left" => Direction::Left,
        "right" => Direction::Right,
        _ => return None,
    };
    let female = snapshot.trainer.player_gender == PLAYER_GENDER_FEMALE;
    let (sprite_id, sprite_token, palette_override) = match snapshot.overworld.mode {
        MovementMode::Normal | MovementMode::Skate if female => {
            ("kris", "SPRITE_KRIS", snapshot.trainer.player_palette_id)
        }
        MovementMode::Normal | MovementMode::Skate => {
            ("chris", "SPRITE_CHRIS", snapshot.trainer.player_palette_id)
        }
        MovementMode::Bike if female => (
            "kris_bike",
            "SPRITE_KRIS_BIKE",
            snapshot.trainer.player_palette_id,
        ),
        MovementMode::Bike => (
            "chris_bike",
            "SPRITE_CHRIS_BIKE",
            snapshot.trainer.player_palette_id,
        ),
        MovementMode::Surf => ("surf", "SPRITE_SURF", 1),
        MovementMode::SurfPika => ("surfing_pikachu", "SPRITE_SURFING_PIKACHU", 0),
    };
    let palette_id = resolve_visible_object_palette(
        sprite_token,
        palette_override,
        &snapshot.presentation.sprite_palette_defaults,
    );
    let live_time_of_day = match snapshot.progression.time.time_of_day {
        crystal_core::world::encounters::TimeOfDay::Morning => "morn",
        crystal_core::world::encounters::TimeOfDay::Day => "day",
        crystal_core::world::encounters::TimeOfDay::Night => "nite",
    };
    let map = snapshot
        .maps
        .iter()
        .find(|map| map.map_name == snapshot.overworld.map_name)?;
    let flash_active = snapshot
        .progression
        .active_engine_flags
        .contains("STATUSFLAGS_FLASH");
    let effective_time_of_day =
        visible_effective_map_time_of_day(map, live_time_of_day, flash_active);
    sprite_frame_for_art(
        tileset_art,
        &runtime_shell.asset_root,
        sprite_id,
        palette_id,
        effective_time_of_day,
        direction,
        walking,
        images,
    )
}

fn multiplayer_ghost_color() -> Color {
    Color::srgba(0.48, 0.88, 1.0, 0.62)
}

fn poll_multiplayer(
    multiplayer: Option<NonSendMut<MultiplayerRuntime>>,
    mut runtime_shell: ResMut<BevyRuntimeShell>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
) {
    let Some(mut multiplayer) = multiplayer else {
        return;
    };
    if let Err(error) = multiplayer.poll(&mut runtime_shell, &mut keys) {
        let cleanup_error = multiplayer.mark_disconnected(&mut runtime_shell).err();
        record_visible_runtime_system_error(
            &mut runtime_shell,
            error.context("advance hosted multiplayer"),
        );
        if let Some(cleanup_error) = cleanup_error {
            record_visible_runtime_system_error(
                &mut runtime_shell,
                cleanup_error.context("clean up disconnected multiplayer session"),
            );
        }
    }
}

fn hosted_direction(direction: crate::core::world::map::Direction) -> &'static str {
    match direction {
        crate::core::world::map::Direction::Up => "up",
        crate::core::world::map::Direction::Down => "down",
        crate::core::world::map::Direction::Left => "left",
        crate::core::world::map::Direction::Right => "right",
    }
}

fn trim_multiplayer_queue<T>(queue: &mut VecDeque<T>) {
    while queue.len() > RECENT_OVERWORLD_INPUT_LIMIT {
        queue.pop_front();
    }
}

#[cfg(test)]
mod multiplayer_tests {
    use super::*;

    #[test]
    fn remote_ghost_motion_advances_smoothly_and_lands_exactly() {
        let start = Vec2::new(4.0, 7.0);
        let target = Vec2::new(5.0, 7.0);
        let first = move_toward(start, target, 0.25);
        assert_eq!(first, Vec2::new(4.25, 7.0));
        assert_eq!(move_toward(first, target, 2.0), target);
    }

    #[test]
    fn interpolated_remote_tile_matches_exact_runtime_tile_coordinates() {
        let tile = TilePosition::new(8, 12);
        let viewport_origin = (6, 14);
        assert_eq!(
            remote_tile_playfield_position(Vec2::new(8.0, 12.0), 6, 14),
            runtime_tile_playfield_position(tile, viewport_origin.0, viewport_origin.1)
                .expect("tile is visible")
        );
    }

    #[test]
    fn connected_peer_state_supplies_exact_cable_club_special_inputs() {
        let mut state = crate::core::state::GameState::default();
        state.link_session.chosen_cable_club_room = 1;
        state.link_session.player_link_action = 1;
        state.script_runtime.last_special_routine = Some("UnrelatedRoutine".to_string());
        assert_eq!(selected_cable_club_room(&state, Some(1)), Some(1));
        assert_eq!(selected_cable_club_room(&state, None), None);

        apply_connected_link_state(&mut state, true, 1);
        assert_eq!(
            state.link_session.serial_connection_status,
            crate::core::state::LinkSerialConnectionStatus::UsingInternalClock
        );
        assert!(
            !state
                .script_runtime
                .variables
                .contains_key("_link_friend_ready")
        );
        assert!(!state.script_runtime.variables.contains_key("_link_timeout"));
        assert_eq!(
            state
                .script_runtime
                .variables
                .get("_other_player_room")
                .map(String::as_str),
            Some("1")
        );
        assert_eq!(
            state
                .script_runtime
                .variables
                .get("_other_player_link_mode")
                .map(String::as_str),
            Some("2")
        );
        state
            .validate_saved_state()
            .expect("live pre-room link handshake is saveable");

        mark_link_disconnected(&mut state);
        assert_eq!(
            state.link_session.serial_connection_status,
            crate::core::state::LinkSerialConnectionStatus::NotEstablished
        );
        assert_eq!(state.link_session.link_mode, 0);
        assert_eq!(state.link_session.player_link_action, 1);
        assert_eq!(state.link_session.chosen_cable_club_room, 1);
        assert_eq!(state.link_session.other_player_link_mode, 0);
        assert!(!state.script_runtime.variables.contains_key("_link_timeout"));
    }

    #[test]
    fn direct_ghost_sessions_activate_every_supported_link_room() {
        for (mode, room) in [
            (crystal_net::hosted::MatchMode::TimeCapsule, 0),
            (crystal_net::hosted::MatchMode::Trade, 1),
            (crystal_net::hosted::MatchMode::Battle, 2),
        ] {
            let mut state = crate::core::state::GameState::default();
            apply_connected_link_state(&mut state, true, room);
            activate_direct_link_room(&mut state, match_mode_room(mode)).unwrap();
            assert_eq!(state.link_session.link_mode, room + 1);
            assert_eq!(state.link_session.battle_random.is_some(), room == 2);
            state.validate_saved_state().unwrap();
        }
    }

    #[test]
    fn terminal_link_results_cover_win_loss_draw_and_ongoing() {
        assert!(terminal_link_results(true, true).is_none());
        assert_eq!(
            terminal_link_results(true, false),
            Some((
                RuntimeLinkBattleResult::Win,
                crystal_net::hosted::MatchOutcome::Local
            ))
        );
        assert_eq!(
            terminal_link_results(false, true),
            Some((
                RuntimeLinkBattleResult::Loss,
                crystal_net::hosted::MatchOutcome::Remote
            ))
        );
        assert_eq!(
            terminal_link_results(false, false),
            Some((
                RuntimeLinkBattleResult::Draw,
                crystal_net::hosted::MatchOutcome::Draw
            ))
        );
    }

    #[test]
    fn hosted_trade_ids_are_accepted_by_the_exact_trade_protocol() {
        let trade_id = hosted_trade_id("56f88ba3-62f5-4f93-aed4-54fd031b996a", 7);
        crate::core::multiplayer::TradeParticipants::new(trade_id, 1, 2)
            .expect("hosted trade id is wire-safe");
    }

    #[test]
    fn both_cancel_selections_exit_but_a_declined_offer_only_restarts_trade_menu() {
        let mut exit = TradeSyncBuffer::new(
            crate::core::multiplayer::TradeParticipants::new("trade-exit", 1, 2)
                .expect("participants"),
        );
        exit.insert_confirmation(
            TradeConfirmation::new("trade-exit", 1, false).expect("first exit"),
        )
        .expect("insert first exit");
        exit.insert_confirmation(
            TradeConfirmation::new("trade-exit", 2, false).expect("second exit"),
        )
        .expect("insert second exit");
        assert_eq!(hosted_trade_state(&exit), HostedTradeState::ExitRoom);

        let mut pending = TradeSyncBuffer::new(
            crate::core::multiplayer::TradeParticipants::new("trade-pending", 1, 2)
                .expect("participants"),
        );
        pending
            .insert_confirmation(
                TradeConfirmation::new("trade-pending", 1, false).expect("first response"),
            )
            .expect("insert first response");
        assert_eq!(hosted_trade_state(&pending), HostedTradeState::Pending);

        let species = crate::core::models::PokemonSpecies::new_for_tests(
            "PIKACHU",
            crate::core::models::BaseStats::new(35, 55, 40, 90, 50, 50),
        );
        let pokemon = crate::core::models::Pokemon::new_for_tests(
            species,
            10,
            crate::core::models::Dv::default(),
        );
        let mut declined = TradeSyncBuffer::new(
            crate::core::multiplayer::TradeParticipants::new("trade-declined", 1, 2)
                .expect("participants"),
        );
        declined
            .insert_offer(
                TradeOffer::new("trade-declined", 1, 0, pokemon.clone()).expect("first offer"),
            )
            .expect("insert first offer");
        declined
            .insert_offer(TradeOffer::new("trade-declined", 2, 0, pokemon).expect("second offer"))
            .expect("insert second offer");
        declined
            .insert_confirmation(
                TradeConfirmation::new("trade-declined", 1, false).expect("decline"),
            )
            .expect("insert decline");
        declined
            .insert_confirmation(
                TradeConfirmation::new("trade-declined", 2, true).expect("confirm"),
            )
            .expect("insert confirmation");
        assert_eq!(
            hosted_trade_state(&declined),
            HostedTradeState::ExchangeReady
        );
        assert!(declined.outcome().expect("declined outcome").cancelled());
    }

    #[test]
    fn link_trade_requires_an_alive_local_or_incoming_pokemon_after_exchange() {
        let species = crate::core::models::PokemonSpecies::new_for_tests(
            "PIKACHU",
            crate::core::models::BaseStats::new(35, 55, 40, 90, 50, 50),
        );
        let alive = crate::core::models::Pokemon::new_for_tests(
            species.clone(),
            10,
            crate::core::models::Dv::default(),
        );
        let mut fainted = alive.clone();
        fainted.hp = 0;
        let mut party = crate::core::models::Party::default();
        party.pokemon[0] = Some(alive.clone());

        assert!(!link_trade_leaves_battle_ready(&party, 0, &fainted));
        assert!(link_trade_leaves_battle_ready(&party, 0, &alive));
        party.pokemon[1] = Some(alive);
        assert!(link_trade_leaves_battle_ready(&party, 0, &fainted));
    }
}
