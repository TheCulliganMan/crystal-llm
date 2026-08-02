fn visible_battle_player_name(snapshot: &RuntimeShellSnapshot) -> &str {
    if snapshot
        .battle
        .as_ref()
        .is_some_and(|battle| battle.battle_type == "BATTLETYPE_TUTORIAL")
    {
        "DUDE"
    } else {
        &snapshot.trainer.player_name
    }
}

fn stage_visible_battle_messages(
    runtime_shell: &mut BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
    events: &[crate::core::battle::turn::BattleEvent],
) {
    use crate::core::battle::damage::Weather;
    use crate::core::battle::turn::{BattleEvent, BattleSide};
    use crate::core::models::Stat;

    let player_name = std::cell::RefCell::new(snapshot
        .battle
        .as_ref()
        .and_then(|battle| battle.active_player_party_index)
        .and_then(|index| snapshot.party.slots.iter().find(|slot| slot.index == index))
        .map(|slot| slot.pokemon.nickname.as_str())
        .unwrap_or("POKEMON")
        .to_string());
    let enemy_name = std::cell::RefCell::new(snapshot
        .battle
        .as_ref()
        .map(|battle| battle.enemy_pokemon.nickname.as_str())
        .unwrap_or("ENEMY")
        .to_string());
    let name = |side: BattleSide| match side {
        BattleSide::Player => player_name.borrow().clone(),
        BattleSide::Enemy => enemy_name.borrow().clone(),
    };
    let stat_name = |stat: &Stat| match stat {
        Stat::Hp => "HP",
        Stat::Attack => "ATTACK",
        Stat::Defense => "DEFENSE",
        Stat::Speed => "SPEED",
        Stat::SpecialAttack => "SPCL.ATK",
        Stat::SpecialDefense => "SPCL.DEF",
        Stat::Accuracy => "ACCURACY",
        Stat::Evasion => "EVASION",
    };
    let message_count_before = runtime_shell.battle_messages.len();
    let stage_message_scenes = message_count_before == 0;
    if stage_message_scenes {
        runtime_shell.battle_message_scenes.clear();
        runtime_shell.pending_battle_scenes_after_message.clear();
    }
    let mut event_scene = snapshot.clone();
    let mut event_scene_baton_pass_sides = BTreeSet::new();
    let mut perish_song_result_staged = false;
    let mut held_escape_item = None;
    let mut effectiveness_text_shown = BTreeSet::new();
    let disobedience_nap = events.iter().any(|event| {
        matches!(
            event,
            BattleEvent::StatusApplied { move_name, .. } if move_name == "DISOBEDIENCE_NAP"
        )
    });
    let disobedience_self_hit = events.iter().any(|event| {
        matches!(
            event,
            BattleEvent::ConfusionSelfDamage { move_name, .. } if move_name == "DISOBEDIENCE"
        )
    });
    let forced_switches = events
        .iter()
        .filter_map(|event| match event {
            BattleEvent::ForceSwitchApplied {
                target, move_name, ..
            } => Some((*target, move_name.as_str())),
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    let teleported_sides = events
        .iter()
        .filter_map(|event| match event {
            BattleEvent::MoveUsed { side, move_name } if move_name == "TELEPORT" => Some(*side),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let missed_moves = events
        .iter()
        .filter_map(|event| match event {
            BattleEvent::Missed { side, move_name, .. } => Some((*side, move_name.as_str())),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let mut baton_pass_sides = BTreeSet::new();
    for event in events {
        let event_scene_before = event_scene.clone();
        apply_visible_battle_event_to_scene(
            &mut event_scene,
            event,
            &mut event_scene_baton_pass_sides,
        );
        let event_message_count_before = runtime_shell.battle_messages.len();
        if let BattleEvent::HeldItemEscape { side, item_id, .. } = event {
            held_escape_item = Some((*side, item_id.clone()));
            continue;
        }
        if matches!(
            event,
            BattleEvent::PerishSongApplied { .. } | BattleEvent::PerishSongFailed { .. }
        ) {
            if perish_song_result_staged {
                continue;
            }
            perish_song_result_staged = true;
        }
        if let BattleEvent::RapidSpinCleared {
            side,
            trap_move,
            cleared_leech_seed,
            cleared_spikes,
            ..
        } = event
        {
            if *cleared_leech_seed {
                runtime_shell
                    .battle_messages
                    .push_back(format!("{} shed LEECH SEED!", name(*side)));
            }
            if *cleared_spikes {
                runtime_shell
                    .battle_messages
                    .push_back(format!("{} blew away SPIKES!", name(*side)));
            }
            if let Some(trap_move) = trap_move {
                runtime_shell.battle_messages.push_back(format!(
                    "{} was released by {}!",
                    name(*side),
                    battle_move_display_name(snapshot, trap_move)
                ));
            }
            if stage_message_scenes {
                for _ in event_message_count_before..runtime_shell.battle_messages.len() {
                    runtime_shell
                        .battle_message_scenes
                        .push_back(Box::new(event_scene.clone()));
                }
            }
            continue;
        }
        if let BattleEvent::Damage {
            side,
            move_name,
            critical,
            result,
            ..
        } = event
        {
            if snapshot
                .moves
                .iter()
                .any(|move_data| {
                    (move_data.move_id == *move_name || move_data.name == *move_name)
                        && move_data.effect == "OHKO"
                })
            {
                runtime_shell
                    .battle_messages
                    .push_back("It's a one-hit KO!".to_string());
            }
            if *critical {
                runtime_shell
                    .battle_messages
                    .push_back("A critical hit!".to_string());
            }
            let side_key = match side {
                BattleSide::Player => 0u8,
                BattleSide::Enemy => 1u8,
            };
            if effectiveness_text_shown.insert((side_key, move_name.as_str())) {
                if result.type_multiplier.numerator > result.type_multiplier.denominator {
                    runtime_shell
                        .battle_messages
                        .push_back("It's super effective!".to_string());
                } else if result.type_multiplier.numerator < result.type_multiplier.denominator {
                    runtime_shell
                        .battle_messages
                        .push_back("It's not very effective...".to_string());
                }
            }
            if stage_message_scenes {
                for _ in event_message_count_before..runtime_shell.battle_messages.len() {
                    runtime_shell
                        .battle_message_scenes
                        .push_back(Box::new(event_scene.clone()));
                }
                if event_message_count_before == runtime_shell.battle_messages.len() {
                    // Neutral, non-critical damage has no follow-up textbox in
                    // Crystal. Retain the pre-hit frame under the "used"
                    // line and apply damage only when that exact page closes.
                    if let Some(trigger_message) = runtime_shell.battle_messages.back().cloned() {
                        runtime_shell
                            .pending_battle_scenes_after_message
                            .push_back((trigger_message, Box::new(event_scene.clone())));
                    }
                }
            }
            continue;
        }
        let message = match event {
            BattleEvent::MoveUsed { side, move_name } => {
                let message = format!(
                    "{} used {}!",
                    name(*side),
                    battle_move_display_name(snapshot, move_name)
                );
                if !missed_moves.contains(&(*side, move_name.as_str())) {
                    if let Some((animation_label, total_frames, sound_events, cry_events, object_events, bg_events)) =
                        visible_move_animation_definition(snapshot, move_name)
                    {
                        runtime_shell.visible_move_animations.push_back(VisibleMoveAnimation {
                            trigger_message: message.clone(),
                            move_id: move_name.clone(),
                            animation_label,
                            player_move: *side == BattleSide::Player,
                            started: false,
                            frame: 0,
                            total_frames,
                            sound_events,
                            next_sound_event: 0,
                            cry_events,
                            next_cry_event: 0,
                            object_events,
                            bg_events,
                        });
                    }
                }
                Some(message)
            }
            BattleEvent::NoPp {
                side, move_name, ..
            } => Some(format!(
                "{} has no PP left for {}!",
                name(*side),
                battle_move_display_name(snapshot, move_name)
            )),
            BattleEvent::Missed { side, .. } => {
                Some(format!("{}'s attack missed!", name(*side)))
            }
            BattleEvent::AirborneAvoided { target, .. } => {
                Some(format!("{} evaded the attack!", name(*target)))
            }
            BattleEvent::NoEffect { side, .. } => {
                Some(format!("It doesn't affect {}!", name(side.other())))
            }
            BattleEvent::MagnitudePower { power, .. } => {
                let magnitude = match power {
                    0..=10 => 4,
                    11..=30 => 5,
                    31..=50 => 6,
                    51..=70 => 7,
                    71..=90 => 8,
                    91..=110 => 9,
                    _ => 10,
                };
                Some(format!("Magnitude {magnitude}!"))
            }
            BattleEvent::BeatUpParticipant { nickname, .. } => {
                Some(format!("{nickname}'s\nattack!"))
            }
            BattleEvent::MultiHitCount { hits, .. } => {
                Some(format!("Hit {hits} times!"))
            }
            BattleEvent::PayDayMoney { .. } => {
                Some("Coins scattered\neverywhere!".to_string())
            }
            BattleEvent::PresentFailed { target, .. } => {
                Some(format!("{} refused\nthe gift!", name(*target)))
            }
            BattleEvent::TeleportFailed { .. }
            | BattleEvent::StatusHealFailed { .. }
            | BattleEvent::ConfusionFailed { .. }
            | BattleEvent::LeechSeedFailed { .. }
            | BattleEvent::CurseFailed { .. }
            | BattleEvent::ConversionFailed { .. }
            | BattleEvent::Conversion2Failed { .. }
            | BattleEvent::MetronomeFailed { .. }
            | BattleEvent::MimicFailed { .. }
            | BattleEvent::SketchFailed { .. }
            | BattleEvent::SleepTalkFailed { .. }
            | BattleEvent::MirrorMoveFailed { .. }
            | BattleEvent::ForceSwitchFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::TrapFailed { .. } => None,
            BattleEvent::LeechSeedImmune { target, .. } => {
                Some(format!("It doesn't affect {}!", name(*target)))
            }
            BattleEvent::OhkoFailed { side, reason, .. } => Some(match reason {
                crate::core::battle::turn::OhkoFailureReason::TargetLevelTooHigh { .. } => {
                    format!("{}'s unaffected!", name(side.other()))
                }
                crate::core::battle::turn::OhkoFailureReason::Missed { .. } => {
                    "The attack missed!".to_string()
                }
            }),
            BattleEvent::Disobeyed { .. } if disobedience_nap => None,
            BattleEvent::Disobeyed { side } if disobedience_self_hit => {
                Some(format!("{} won't obey!", name(*side)))
            }
            BattleEvent::Disobeyed { side } => {
                Some(format!("{} ignored orders!", name(*side)))
            }
            BattleEvent::DisobedienceIdle { side, roll } => Some(format!(
                "{} {}",
                name(*side),
                match roll {
                    0 => "is loafing around!",
                    1 => "won't obey!",
                    2 => "turned away!",
                    _ => "ignored orders!",
                }
            )),
            BattleEvent::DisobedienceIgnoredSleeping { side } => {
                Some(format!("{} ignored orders…sleeping!", name(*side)))
            }
            BattleEvent::StatusApplied {
                target,
                status,
                move_name,
                ..
            } => {
                if move_name == "REST" {
                    None
                } else {
                let message = match status.as_str() {
                    "SLEEP" if move_name == "DISOBEDIENCE_NAP" => {
                        format!("{} began to nap!", name(*target))
                    }
                    "SLEEP" => format!("{} fell asleep!", name(*target)),
                    "POISON" | "BAD_POISON" => {
                        format!("{} was poisoned!", name(*target))
                    }
                    "BURN" => format!("{} was burned!", name(*target)),
                    "PARALYSIS" => format!("{} was paralyzed!", name(*target)),
                    "FREEZE" => format!("{} was frozen solid!", name(*target)),
                    _ => unreachable!("core emitted unsupported battle status {status}"),
                };
                Some(message)
                }
            }
            BattleEvent::StatusFailed {
                target,
                existing_status: None,
                ..
            } => Some(format!("It didn't affect {}!", name(*target))),
            BattleEvent::StatusFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::StatusHealed {
                target,
                move_name,
                status_before,
                ..
            } => {
                if move_name == "DEFROST" || status_before == "FREEZE" {
                    Some(format!("{} was defrosted!", name(*target)))
                } else {
                    Some(format!("{}'s {} was cured!", name(*target), status_before))
                }
            }
            BattleEvent::HealBellChimed { .. } => Some("A bell chimed!".to_string()),
            BattleEvent::StatusImmune { target, .. } => {
                Some(format!("It doesn't affect {}!", name(*target)))
            }
            BattleEvent::ResidualStatusDamage { side, status, .. } => {
                Some(if status == "BURN" {
                    format!("{}'s\nhurt by its burn!", name(*side))
                } else {
                    format!("{}\nis hurt by poison!", name(*side))
                })
            }
            BattleEvent::StatStageChanged {
                target,
                stat,
                amount,
                ..
            } => {
                let change = if *amount > 1 {
                    "went way up"
                } else if *amount > 0 {
                    "went up"
                } else if *amount < -1 {
                    "sharply fell"
                } else {
                    "fell"
                };
                Some(format!("{}'s {} {}!", name(*target), stat_name(stat), change))
            }
            BattleEvent::RageBuilding { side, .. } => {
                Some(format!("{}'s\nRAGE is building!", name(*side)))
            }
            BattleEvent::StatStageUnchanged {
                target,
                stat,
                amount,
                ..
            } => Some(format!(
                "{}'s {} won't\n{} anymore!",
                name(*target),
                stat_name(stat),
                if *amount >= 0 { "rise" } else { "drop" }
            )),
            BattleEvent::StatStageFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::ReflectApplied { side, .. } => {
                Some(format!("{} raised REFLECT!", name(*side)))
            }
            BattleEvent::LightScreenApplied { side, .. } => {
                Some(format!("{} raised LIGHT SCREEN!", name(*side)))
            }
            BattleEvent::SafeguardApplied { side, .. } => {
                Some(format!("{}'s covered by a veil!", name(*side)))
            }
            BattleEvent::SafeguardProtected { target, .. } => {
                Some(format!("{} is protected by SAFEGUARD!", name(*target)))
            }
            BattleEvent::SafeguardFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::SafeguardCount {
                side,
                turns_remaining: 0,
            } => Some(format!(
                "{} POKéMON's SAFEGUARD faded!",
                if *side == BattleSide::Player {
                    "Your"
                } else {
                    "Enemy"
                }
            )),
            BattleEvent::ReflectCount {
                side,
                turns_remaining: 0,
            } => Some(format!(
                "{} POKéMON's REFLECT faded!",
                if *side == BattleSide::Player {
                    "Your"
                } else {
                    "Enemy"
                }
            )),
            BattleEvent::LightScreenCount {
                side,
                turns_remaining: 0,
            } => Some(format!(
                "{} POKéMON's LIGHT SCREEN fell!",
                if *side == BattleSide::Player {
                    "Your"
                } else {
                    "Enemy"
                }
            )),
            BattleEvent::ReflectFailed { .. } | BattleEvent::LightScreenFailed { .. } => {
                Some("But it failed!".to_string())
            }
            BattleEvent::MistApplied { side, .. } => {
                Some(format!("{} became shrouded in MIST!", name(*side)))
            }
            BattleEvent::MistProtected { target, .. } => {
                Some(format!("{} is protected by MIST!", name(*target)))
            }
            BattleEvent::MistFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::LeechSeedApplied { target, .. } => {
                Some(format!("{} was seeded!", name(*target)))
            }
            BattleEvent::LeechSeedDamage { side, .. } => {
                Some(format!("LEECH SEED saps\n{}!", name(*side)))
            }
            BattleEvent::CurseDamage { side, .. } => {
                Some(format!("{} is hurt by the CURSE!", name(*side)))
            }
            BattleEvent::CurseApplied { side, target, .. } => Some(format!(
                "{} cut its own HP and put a CURSE on {}!",
                name(*side),
                name(*target)
            )),
            BattleEvent::NightmareDamage { side, .. } => {
                Some(format!("{} has a NIGHTMARE!", name(*side)))
            }
            BattleEvent::SpikesDamage { side, .. } => {
                Some(format!("{}'s hurt by SPIKES!", name(*side)))
            }
            BattleEvent::FutureSightLanded { side, .. } => {
                Some(format!("{} was hit by FUTURE SIGHT!", name(*side)))
            }
            BattleEvent::SandstormDamage { side, .. } => {
                Some(format!("The SANDSTORM hits\n{}!", name(*side)))
            }
            BattleEvent::SubstituteCreated { side, .. } => {
                Some(format!("{} made a SUBSTITUTE!", name(*side)))
            }
            BattleEvent::SubstituteDamaged { target, .. } => {
                Some(format!("The SUBSTITUTE took damage for {}!", name(*target)))
            }
            BattleEvent::SubstituteBroken { target, .. } => {
                Some(format!("{}'s SUBSTITUTE faded!", name(*target)))
            }
            BattleEvent::SubstituteBlocked { target, .. } => {
                Some(format!("It didn't affect {}!", name(*target)))
            }
            BattleEvent::SubstituteFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::TrapApplied {
                side,
                target,
                move_name,
                ..
            } => Some(match move_name.as_str() {
                "BIND" => format!("{} used BIND on {}!", name(*side), name(*target)),
                "WRAP" => format!("{} was WRAPPED by {}!", name(*target), name(*side)),
                "CLAMP" => format!("{} was CLAMPED by {}!", name(*target), name(*side)),
                "FIRE_SPIN" | "WHIRLPOOL" => format!("{} was trapped!", name(*target)),
                _ => format!(
                    "{} was trapped by {}!",
                    name(*target),
                    battle_move_display_name(snapshot, move_name)
                ),
            }),
            BattleEvent::TrapDamage {
                side, move_name, ..
            } => Some(format!(
                "{}'s hurt by {}!",
                name(*side),
                battle_move_display_name(snapshot, move_name)
            )),
            BattleEvent::TrapEnded { side, move_name, .. } => {
                Some(format!(
                    "{} was released from {}!",
                    name(*side),
                    battle_move_display_name(snapshot, move_name)
                ))
            }
            BattleEvent::EscapeTrapApplied { target, .. } => {
                Some(format!("{} can't escape now!", name(*target)))
            }
            BattleEvent::EscapeTrapEnded { .. } => None,
            BattleEvent::ConfusionApplied { target, .. } => {
                Some(format!("{} became confused!", name(*target)))
            }
            BattleEvent::ConfusedTurn { side, .. } => {
                Some(format!("{} is confused!", name(*side)))
            }
            BattleEvent::ConfusionEnded { side, .. } => {
                Some(format!("{} snapped out of confusion!", name(*side)))
            }
            BattleEvent::ConfusionSelfDamage { side, .. } => {
                Some(format!("{} hurt itself!", name(*side)))
            }
            BattleEvent::AttractApplied { target, .. } => {
                Some(format!("{} fell in love!", name(*target)))
            }
            BattleEvent::AttractFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::InfatuatedTurn { side, .. } => {
                Some(format!("{} is in love!", name(*side)))
            }
            BattleEvent::InfatuatedImmobilized { side, .. } => {
                Some(format!("{} is immobilized by love!", name(*side)))
            }
            BattleEvent::DisableApplied {
                target,
                disabled_move,
                ..
            } => Some(format!(
                "{}'s {} was DISABLED!",
                name(*target),
                battle_move_display_name(snapshot, disabled_move)
            )),
            BattleEvent::DisabledMove {
                side, move_name, ..
            } => Some(format!(
                "{}'s {} is DISABLED!",
                name(*side),
                battle_move_display_name(snapshot, move_name)
            )),
            BattleEvent::DisableEnded { side, .. } => {
                Some(format!("{}'s disabled no more!", name(*side)))
            }
            BattleEvent::DisableFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::EncoreApplied { target, .. } => {
                Some(format!("{} got an ENCORE!", name(*target)))
            }
            BattleEvent::EncoreEnded { side, .. } => {
                Some(format!("{}'s ENCORE ended!", name(*side)))
            }
            BattleEvent::EncoreFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::ProtectApplied { side, .. } => {
                Some(format!("{} PROTECTED itself!", name(*side)))
            }
            BattleEvent::MoveProtected { target, .. } => {
                Some(format!("{}'s PROTECTING itself!", name(*target)))
            }
            BattleEvent::ProtectFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::SpikesApplied { target, .. } => Some(format!(
                "SPIKES scattered all around {}!",
                name(*target)
            )),
            BattleEvent::SpikesFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::FutureSightQueued { side, .. } => {
                Some(format!("{} foresaw an attack!", name(*side)))
            }
            BattleEvent::FutureSightFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::PerishSongApplied { .. } => {
                Some("Both POKéMON will faint in 3 turns!".to_string())
            }
            BattleEvent::PerishSongCount {
                side,
                turns_remaining,
            } => Some(format!(
                "{}'s PERISH count is {}!",
                name(*side),
                turns_remaining
            )),
            BattleEvent::PerishSongFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::FocusEnergyApplied { side, .. } => {
                Some(format!("{}'s getting pumped!", name(*side)))
            }
            BattleEvent::FocusEnergyFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::ForesightApplied { side, target, .. } => {
                Some(format!("{} identified\n{}!", name(*side), name(*target)))
            }
            BattleEvent::ForesightFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::NightmareApplied { target, .. } => {
                Some(format!("{} started to have a\nNIGHTMARE!", name(*target)))
            }
            BattleEvent::NightmareFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::PsychUpApplied { side, target, .. } => Some(format!(
                "{} copied {}'s stat changes!",
                name(*side),
                name(*target)
            )),
            BattleEvent::TransformApplied {
                side, species, ..
            } => Some(format!(
                "{} TRANSFORMED into\n{}!",
                name(*side),
                crate::core::models::pokemon_species_display_name(species)
            )),
            BattleEvent::TransformFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::MimicApplied {
                side, copied_move, ..
            } => Some(format!(
                "{} learned {}!",
                name(*side),
                battle_move_display_name(snapshot, copied_move)
            )),
            BattleEvent::SketchApplied {
                side, copied_move, ..
            } => Some(format!(
                "{} SKETCHED {}!",
                name(*side),
                battle_move_display_name(snapshot, copied_move)
            )),
            BattleEvent::ConversionApplied { side, new_type, .. }
            | BattleEvent::Conversion2Applied { side, new_type, .. } => Some(format!(
                "{} transformed into the {}-type!",
                name(*side),
                new_type
            )),
            BattleEvent::StatsReset { .. } => {
                Some("All stat changes were eliminated!".to_string())
            }
            BattleEvent::LockOnApplied { side, .. } => {
                Some(format!("{} took aim!", name(*side)))
            }
            BattleEvent::DestinyBondApplied { side, .. } => {
                Some(format!(
                    "{}'s trying to take its\nopponent with it!",
                    name(*side)
                ))
            }
            BattleEvent::DestinyBondActivated { side, source, .. } => Some(format!(
                "{} took down with it, {}!",
                name(*side),
                name(*source)
            )),
            BattleEvent::EndureApplied { side, .. } => {
                Some(format!("{} braced itself!", name(*side)))
            }
            BattleEvent::EndureFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::EnduredHit {
                target,
                held_item: Some(item_id),
                ..
            } => Some(format!(
                "{} hung on with\n{}!",
                name(*target),
                item_display_name(snapshot, item_id)
            )),
            BattleEvent::EnduredHit { target, .. } => {
                Some(format!("{} ENDURED the hit!", name(*target)))
            }
            BattleEvent::BideStarted { side, .. } | BattleEvent::BideStoring { side, .. } => {
                Some(format!("{} is storing energy!", name(*side)))
            }
            BattleEvent::BideReleased { side, .. } => {
                Some(format!("{} unleashed energy!", name(*side)))
            }
            BattleEvent::BideFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::SpiteApplied {
                target,
                target_move,
                reduction,
                ..
            } => Some(format!(
                "{}'s {} was reduced by {}!",
                name(*target),
                battle_move_display_name(snapshot, target_move),
                reduction
            )),
            BattleEvent::SpiteFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::Splash { .. } => Some("But nothing happened!".to_string()),
            BattleEvent::HealApplied {
                side, move_name, ..
            } => {
                if move_name == "LEFTOVERS" {
                    Some(format!("{} regained health with Leftovers!", name(*side)))
                } else if move_name == "REST" {
                    let had_status = match side {
                        BattleSide::Player => snapshot
                            .battle
                            .as_ref()
                            .and_then(|battle| battle.active_player_party_index)
                            .and_then(|active| {
                                snapshot.party.slots.iter().find(|slot| slot.index == active)
                            })
                            .is_some_and(|slot| slot.pokemon.status.is_some()),
                        BattleSide::Enemy => snapshot
                            .battle
                            .as_ref()
                            .is_some_and(|battle| battle.enemy_pokemon.status.is_some()),
                    };
                    Some(if had_status {
                        format!("{} fell asleep and became healthy!", name(*side))
                    } else {
                        format!("{} went to sleep!", name(*side))
                    })
                } else {
                    Some(format!("{} regained health!", name(*side)))
                }
            }
            BattleEvent::PresentHeal { target, .. } => {
                Some(format!("{} had its HP restored!", name(*target)))
            }
            BattleEvent::HpDrained { target, .. } => {
                Some(format!("Sucked health from\n{}!", name(*target)))
            }
            BattleEvent::PainSplitApplied { .. } => {
                Some("The battlers\nshared pain!".to_string())
            }
            BattleEvent::HeldItemHpHealed {
                side, item_id, ..
            } => Some(format!(
                "{} recovered using a {}!",
                name(*side),
                item_display_name(snapshot, item_id)
            )),
            BattleEvent::HeldItemPpRestored {
                side,
                item_id,
                ..
            } => Some(format!(
                "{} recovered PP using {}.",
                name(*side),
                item_display_name(snapshot, item_id)
            )),
            BattleEvent::HeldItemStatusHealed {
                side,
                item_id,
                status_before,
                confusion_turns_before,
                ..
            } => {
                let display_name = item_display_name(snapshot, item_id);
                if status_before.is_none() && *confusion_turns_before > 0 {
                    Some(format!(
                        "A {} rid {} of its confusion.",
                        display_name,
                        name(*side)
                    ))
                } else {
                    Some(format!(
                        "{} recovered using a {}!",
                        name(*side),
                        display_name
                    ))
                }
            }
            BattleEvent::HealFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::RecoilDamage { side, .. } => {
                Some(format!("{}'s\nhit with recoil!", name(*side)))
            }
            BattleEvent::JumpKickCrash { side, .. } => {
                Some(format!("{} kept going and crashed!", name(*side)))
            }
            BattleEvent::SleepTurn { side, .. } => {
                Some(format!("{} is fast asleep!", name(*side)))
            }
            BattleEvent::RechargeTurn { side, .. } => {
                Some(format!("{} must recharge!", name(*side)))
            }
            BattleEvent::ChargeStarted { side, move_name } => {
                let action = match move_name.as_str() {
                    "SOLARBEAM" => "took in sunlight!",
                    "SKULL_BASH" => "lowered its head!",
                    "SKY_ATTACK" => "is glowing!",
                    "RAZOR_WIND" => "whipped up a whirlwind!",
                    _ => "began charging power!",
                };
                Some(format!("{} {action}", name(*side)))
            }
            BattleEvent::AirborneStarted { side, move_name } => {
                let action = if move_name == "DIG" {
                    "dug a hole!"
                } else {
                    "flew up high!"
                };
                Some(format!("{} {action}", name(*side)))
            }
            BattleEvent::WeatherApplied { weather, .. } => {
                Some(match weather {
                    Weather::Rain => "A downpour started!",
                    Weather::Sun => "The sunlight got bright!",
                    Weather::Sandstorm => "A sandstorm brewed!",
                    Weather::None => "The weather returned to normal.",
                }
                .to_string())
            }
            BattleEvent::WeatherContinues { weather, .. } => {
                Some(match weather {
                    Weather::Rain => "Rain continues to fall.",
                    Weather::Sun => "The sunlight is strong.",
                    Weather::Sandstorm => "The sandstorm rages.",
                    Weather::None => "The weather returned to normal.",
                }
                .to_string())
            }
            BattleEvent::WeatherEnded { weather } => Some(match weather {
                Weather::Rain => "The rain stopped.",
                Weather::Sun => "The sunlight faded.",
                Weather::Sandstorm => "The SANDSTORM subsided.",
                Weather::None => unreachable!("core ended WEATHER_NONE"),
            }
            .to_string()),
            BattleEvent::ItemUsed { side, item_id } => {
                let item_name = item_display_name(snapshot, item_id);
                if *side == BattleSide::Enemy {
                    Some(format!("{} used {item_name}!", name(*side)))
                } else {
                    Some(format!(
                        "{} used the {item_name}.",
                        visible_battle_player_name(snapshot)
                    ))
                }
            }
            BattleEvent::BattleItemEffect { side, outcome } => {
                if outcome.item_id == "GUARD_SPEC" {
                    Some(format!("{} became shrouded in MIST!", name(*side)))
                } else if let Some(change) = outcome.battle_stat_stage_changes.first() {
                    let stat = battle_stat_display_name(&change.stat);
                    Some(format!(
                        "{}'s {stat} {}!",
                        name(*side),
                        if change.stage_after > change.stage_before {
                            "rose"
                        } else {
                            "fell"
                        }
                    ))
                } else if !outcome.focus_energy_before && outcome.focus_energy_after {
                    Some(format!("{} is getting pumped!", name(*side)))
                } else {
                    None
                }
            }
            BattleEvent::HeldItemStolen { side, item_id, .. } => Some(format!(
                "{} stole {} from its foe!",
                name(*side),
                item_display_name(snapshot, item_id)
            )),
            BattleEvent::HeldItemStealFailed { .. } => Some("But it failed!".to_string()),
            BattleEvent::HeldItemActivated { side, item_id, .. } => Some(format!(
                "{}'s {} activated!",
                name(*side),
                item_display_name(snapshot, item_id)
            )),
            BattleEvent::SwitchBlocked { side, .. } => {
                Some(format!("{} can't be recalled!", name(*side)))
            }
            BattleEvent::RunBlocked { .. } | BattleEvent::RunPrevented { .. } => {
                Some("Can't escape!".to_string())
            }
            BattleEvent::RunAttempt { outcome, .. } if !outcome.escaped => {
                Some("Can't escape!".to_string())
            }
            BattleEvent::RunAttempt { side, outcome } if outcome.escaped => {
                if let Some((item_side, item_id)) = held_escape_item.take() {
                    debug_assert_eq!(item_side, *side);
                    Some(format!(
                        "{} fled using a {}!",
                        name(*side),
                        item_display_name(snapshot, &item_id)
                    ))
                } else {
                    Some("Got away safely!".to_string())
                }
            }
            BattleEvent::Fled { side } => {
                if teleported_sides.contains(side) {
                    Some(format!("{} fled from battle!", name(*side)))
                } else if let Some(move_name) = forced_switches.get(side) {
                    Some(if *move_name == "ROAR" {
                        format!("{} fled in fear!", name(*side))
                    } else {
                        format!("{} was blown away!", name(*side))
                    })
                } else {
                    Some(format!("{} got away safely!", name(*side)))
                }
            }
            BattleEvent::Fainted { side } => {
                let message = match side {
                    BattleSide::Player => format!("{} fainted!", name(*side)),
                    BattleSide::Enemy => format!("Enemy {} fainted!", name(*side)),
                };
                runtime_shell.visible_move_animations.push_back(VisibleMoveAnimation {
                    trigger_message: message.clone(),
                    move_id: "FAINT_MON".to_string(),
                    animation_label: "BattleAnim_FaintMon".to_string(),
                    player_move: *side == BattleSide::Player,
                    started: false,
                    frame: 0,
                    total_frames: 24,
                    sound_events: vec![(0, "SFX_FAINT".to_string())],
                    next_sound_event: 0,
                    cry_events: Vec::new(),
                    next_cry_event: 0,
                    object_events: Vec::new(),
                    bg_events: vec![VisibleMoveBgEvent {
                        frame: 0,
                        effect_id: "BATTLE_BG_EFFECT_FAINT_MON".to_string(),
                        duration: 20,
                        target: "BG_EFFECT_USER".to_string(),
                        param: 4,
                        incremented: false,
                    }],
                });
                Some(message)
            }
            BattleEvent::Switched { side, party_index } if forced_switches.contains_key(side) => {
                match side {
                    BattleSide::Player => snapshot
                        .party
                        .slots
                        .iter()
                        .find(|slot| slot.index == *party_index)
                        .map(|slot| format!("{} was dragged out!", slot.pokemon.nickname)),
                    BattleSide::Enemy => snapshot
                        .battle
                        .as_ref()
                        .and_then(|battle| battle.enemy_party.get(*party_index))
                        .map(|pokemon| format!("{} was dragged out!", pokemon.nickname)),
                }
            }
            BattleEvent::Switched { side, party_index } => match side {
                BattleSide::Player => snapshot
                    .party
                    .slots
                    .iter()
                    .find(|slot| slot.index == *party_index)
                    .map(|slot| format!("Go! {}!", slot.pokemon.nickname)),
                BattleSide::Enemy => snapshot
                    .battle
                    .as_ref()
                    .and_then(|battle| {
                        battle
                            .enemy_party
                            .get(*party_index)
                            .map(|pokemon| (battle, pokemon))
                    })
                    .map(|(battle, pokemon)| match &battle.kind {
                        RuntimeBattleKind::Trainer { trainer_name, .. } => {
                            format!("{trainer_name} sent out {}!", pokemon.nickname)
                        }
                        _ => format!("{} was dragged out!", pokemon.nickname),
                    }),
            },
            BattleEvent::FullyParalyzed { side, .. } => {
                Some(format!("{} is fully paralyzed!", name(*side)))
            }
            BattleEvent::Flinched { side, .. } => {
                Some(format!("{} flinched!", name(*side)))
            }
            BattleEvent::FrozenTurn { side, .. } => {
                Some(format!("{} is frozen solid!", name(*side)))
            }
            BattleEvent::WokeUp { side, .. } => Some(format!("{} woke up!", name(*side))),
            _ => None,
        };
        if let Some(message) = message {
            runtime_shell.battle_messages.push_back(message);
        }
        if stage_message_scenes {
            for _ in event_message_count_before..runtime_shell.battle_messages.len() {
                runtime_shell
                    .battle_message_scenes
                    .push_back(Box::new(if matches!(event, BattleEvent::Fainted { .. }) {
                        event_scene_before.clone()
                    } else {
                        event_scene.clone()
                    }));
            }
            if let BattleEvent::Fainted { side } = event {
                let trigger = match side {
                    BattleSide::Player => format!("{} fainted!", name(*side)),
                    BattleSide::Enemy => format!("Enemy {} fainted!", name(*side)),
                };
                runtime_shell
                    .pending_battle_scenes_after_message
                    .push_back((trigger, Box::new(event_scene.clone())));
            }
            if event_message_count_before == runtime_shell.battle_messages.len()
                && battle_event_changes_visible_scene(event)
            {
                // Several Crystal effects deliberately have no additional
                // textbox after their initiating/result line. Keep their HP
                // or battler mutation on that preceding visible boundary so
                // the HUD animates before control returns instead of jumping
                // only after the message queue vanishes.
                if let Some(scene) = runtime_shell.battle_message_scenes.back_mut() {
                    *scene = Box::new(event_scene.clone());
                }
            }
        }
        if let BattleEvent::Switched { side, party_index } = event {
            let switched_name = match side {
                BattleSide::Player => snapshot
                    .party
                    .slots
                    .iter()
                    .find(|slot| slot.index == *party_index)
                    .map(|slot| slot.pokemon.nickname.clone()),
                BattleSide::Enemy => snapshot
                    .battle
                    .as_ref()
                    .and_then(|battle| battle.enemy_party.get(*party_index))
                    .map(|pokemon| pokemon.nickname.clone()),
            };
            if let Some(switched_name) = switched_name {
                match side {
                    BattleSide::Player => *player_name.borrow_mut() = switched_name,
                    BattleSide::Enemy => *enemy_name.borrow_mut() = switched_name,
                }
            }
        }
    }
    let has_visible_scene_change = events.iter().any(battle_event_changes_visible_scene);
    if runtime_shell.battle_messages.len() > message_count_before || has_visible_scene_change {
        let mut scene = snapshot.clone();
        for event in events {
            apply_visible_battle_event_to_scene(&mut scene, event, &mut baton_pass_sides);
        }
        let old_player_pixels = snapshot
            .battle
            .as_ref()
            .and_then(|battle| battle.active_player_party_index)
            .and_then(|index| snapshot.party.slots.iter().find(|slot| slot.index == index))
            .map(|slot| battle_hud_hp_pixels(slot.pokemon.hp, slot.pokemon.max_hp))
            .unwrap_or(0);
        let old_enemy_pixels = snapshot
            .battle
            .as_ref()
            .map(|battle| battle_hud_hp_pixels(battle.enemy_pokemon.hp, battle.enemy_pokemon.max_hp))
            .unwrap_or(0);
        let displayed_scene = if stage_message_scenes {
            runtime_shell
                .battle_message_scenes
                .front()
                .map(|scene| scene.as_ref())
                .unwrap_or(&scene)
        } else {
            &scene
        };
        let new_player_pixels = displayed_scene
            .battle
            .as_ref()
            .and_then(|battle| battle.active_player_party_index)
            .and_then(|index| {
                displayed_scene
                    .party
                    .slots
                    .iter()
                    .find(|slot| slot.index == index)
            })
            .map(|slot| battle_hud_hp_pixels(slot.pokemon.hp, slot.pokemon.max_hp))
            .unwrap_or(old_player_pixels);
        let new_enemy_pixels = displayed_scene
            .battle
            .as_ref()
            .map(|battle| battle_hud_hp_pixels(battle.enemy_pokemon.hp, battle.enemy_pokemon.max_hp))
            .unwrap_or(old_enemy_pixels);
        let tween = runtime_shell
            .battle_hp_tween
            .get_or_insert(VisibleBattleHpTween {
                player_pixels: old_player_pixels,
                player_target_pixels: old_player_pixels,
                player_frames_until_step: 0,
                enemy_pixels: old_enemy_pixels,
                enemy_target_pixels: old_enemy_pixels,
                enemy_frames_until_step: 0,
            });
        tween.player_target_pixels = new_player_pixels;
        tween.enemy_target_pixels = new_enemy_pixels;
        tween.player_frames_until_step = 0;
        tween.enemy_frames_until_step = 0;
        for event in events {
            if let BattleEvent::Switched { side, .. } = event {
                match side {
                    BattleSide::Player => tween.player_pixels = new_player_pixels,
                    BattleSide::Enemy => tween.enemy_pixels = new_enemy_pixels,
                }
            }
        }
        runtime_shell.battle_message_scene = if stage_message_scenes {
            runtime_shell
                .battle_message_scenes
                .front()
                .cloned()
                .or_else(|| Some(Box::new(scene)))
        } else {
            Some(Box::new(scene))
        };
    }
    mark_runtime_snapshot_dirty(runtime_shell);
}

fn visible_move_animation_definition(
    snapshot: &RuntimeShellSnapshot,
    move_id: &str,
) -> Option<(
    String,
    u16,
    Vec<(u16, String)>,
    Vec<(u16, u8)>,
    Vec<VisibleMoveObjectEvent>,
    Vec<VisibleMoveBgEvent>,
)> {
    let normalized = move_id.replace(' ', "_").to_ascii_uppercase();
    let move_index = snapshot
        .presentation
        .move_names
        .iter()
        .position(|name| name.replace(' ', "_").to_ascii_uppercase() == normalized)?;
    // BattleAnimations begins with the status-animation entry; move one is
    // therefore table entry one, matching TypeScript's table.slice(1).
    let label = snapshot
        .presentation
        .battle_animation_table
        .get(move_index.saturating_add(1))?
        .clone();
    let (timeline_frame, sound_events, cry_events, object_events, bg_events) =
        compile_visible_battle_animation_timeline(snapshot, &label)?;
    // TypeScript/ASM execute consecutive commands in one update and yield only
    // at an explicit wait (or while a live object/background effect continues).
    let final_sound_frame = sound_events.last().map_or(0, |(frame, _)| *frame);
    let final_cry_frame = cry_events.last().map_or(0, |(frame, _)| *frame);
    let bundle = serde_json::from_str::<serde_json::Value>(
        &snapshot.presentation.battle_anim_bundle,
    )
    .ok()?;
    let final_object_frame = object_events
        .iter()
        .filter_map(|event| {
            let VisibleMoveObjectCommand::Spawn { object_id, .. } = &event.command else {
                return None;
            };
            let object = bundle.get("objects")?.get(object_id)?;
            if object
                .get("function")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|function| function != "BATTLE_ANIM_FUNC_NULL")
            {
                return None;
            }
            let frameset = object.get("frameset")?.as_str()?;
            let lifetime = visible_null_battle_animation_object_lifetime(&bundle, frameset)?;
            let natural_end = event.frame.saturating_add(lifetime.saturating_sub(1));
            let cleared_at = object_events
                .iter()
                .find(|candidate| {
                    candidate.frame >= event.frame
                        && matches!(&candidate.command, VisibleMoveObjectCommand::Clear)
                })
                .map(|candidate| candidate.frame);
            Some(cleared_at.map_or(natural_end, |clear| natural_end.min(clear)))
        })
        .max()
        .unwrap_or(0);
    let final_bg_frame = bg_events
        .iter()
        .filter(|effect| !effect.incremented)
        .filter_map(|effect| {
            let terminating_increment = bg_events
                .iter()
                .find(|candidate| {
                    candidate.incremented
                        && candidate.effect_id == effect.effect_id
                        && candidate.frame >= effect.frame
                })
                .map(|candidate| candidate.frame);
            let lifetime = match effect.effect_id.as_str() {
                "BATTLE_BG_EFFECT_TACKLE"
                | "BATTLE_BG_EFFECT_BODY_SLAM"
                | "BATTLE_BG_EFFECT_BETA_PURSUIT"
                | "BATTLE_BG_EFFECT_ROLLOUT"
                | "BATTLE_BG_EFFECT_VITAL_THROW" => 8,
                "BATTLE_BG_EFFECT_SHAKE_SCREEN_X" | "BATTLE_BG_EFFECT_SHAKE_SCREEN_Y" => {
                    effect.duration.max(1)
                }
                "BATTLE_BG_EFFECT_FLASH_INVERTED" | "BATTLE_BG_EFFECT_FLASH_WHITE" => {
                    if effect.duration == 0 { 4 } else { effect.duration }
                }
                "BATTLE_BG_EFFECT_WHITE_HUES" | "BATTLE_BG_EFFECT_BLACK_HUES" => {
                    effect.duration.max(1)
                }
                "BATTLE_BG_EFFECT_ALTERNATE_HUES"
                | "BATTLE_BG_EFFECT_CYCLE_BGPALS_INVERTED"
                | "BATTLE_BG_EFFECT_ACID_ARMOR" => {
                    if effect.duration == 0 { 4 } else { effect.duration }
                }
                "BATTLE_BG_EFFECT_CYCLE_OBPALS_GRAY_AND_YELLOW"
                | "BATTLE_BG_EFFECT_CYCLE_MID_OBPALS_GRAY_AND_YELLOW"
                | "BATTLE_BG_EFFECT_CYCLE_MON_LIGHT_DARK_REPEATING" => {
                    if effect.duration == 0 { 6 } else { effect.duration }
                }
                "BATTLE_BG_EFFECT_START_WATER"
                | "BATTLE_BG_EFFECT_WATER"
                | "BATTLE_BG_EFFECT_END_WATER"
                | "BATTLE_BG_EFFECT_WHIRLPOOL" => {
                    if effect.duration == 0 { 6 } else { effect.duration }
                }
                "BATTLE_BG_EFFECT_NIGHT_SHADE" | "BATTLE_BG_EFFECT_TELEPORT" => {
                    effect.duration.max(1)
                }
                "BATTLE_BG_EFFECT_PSYCHIC" => {
                    if effect.duration == 0 { 4 } else { effect.duration }
                }
                "BATTLE_BG_EFFECT_WOBBLE_MON" | "BATTLE_BG_EFFECT_WAVE_DEFORM_MON" => {
                    if effect.duration == 0 { 6 } else { effect.duration }
                }
                "BATTLE_BG_EFFECT_WOBBLE_PLAYER" | "BATTLE_BG_EFFECT_WOBBLE_SCREEN" => {
                    if effect.duration == 0 { 8 } else { effect.duration }
                }
                "BATTLE_BG_EFFECT_VIBRATE_MON" => effect.duration.max(2),
                "BATTLE_BG_EFFECT_DIG" | "BATTLE_BG_EFFECT_FLAIL" | "BATTLE_BG_EFFECT_DOUBLE_TEAM" => {
                    if effect.duration == 0 {
                        u16::from(if effect.param == 0 { 3 } else { effect.param })
                            .saturating_mul(2)
                    } else {
                        effect.duration
                    }
                }
                "BATTLE_BG_EFFECT_BOUNCE_DOWN" => 4,
                "BATTLE_BG_EFFECT_REMOVE_MON" => {
                    if effect.duration == 0 { 6 } else { effect.duration }
                }
                "BATTLE_BG_EFFECT_FAINT_MON" => {
                    if effect.duration == 0 { 14 } else { effect.duration }
                }
                "BATTLE_BG_EFFECT_BETA_SEND_OUT_MON1" | "BATTLE_BG_EFFECT_BETA_SEND_OUT_MON2" => {
                    if effect.duration == 0 { 6 } else { effect.duration }
                }
                "BATTLE_BG_EFFECT_ENTER_MON" => effect.duration.max(1).saturating_mul(3),
                "BATTLE_BG_EFFECT_RETURN_MON" => effect.duration.max(1).saturating_mul(4),
                "BATTLE_BG_EFFECT_BATTLEROBJ_1ROW" | "BATTLE_BG_EFFECT_BATTLEROBJ_2ROW" => {
                    if effect.duration == 0 { 6 } else { effect.duration }
                }
                "BATTLE_BG_EFFECT_FADE_MON_TO_LIGHT" | "BATTLE_BG_EFFECT_FADE_MON_TO_BLACK" => {
                    u16::from((effect.param >> 4).max(1)).saturating_mul(3)
                }
                "BATTLE_BG_EFFECT_FADE_MON_TO_LIGHT_REPEATING"
                | "BATTLE_BG_EFFECT_FADE_MON_TO_BLACK_REPEATING"
                | "BATTLE_BG_EFFECT_FADE_MONS_TO_BLACK_REPEATING" => {
                    return terminating_increment.or(Some(timeline_frame));
                }
                "BATTLE_BG_EFFECT_FADE_MON_TO_WHITE_WAIT_FADE_BACK" => {
                    let step_delay = u16::from((effect.param >> 4).max(1));
                    let initial_delay = u16::from((effect.param & 0x0f).max(effect.param >> 4).max(1));
                    let natural_end = effect.frame
                        .saturating_add(initial_delay)
                        .saturating_add(step_delay.saturating_mul(16));
                    return Some(terminating_increment.map_or(natural_end, |frame| frame.min(natural_end)));
                }
                "BATTLE_BG_EFFECT_RAPID_FLASH"
                | "BATTLE_BG_EFFECT_FLASH_MON_REPEATING" => {
                    return terminating_increment.or(Some(timeline_frame));
                }
                "BATTLE_BG_EFFECT_FADE_MON_FROM_WHITE" => {
                    if effect.duration == 0 { 8 } else { effect.duration }
                }
                _ => return None,
            };
            let last_reset = bg_events
                .iter()
                .filter(|candidate| {
                    candidate.incremented
                        && candidate.effect_id == effect.effect_id
                        && candidate.frame >= effect.frame
                })
                .map(|candidate| candidate.frame)
                .max()
                .unwrap_or(effect.frame);
            Some(last_reset.saturating_add(lifetime.saturating_sub(1)))
        })
        .max()
        .unwrap_or(0);
    Some((
        label,
        timeline_frame
            .max(final_sound_frame)
            .max(final_cry_frame)
            .max(final_object_frame)
            .max(final_bg_frame)
            .max(1),
        sound_events,
        cry_events,
        object_events,
        bg_events,
    ))
}

#[derive(Default)]
struct VisibleBattleAnimationTimeline {
    frame: u16,
    sounds: Vec<(u16, String)>,
    cries: Vec<(u16, u8)>,
    objects: Vec<VisibleMoveObjectEvent>,
    bg_effects: Vec<VisibleMoveBgEvent>,
    loops: std::collections::BTreeMap<(String, usize), i32>,
    anim_var: i32,
    anim_param: i32,
    commands_executed: usize,
}

fn compile_visible_battle_animation_timeline(
    snapshot: &RuntimeShellSnapshot,
    root_label: &str,
) -> Option<(
    u16,
    Vec<(u16, String)>,
    Vec<(u16, u8)>,
    Vec<VisibleMoveObjectEvent>,
    Vec<VisibleMoveBgEvent>,
)> {
    let mut timeline = VisibleBattleAnimationTimeline::default();
    execute_visible_battle_animation_script(snapshot, root_label, &mut timeline, 0)?;
    Some((
        timeline.frame,
        timeline.sounds,
        timeline.cries,
        timeline.objects,
        timeline.bg_effects,
    ))
}

fn execute_visible_battle_animation_script(
    snapshot: &RuntimeShellSnapshot,
    script_label: &str,
    timeline: &mut VisibleBattleAnimationTimeline,
    depth: usize,
) -> Option<()> {
    if depth > 32 {
        return None;
    }
    let source = snapshot.presentation.battle_animations.get(script_label)?;
    let mut labels = std::collections::BTreeMap::<String, usize>::new();
    let mut commands = Vec::<String>::new();
    for line in source {
        let trimmed = line.trim();
        if trimmed.starts_with('.') && !trimmed.contains(char::is_whitespace) {
            labels.insert(trimmed.to_string(), commands.len());
        } else {
            commands.push(trimmed.to_string());
        }
    }
    let mut pointer = 0_usize;
    while pointer < commands.len() {
        timeline.commands_executed = timeline.commands_executed.saturating_add(1);
        if timeline.commands_executed > 65_535 {
            return None;
        }
        let command_index = pointer;
        let command = &commands[pointer];
        pointer += 1;
        let (opcode, raw_arguments) = command
            .split_once(char::is_whitespace)
            .map_or((command.as_str(), ""), |(opcode, arguments)| (opcode, arguments));
        let arguments = raw_arguments
            .split(',')
            .map(str::trim)
            .filter(|argument| !argument.is_empty())
            .collect::<Vec<_>>();
        match opcode {
            "anim_wait" => {
                let frames = arguments
                    .first()
                    .and_then(|argument| parse_visible_battle_animation_int(argument))
                    .and_then(|frames| u16::try_from(frames).ok())?;
                timeline.frame = timeline.frame.saturating_add(frames);
            }
            "anim_sound" => {
                if let Some(sound) = arguments.get(2) {
                    timeline
                        .sounds
                        .push((timeline.frame.saturating_add(1), (*sound).to_string()));
                }
            }
            "playsound" => {
                if let Some(sound) = arguments.first() {
                    timeline
                        .sounds
                        .push((timeline.frame.saturating_add(1), (*sound).to_string()));
                }
            }
            "anim_cry" => {
                let selector = arguments
                    .first()
                    .and_then(|argument| parse_visible_battle_animation_int(argument))
                    .unwrap_or(0);
                timeline.cries.push((
                    timeline.frame.saturating_add(1),
                    u8::try_from(selector & 0x03).ok()?,
                ));
            }
            "anim_obj" if arguments.len() >= 4 => {
                let (x, y, param) = if arguments.len() >= 6 {
                    (
                        parse_visible_battle_animation_int(arguments[1])?
                            .saturating_mul(8)
                            .saturating_add(parse_visible_battle_animation_int(arguments[2])?),
                        parse_visible_battle_animation_int(arguments[3])?
                            .saturating_mul(8)
                            .saturating_add(parse_visible_battle_animation_int(arguments[4])?),
                        parse_visible_battle_animation_int(arguments[5])?,
                    )
                } else {
                    (
                        parse_visible_battle_animation_int(arguments[1])?,
                        parse_visible_battle_animation_int(arguments[2])?,
                        parse_visible_battle_animation_int(arguments[3])?,
                    )
                };
                timeline.objects.push(VisibleMoveObjectEvent {
                    frame: timeline.frame.saturating_add(1),
                    command: VisibleMoveObjectCommand::Spawn {
                        object_id: arguments[0].to_string(),
                        x: i16::try_from(x).ok()?,
                        y: i16::try_from(y).ok()?,
                        param: u8::try_from(param & 0xff).ok()?,
                    },
                });
            }
            "anim_clearobjs" => timeline.objects.push(VisibleMoveObjectEvent {
                frame: timeline.frame.saturating_add(1),
                command: VisibleMoveObjectCommand::Clear,
            }),
            "anim_incobj" => {
                let slot = parse_visible_battle_animation_int(arguments.first()?)?;
                timeline.objects.push(VisibleMoveObjectEvent {
                    frame: timeline.frame.saturating_add(1),
                    command: VisibleMoveObjectCommand::Increment {
                        slot: u8::try_from(slot).ok()?,
                    },
                });
            }
            "anim_setobj" => {
                let slot = parse_visible_battle_animation_int(arguments.first()?)?;
                let value = parse_visible_battle_animation_int(arguments.get(1)?)?;
                timeline.objects.push(VisibleMoveObjectEvent {
                    frame: timeline.frame.saturating_add(1),
                    command: VisibleMoveObjectCommand::Set {
                        slot: u8::try_from(slot).ok()?,
                        value: u8::try_from(value & 0xff).ok()?,
                    },
                });
            }
            "anim_transform" | "anim_raisesub" | "anim_dropsub" | "anim_minimize"
            | "anim_minimizeopp" | "anim_updateactorpic" => {
                timeline.bg_effects.push(VisibleMoveBgEvent {
                    frame: timeline.frame.saturating_add(1),
                    effect_id: format!("BATTLE_ACTOR_{}", opcode.trim_start_matches("anim_").to_ascii_uppercase()),
                    duration: 0,
                    target: "BG_EFFECT_USER".to_string(),
                    param: 0,
                    incremented: false,
                });
            }
            "anim_bgp" | "anim_obp0" | "anim_obp1" => {
                let value = parse_visible_battle_animation_int(arguments.first()?)?;
                timeline.bg_effects.push(VisibleMoveBgEvent {
                    frame: timeline.frame.saturating_add(1),
                    effect_id: format!("BATTLE_PALETTE_{}", opcode.trim_start_matches("anim_").to_ascii_uppercase()),
                    duration: 0,
                    target: String::new(),
                    param: u8::try_from(value & 0xff).ok()?,
                    incremented: false,
                });
            }
            "anim_resetobp0" => timeline.bg_effects.push(VisibleMoveBgEvent {
                frame: timeline.frame.saturating_add(1),
                effect_id: "BATTLE_PALETTE_OBP0".to_string(),
                duration: 0,
                target: String::new(),
                param: 0xe4,
                incremented: false,
            }),
            "anim_1gfx" | "anim_2gfx" | "anim_3gfx" | "anim_battlergfx_1row"
            | "anim_battlergfx_2row" | "anim_beatup" | "anim_checkpokeball"
            | "anim_keepsprites" => {}
            "anim_bgeffect" if arguments.len() >= 4 => {
                let duration = parse_visible_battle_animation_int(arguments[1])?;
                let param = parse_visible_battle_animation_int(arguments[3])?;
                timeline.bg_effects.push(VisibleMoveBgEvent {
                    frame: timeline.frame.saturating_add(1),
                    effect_id: arguments[0].to_string(),
                    duration: u16::try_from(duration & 0xffff).ok()?,
                    target: arguments[2].to_string(),
                    param: u8::try_from(param & 0xff).ok()?,
                    incremented: false,
                });
            }
            "anim_incbgeffect" => {
                timeline.bg_effects.push(VisibleMoveBgEvent {
                    frame: timeline.frame.saturating_add(1),
                    effect_id: arguments.first()?.to_string(),
                    duration: 0,
                    target: String::new(),
                    param: 0,
                    incremented: true,
                });
            }
            "anim_call" => {
                let target = *arguments.first()?;
                if target.starts_with('.') {
                    return None;
                }
                execute_visible_battle_animation_script(snapshot, target, timeline, depth + 1)?;
            }
            "anim_jump" => {
                let target = *arguments.first()?;
                if target.starts_with('.') {
                    pointer = *labels.get(target)?;
                } else {
                    execute_visible_battle_animation_script(snapshot, target, timeline, depth + 1)?;
                    return Some(());
                }
            }
            "anim_loop" => {
                let count = parse_visible_battle_animation_int(arguments.first()?)?;
                let target = *arguments.get(1)?;
                let key = (script_label.to_string(), command_index);
                if let Some(remaining) = timeline.loops.get_mut(&key) {
                    if *remaining < 0 {
                        pointer = *labels.get(target)?;
                    } else if *remaining > 0 {
                        *remaining -= 1;
                        pointer = *labels.get(target)?;
                    } else {
                        timeline.loops.remove(&key);
                    }
                } else {
                    timeline.loops.insert(key, if count <= 0 { -1 } else { count - 1 });
                    pointer = *labels.get(target)?;
                }
            }
            "anim_setvar" => {
                timeline.anim_var = parse_visible_battle_animation_int(arguments.first()?)?;
            }
            "anim_incvar" => timeline.anim_var = (timeline.anim_var + 1) & 0xff,
            "anim_if_var_equal" => {
                let value = parse_visible_battle_animation_int(arguments.first()?)?;
                if timeline.anim_var == value {
                    let target = *arguments.get(1)?;
                    if target.starts_with('.') {
                        pointer = *labels.get(target)?;
                    } else {
                        execute_visible_battle_animation_script(snapshot, target, timeline, depth + 1)?;
                        return Some(());
                    }
                }
            }
            "anim_if_param_equal" => {
                let value = parse_visible_battle_animation_int(arguments.first()?)?;
                if timeline.anim_param == value {
                    let target = *arguments.get(1)?;
                    if target.starts_with('.') {
                        pointer = *labels.get(target)?;
                    } else {
                        execute_visible_battle_animation_script(snapshot, target, timeline, depth + 1)?;
                        return Some(());
                    }
                }
            }
            "anim_if_param_and" => {
                let mask = parse_visible_battle_animation_int(arguments.first()?)?;
                if timeline.anim_param & mask != 0 {
                    pointer = *labels.get(*arguments.get(1)?)?;
                }
            }
            "anim_jumpuntil" => {
                if timeline.anim_param > 0 {
                    timeline.anim_param -= 1;
                    pointer = *labels.get(*arguments.first()?)?;
                }
            }
            "anim_ret" => return Some(()),
            _ => {}
        }
    }
    Some(())
}

fn visible_null_battle_animation_object_lifetime(
    bundle: &serde_json::Value,
    frameset_name: &str,
) -> Option<u16> {
    let frames = bundle.get("framesets")?.get(frameset_name)?.as_array()?;
    let mut duration = 0_u16;
    for frame in frames {
        match frame.get("command")?.as_str()? {
            "frame" | "wait" => {
                let frames = frame.get("duration")?.as_u64()?.max(1);
                duration = duration.saturating_add(u16::try_from(frames).ok()?);
            }
            "delete" => return Some(duration.max(1)),
            "restart" | "end" => return None,
            _ => return None,
        }
    }
    None
}

fn parse_visible_battle_animation_int(token: &str) -> Option<i32> {
    let token = token.trim().replace('_', "");
    if let Some(hex) = token.strip_prefix('$') {
        i32::from_str_radix(hex, 16).ok()
    } else if let Some(binary) = token.strip_prefix('%') {
        i32::from_str_radix(binary, 2).ok()
    } else if let Some(hex) = token.strip_prefix("0x") {
        i32::from_str_radix(hex, 16).ok()
    } else if let Some(binary) = token.strip_prefix("0b") {
        i32::from_str_radix(binary, 2).ok()
    } else {
        token.parse::<i32>().ok()
    }
}

fn battle_stat_display_name(stat: &str) -> &str {
    match stat {
        "ATTACK" => "ATTACK",
        "DEFENSE" => "DEFENSE",
        "SPEED" => "SPEED",
        "SPECIAL_ATTACK" => "SPCL.ATK",
        "SPECIAL_DEFENSE" => "SPCL.DEF",
        "ACCURACY" => "ACCURACY",
        "EVASION" => "EVASION",
        "ABILITY" => "ABILITY",
        _ => "INVALID STAT",
    }
}

fn battle_event_changes_visible_scene(
    event: &crate::core::battle::turn::BattleEvent,
) -> bool {
    use crate::core::battle::turn::BattleEvent;

    matches!(
        event,
        BattleEvent::Switched { .. }
            | BattleEvent::Damage { .. }
            | BattleEvent::ResidualStatusDamage { .. }
            | BattleEvent::LeechSeedDamage { .. }
            | BattleEvent::CurseDamage { .. }
            | BattleEvent::NightmareDamage { .. }
            | BattleEvent::TrapDamage { .. }
            | BattleEvent::SpikesDamage { .. }
            | BattleEvent::FutureSightDamage { .. }
            | BattleEvent::SandstormDamage { .. }
            | BattleEvent::ConfusionSelfDamage { .. }
            | BattleEvent::HealApplied { .. }
            | BattleEvent::PresentHeal { .. }
            | BattleEvent::HpDrained { .. }
            | BattleEvent::LeechSeedDrain { .. }
            | BattleEvent::PainSplitApplied { .. }
            | BattleEvent::CounterDamage { .. }
            | BattleEvent::BideReleased { .. }
            | BattleEvent::SubstituteCreated { .. }
            | BattleEvent::SubstituteDamaged { .. }
            | BattleEvent::SubstituteBroken { .. }
            | BattleEvent::TransformApplied { .. }
            | BattleEvent::CurseApplied { .. }
            | BattleEvent::SelfdestructDamage { .. }
            | BattleEvent::Fainted { .. }
            | BattleEvent::StatusApplied { .. }
            | BattleEvent::StatusHealed { .. }
            | BattleEvent::HealBellChimed { .. }
            | BattleEvent::HeldItemStatusHealed { .. }
            | BattleEvent::HeldItemHpHealed { .. }
            | BattleEvent::HeldItemPpRestored { .. }
            | BattleEvent::WokeUp { .. }
            | BattleEvent::BattleItemEffect { .. }
            | BattleEvent::RecoilDamage { .. }
            | BattleEvent::JumpKickCrash { .. }
    )
}

fn retarget_visible_battle_hp_tween(
    runtime_shell: &mut BevyRuntimeShell,
    scene: &RuntimeShellSnapshot,
) {
    let player_target_pixels = scene
        .battle
        .as_ref()
        .and_then(|battle| battle.active_player_party_index)
        .and_then(|index| scene.party.slots.iter().find(|slot| slot.index == index))
        .map(|slot| battle_hud_hp_pixels(slot.pokemon.hp, slot.pokemon.max_hp));
    let enemy_target_pixels = scene
        .battle
        .as_ref()
        .map(|battle| battle_hud_hp_pixels(battle.enemy_pokemon.hp, battle.enemy_pokemon.max_hp));
    let Some(tween) = runtime_shell.battle_hp_tween.as_mut() else {
        return;
    };
    if let Some(pixels) = player_target_pixels {
        tween.player_target_pixels = pixels;
        tween.player_frames_until_step = 0;
    }
    if let Some(pixels) = enemy_target_pixels {
        tween.enemy_target_pixels = pixels;
        tween.enemy_frames_until_step = 0;
    }
}

fn apply_visible_battle_event_to_scene(
    scene: &mut RuntimeShellSnapshot,
    event: &crate::core::battle::turn::BattleEvent,
    pending_baton_pass_sides: &mut BTreeSet<crate::core::battle::turn::BattleSide>,
) {
    use crate::core::battle::turn::{BattleEvent, BattleSide};

    match event {
        BattleEvent::BatonPassed { side, .. } => {
            pending_baton_pass_sides.insert(*side);
            return;
        }
        BattleEvent::TransformApplied { side, species, .. } => {
            let Some(battle) = scene.battle.as_mut() else {
                return;
            };
            match side {
                BattleSide::Player => battle.player_transformed_species = Some(species.clone()),
                BattleSide::Enemy => battle.enemy_transformed_species = Some(species.clone()),
            }
            return;
        }
        BattleEvent::SubstituteCreated {
            side,
            substitute_hp,
            ..
        }
        | BattleEvent::SubstituteDamaged {
            target: side,
            substitute_hp_after: substitute_hp,
            ..
        } => {
            set_visible_battle_substitute_hp(scene, *side, *substitute_hp);
        }
        BattleEvent::SubstituteBroken { target, .. } => {
            set_visible_battle_substitute_hp(scene, *target, 0);
        }
        BattleEvent::BattleItemEffect { side, outcome } => {
            set_visible_battle_side_hp(scene, *side, outcome.hp_after);
            set_visible_battle_side_status(scene, *side, outcome.status_after.clone());
            return;
        }
        BattleEvent::HeldItemHpHealed { side, hp_after, .. } => {
            set_visible_battle_side_hp(scene, *side, *hp_after);
            return;
        }
        BattleEvent::StatusApplied { target, status, .. } => {
            set_visible_battle_side_status(scene, *target, Some(status.clone()));
            return;
        }
        BattleEvent::StatusHealed { target, .. } | BattleEvent::WokeUp { side: target, .. } => {
            set_visible_battle_side_status(scene, *target, None);
            return;
        }
        BattleEvent::HealBellChimed {
            side,
            active_status_before: Some(_),
        } => {
            set_visible_battle_side_status(scene, *side, None);
            return;
        }
        BattleEvent::HeldItemStatusHealed {
            side,
            status_before: Some(_),
            ..
        } => {
            set_visible_battle_side_status(scene, *side, None);
            return;
        }
        BattleEvent::Fainted { side } => {
            set_visible_battle_side_hp(scene, *side, 0);
            return;
        }
        _ => {}
    }

    if let BattleEvent::PainSplitApplied {
        side,
        target,
        user_hp_after,
        target_hp_after,
        ..
    } = event
    {
        set_visible_battle_side_hp(scene, *side, *user_hp_after);
        set_visible_battle_side_hp(scene, *target, *target_hp_after);
        return;
    }

    if let BattleEvent::HpDrained { side, hp_after, .. }
    | BattleEvent::LeechSeedDrain { side, hp_after, .. } = event
    {
        set_visible_battle_side_hp(scene, *side, *hp_after);
        return;
    }

    if let BattleEvent::PresentHeal {
        target, hp_after, ..
    }
    | BattleEvent::CounterDamage {
        target,
        defender_hp_after: hp_after,
        ..
    }
    | BattleEvent::BideReleased {
        target,
        target_hp_after: hp_after,
        ..
    } = event
    {
        set_visible_battle_side_hp(scene, *target, *hp_after);
        return;
    }

    if let BattleEvent::SubstituteCreated { side, hp_after, .. }
    | BattleEvent::CurseApplied { side, hp_after, .. } = event
    {
        set_visible_battle_side_hp(scene, *side, *hp_after);
        return;
    }

    if let BattleEvent::SelfdestructDamage { side, .. } = event {
        set_visible_battle_side_hp(scene, *side, 0);
        return;
    }

    if let BattleEvent::Switched { side, party_index } = event {
        let baton_pass_switch = pending_baton_pass_sides.remove(side);
        let Some(battle) = scene.battle.as_mut() else {
            return;
        };
        match side {
            BattleSide::Player => {
                battle.active_player_party_index = Some(*party_index);
                battle.player_transformed_species = None;
                if !baton_pass_switch {
                    battle.player_substitute_hp = 0;
                }
            }
            BattleSide::Enemy => {
                battle.active_enemy_party_index = Some(*party_index);
                battle.enemy_transformed_species = None;
                if !baton_pass_switch {
                    battle.enemy_substitute_hp = 0;
                }
                if let Some(pokemon) = battle.enemy_party.get(*party_index).cloned() {
                    battle.enemy_pokemon = pokemon;
                }
            }
        }
        return;
    }

    let hp_update = match event {
        BattleEvent::Damage {
            side,
            defender_hp_after,
            ..
        } => Some((
            match side {
                BattleSide::Player => BattleSide::Enemy,
                BattleSide::Enemy => BattleSide::Player,
            },
            *defender_hp_after,
        )),
        BattleEvent::ResidualStatusDamage { side, hp_after, .. }
        | BattleEvent::LeechSeedDamage { side, hp_after, .. }
        | BattleEvent::CurseDamage { side, hp_after, .. }
        | BattleEvent::NightmareDamage { side, hp_after, .. }
        | BattleEvent::TrapDamage { side, hp_after, .. }
        | BattleEvent::SpikesDamage { side, hp_after, .. }
        | BattleEvent::FutureSightDamage { side, hp_after, .. }
        | BattleEvent::SandstormDamage { side, hp_after, .. }
        | BattleEvent::ConfusionSelfDamage { side, hp_after, .. }
        | BattleEvent::HealApplied { side, hp_after, .. }
        | BattleEvent::RecoilDamage { side, hp_after, .. }
        | BattleEvent::JumpKickCrash { side, hp_after, .. } => Some((*side, *hp_after)),
        _ => None,
    };
    let Some((side, hp)) = hp_update else {
        return;
    };
    set_visible_battle_side_hp(scene, side, hp);
}

fn set_visible_battle_side_hp(
    scene: &mut RuntimeShellSnapshot,
    side: crate::core::battle::turn::BattleSide,
    hp: u16,
) {
    use crate::core::battle::turn::BattleSide;

    let Some(battle) = scene.battle.as_mut() else {
        return;
    };
    match side {
        BattleSide::Enemy => battle.enemy_pokemon.hp = hp,
        BattleSide::Player => {
            let Some(active_index) = battle.active_player_party_index else {
                return;
            };
            if let Some(slot) = scene
                .party
                .slots
                .iter_mut()
                .find(|slot| slot.index == active_index)
            {
                slot.pokemon.hp = hp;
            }
        }
    }
}

fn set_visible_battle_side_status(
    scene: &mut RuntimeShellSnapshot,
    side: crate::core::battle::turn::BattleSide,
    status: Option<String>,
) {
    use crate::core::battle::turn::BattleSide;

    let Some(battle) = scene.battle.as_mut() else {
        return;
    };
    match side {
        BattleSide::Enemy => battle.enemy_pokemon.status = status,
        BattleSide::Player => {
            let Some(active_index) = battle.active_player_party_index else {
                return;
            };
            if let Some(slot) = scene
                .party
                .slots
                .iter_mut()
                .find(|slot| slot.index == active_index)
            {
                slot.pokemon.status = status;
            }
        }
    }
}

fn set_visible_battle_substitute_hp(
    scene: &mut RuntimeShellSnapshot,
    side: crate::core::battle::turn::BattleSide,
    hp: u16,
) {
    use crate::core::battle::turn::BattleSide;

    let Some(battle) = scene.battle.as_mut() else {
        return;
    };
    match side {
        BattleSide::Player => battle.player_substitute_hp = hp,
        BattleSide::Enemy => battle.enemy_substitute_hp = hp,
    }
}

fn format_battle_turn_summary(outcome: &crate::core::battle::turn::BattleTurnOutcome) -> String {
    format!(
        "player_hp={}/{} enemy_hp={}/{} turn={}",
        outcome.state.player.hp,
        outcome.state.player.max_hp,
        outcome.state.enemy.hp,
        outcome.state.enemy.max_hp,
        outcome.state.turn
    )
}

fn use_visible_repel(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_id = selected_carried_normal_item_matching(
        runtime_shell,
        |item| item.repel_steps.is_some(),
        "selected item is not a repel",
    )?;
    record_visible_runtime_action(runtime_shell, format!("field:item:{item_id}:repel"))?;
    if snapshot.progression.repel_steps_remaining > 0 {
        runtime_shell.field_notice = Some(visible_asm_text(
            &snapshot,
            "RepelUsedEarlierIsStillInEffectText",
        )?);
        mark_runtime_snapshot_dirty(runtime_shell);
        set_shell_action_status(runtime_shell, "REPEL STILL IN EFFECT");
        return Ok(());
    }
    let item_use = match runtime_shell.shell.use_bag_repel_in_field(&item_id) {
        Ok(item_use) => item_use,
        Err(error) if party_field_move_error_is_play_refusal(&error) => {
            return handle_visible_field_action_refusal(
                runtime_shell,
                &item_id,
                format!("{item_id} CAN'T BE USED HERE"),
                error,
            );
        }
        Err(error) => return Err(error),
    };
    runtime_shell.last_audio_events.push(format!(
        "field repel item={} steps={} consumed={} checksum={:?}",
        item_id, item_use.repel_steps_after, item_use.item_use.consumed, item_use.state_checksum
    ));
    Ok(())
}

fn use_visible_bicycle(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_id = carried_field_rule_item(&snapshot, &runtime_shell.shell, "bicycle")?;
    record_visible_runtime_action(runtime_shell, format!("field:item:{item_id}:bicycle"))?;
    if snapshot.overworld.mode == MovementMode::Bike
        && snapshot
            .progression
            .active_engine_flags
            .contains("ENGINE_ALWAYS_ON_BIKE")
    {
        runtime_shell.field_notice = Some(visible_asm_text(&snapshot, "CantGetOffBikeText")?);
        mark_runtime_snapshot_dirty(runtime_shell);
        set_shell_action_status(runtime_shell, "CAN'T GET OFF HERE");
        return Ok(());
    }
    let item_use = match runtime_shell.shell.use_bag_bicycle_in_field(&item_id) {
        Ok(item_use) => item_use,
        Err(error) if party_field_move_error_is_play_refusal(&error) => {
            return handle_visible_field_action_refusal(
                runtime_shell,
                &item_id,
                format!("{item_id} CAN'T BE USED HERE"),
                error,
            );
        }
        Err(error) => return Err(error),
    };
    runtime_shell.last_audio_events.push(format!(
        "field bicycle item={} mode={:?}->{:?} checksum={:?}",
        item_id, item_use.mode_before, item_use.mode_after, item_use.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!("BICYCLE MODE {:?}", item_use.mode_after),
    );
    close_visible_field_pack_without_log(runtime_shell);
    let display_name = item_display_name(&snapshot, &item_id);
    runtime_shell.field_notice = Some(match item_use.mode_after {
        MovementMode::Bike => format!(
            "{} got on the\n{}.",
            snapshot.trainer.player_name, display_name
        ),
        MovementMode::Normal => format!(
            "{} got off\nthe {}.",
            snapshot.trainer.player_name, display_name
        ),
        MovementMode::Skate | MovementMode::Surf | MovementMode::SurfPika => {
            anyhow::bail!("bicycle use ended in invalid mode {:?}", item_use.mode_after)
        }
    });
    mark_runtime_snapshot_dirty(runtime_shell);
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn use_visible_town_map(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_id = carried_field_rule_item(&snapshot, &runtime_shell.shell, "town_map")?;
    record_visible_runtime_action(runtime_shell, format!("field:item:{item_id}:town_map"))?;
    let item_use = runtime_shell.shell.use_bag_town_map_in_field(&item_id)?;
    open_visible_pokegear_menu(runtime_shell)?;
    runtime_shell.last_audio_events.push(format!(
        "field town_map item={} landmark={:?} checksum={:?}",
        item_id, item_use.landmark, item_use.state_checksum
    ));
    Ok(())
}

fn use_visible_escape_rope(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_id = carried_field_rule_item(&snapshot, &runtime_shell.shell, "escape_rope")?;
    record_visible_runtime_action(runtime_shell, format!("field:item:{item_id}:escape_rope"))?;
    let item_use = match runtime_shell.shell.use_bag_escape_rope_in_field(&item_id) {
        Ok(item_use) => item_use,
        Err(error) if party_field_move_error_is_play_refusal(&error) => {
            return handle_visible_field_action_refusal(
                runtime_shell,
                &item_id,
                format!("{item_id} CAN'T BE USED HERE"),
                error,
            );
        }
        Err(error) => return Err(error),
    };
    runtime_shell.last_audio_events.push(format!(
        "field escape_rope item={} destination={} warp={} checksum={:?}",
        item_id, item_use.destination_map, item_use.destination_warp_index, item_use.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!(
            "ESCAPE ROPE TO {} WARP {}",
            item_use.destination_map, item_use.destination_warp_index
        ),
    );
    runtime_shell.field_notice = Some(visible_asm_text(&snapshot, "UseEscapeRopeText")?);
    runtime_shell.pending_field_travel_arrival = true;
    runtime_shell.pending_field_travel_delay_frames = None;
    Ok(())
}

fn use_visible_fishing_rod(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let scene = runtime_shell.shell.snapshot()?;
    let rods = runtime_shell.shell.fishing_rod_ids();
    let item_id = selected_carried_normal_item_matching(
        runtime_shell,
        |item| rods.contains(&item.item_id),
        "selected item is not a fishing rod",
    )?;
    record_visible_runtime_action(runtime_shell, format!("field:item:{item_id}:fishing_rod"))?;
    let item_use = match runtime_shell.shell.use_bag_fishing_rod_in_field(&item_id) {
        Ok(item_use) => item_use,
        Err(error) if fishing_error_is_cant_fish_here(&error) => {
            close_visible_field_pack_without_log(runtime_shell);
            retain_visible_field_notice_scene(runtime_shell, &scene);
            runtime_shell.field_notice = Some(visible_fishing_cant_cast_text());
            mark_runtime_snapshot_dirty(runtime_shell);
            return Ok(());
        }
        Err(error) if party_field_move_error_is_play_refusal(&error) => {
            return handle_visible_field_action_refusal(
                runtime_shell,
                &item_id,
                "NOT EVEN A NIBBLE",
                error,
            );
        }
        Err(error) => return Err(error),
    };
    runtime_shell.last_audio_events.push(format!(
        "field fishing item={} rod={} group={:?} bite={:?} battle={:?} checksum={:?}",
        item_id,
        item_use.rod,
        item_use.cast.session.group,
        item_use.cast.bite,
        item_use.cast.wild_battle,
        item_use.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!(
            "FISHING {:?} {:?}",
            item_use.cast.bite, item_use.cast.wild_battle
        ),
    );
    close_visible_field_pack_without_log(runtime_shell);
    present_visible_fishing_cast(
        runtime_shell,
        &scene,
        item_use.cast.bite,
        item_use.cast.wild_battle.is_some(),
    )?;
    Ok(())
}

fn fishing_error_is_cant_fish_here(error: &anyhow::Error) -> bool {
    matches!(
        error.downcast_ref::<FishingError>(),
        Some(
            FishingError::CannotFishWhileSurfing
                | FishingError::FacingTileOutOfBounds
                | FishingError::FacingTileIsNotWater
        )
    )
}

fn visible_fishing_cant_cast_text() -> String {
    "There's no water\nto fish here.".to_string()
}

fn present_visible_fishing_cast(
    runtime_shell: &mut BevyRuntimeShell,
    scene: &RuntimeShellSnapshot,
    bite: Option<bool>,
    starts_battle: bool,
) -> Result<()> {
    retain_visible_field_notice_scene(runtime_shell, scene);
    runtime_shell.field_notice_queue.clear();
    runtime_shell.field_notice = None;
    runtime_shell.pending_field_battle_entry = false;
    runtime_shell.visible_fishing_animation = Some(VisibleFishingAnimation {
        phase: VisibleFishingPhase::Cast,
        frame: 0,
        bite: bite == Some(true),
        starts_battle,
    });
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn use_visible_itemfinder(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_id = carried_field_rule_item(&snapshot, &runtime_shell.shell, "itemfinder")?;
    record_visible_runtime_action(runtime_shell, format!("field:item:{item_id}:itemfinder"))?;
    let item_use = match runtime_shell.shell.use_bag_itemfinder_in_field(&item_id) {
        Ok(item_use) => item_use,
        Err(error) if party_field_move_error_is_play_refusal(&error) => {
            return handle_visible_field_action_refusal(
                runtime_shell,
                &item_id,
                format!("{item_id} CAN'T BE USED HERE"),
                error,
            );
        }
        Err(error) => return Err(error),
    };
    runtime_shell.last_audio_events.push(format!(
        "field itemfinder item={} found={:?} cues={} consumed={} checksum={:?}",
        item_id,
        item_use.found,
        item_use.itemfinder_sound_cues,
        item_use.item_use.consumed,
        item_use.state_checksum
    ));
    set_shell_action_status(runtime_shell, format!("ITEMFINDER {:?}", item_use.found));
    close_visible_field_pack_without_log(runtime_shell);
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn use_visible_squirtbottle(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_id = carried_field_rule_item(&snapshot, &runtime_shell.shell, "squirtbottle")?;
    record_visible_runtime_action(runtime_shell, format!("field:item:{item_id}:squirtbottle"))?;
    let item_use = match runtime_shell.shell.use_bag_squirtbottle_in_field(&item_id) {
        Ok(item_use) => item_use,
        Err(error) if party_field_move_error_is_play_refusal(&error) => {
            return handle_visible_field_action_refusal(
                runtime_shell,
                &item_id,
                format!("{item_id} CAN'T BE USED HERE"),
                error,
            );
        }
        Err(error) => return Err(error),
    };
    runtime_shell.last_audio_events.push(format!(
        "field squirtbottle item={} target={:?} movement={} script={:?} checksum={:?}",
        item_id,
        item_use.target_object_identifier,
        item_use.target_movement,
        item_use.target_script,
        item_use.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!(
            "SQUIRTBOTTLE TARGET {:?}",
            item_use.target_object_identifier
        ),
    );
    close_visible_field_pack_without_log(runtime_shell);
    consume_visible_dispatched_field_script(runtime_shell)?;
    Ok(())
}

fn consume_visible_dispatched_field_script(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.script_events.next_script.is_some() {
        take_visible_next_script(runtime_shell)?;
    }
    Ok(())
}

fn use_visible_coin_case(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_id = carried_field_rule_item(&snapshot, &runtime_shell.shell, "coin_case")?;
    record_visible_runtime_action(runtime_shell, format!("field:item:{item_id}:coin_case"))?;
    let item_use = runtime_shell.shell.use_bag_coin_case_in_field(&item_id)?;
    runtime_shell.last_audio_events.push(format!(
        "field coin_case item={} {}={} checksum={:?}",
        item_id, item_use.balance_label, item_use.balance, item_use.state_checksum
    ));
    open_visible_field_balance_boundary(
        runtime_shell,
        "FieldCoinCase",
        &item_id,
        &item_use.balance_label,
        item_use.balance,
    );
    Ok(())
}

fn use_visible_blue_card(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_id = carried_field_rule_item(&snapshot, &runtime_shell.shell, "blue_card")?;
    record_visible_runtime_action(runtime_shell, format!("field:item:{item_id}:blue_card"))?;
    let item_use = runtime_shell.shell.use_bag_blue_card_in_field(&item_id)?;
    runtime_shell.last_audio_events.push(format!(
        "field blue_card item={} {}={} checksum={:?}",
        item_id, item_use.balance_label, item_use.balance, item_use.state_checksum
    ));
    open_visible_field_balance_boundary(
        runtime_shell,
        "FieldBlueCard",
        &item_id,
        &item_use.balance_label,
        item_use.balance,
    );
    Ok(())
}

fn open_visible_field_balance_boundary(
    runtime_shell: &mut BevyRuntimeShell,
    label: &str,
    item_id: &str,
    balance_label: &str,
    balance: impl Display,
) {
    close_visible_field_pack_without_log(runtime_shell);
    let balance = balance.to_string();
    runtime_shell.field_notice = Some(match label {
        "FieldCoinCase" => format!("Coins:\n{balance}"),
        "FieldBlueCard" => format!("You now have\n{balance} points."),
        _ => unreachable!("unknown field balance surface {label}"),
    });
    mark_runtime_snapshot_dirty(runtime_shell);
    runtime_shell.last_audio_events.push(format!(
        "opened field balance text {label} item={item_id} {balance_label}={balance}"
    ));
    trim_event_log(&mut runtime_shell.last_audio_events);
}

fn use_visible_surf(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "surf",
        runtime_shell.party_cursor,
    )?;
    record_visible_runtime_action(runtime_shell, format!("field_move:surf:{party_index}"))?;
    let field_move = runtime_shell.shell.use_surf_field_move(party_index)?;
    runtime_shell.last_audio_events.push(format!(
        "field surf party_index={} outcome={:?} checksum={:?}",
        party_index, field_move.outcome, field_move.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!("SURF PARTY #{} {:?}", party_index, field_move.outcome),
    );
    retain_visible_field_notice_scene(runtime_shell, &snapshot);
    runtime_shell.field_notice = Some(visible_field_move_use_text(&snapshot, party_index, "SURF")?);
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn execute_visible_contextual_field_move(runtime_shell: &mut BevyRuntimeShell) -> Result<bool> {
    if !visible_field_shortcut_allowed(runtime_shell) {
        return Ok(false);
    }
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(permission) = runtime_shell.shell.facing_tile_collision_permission()? else {
        return Ok(false);
    };
    if runtime_shell_has_object_movement_target(
        runtime_shell,
        &snapshot,
        "SPRITEMOVEDATA_STRENGTH_BOULDER",
    )? {
        if snapshot
            .progression
            .active_engine_flags
            .contains("ENGINE_STRENGTH_ACTIVE")
        {
            runtime_shell.field_notice = Some(visible_asm_text(&snapshot, "BouldersMoveText")?);
            mark_runtime_snapshot_dirty(runtime_shell);
        } else if snapshot_has_field_move_actor_and_badge(
            &snapshot,
            &runtime_shell.shell,
            "strength",
        )? {
            open_visible_contextual_field_move_prompt(
                runtime_shell,
                PartyFieldMove::Strength,
                "AskStrengthText",
            )?;
        } else {
            runtime_shell.field_notice = Some(visible_asm_text(&snapshot, "BouldersMayMoveText")?);
            mark_runtime_snapshot_dirty(runtime_shell);
        }
        return Ok(true);
    }
    if matches!(snapshot.overworld.mode, MovementMode::Surf | MovementMode::SurfPika)
        && snapshot.overworld.facing == crate::core::world::map::Direction::Up
        && field_move_rule_contains_target_collision(&runtime_shell.shell, "waterfall", permission)?
        && snapshot_has_field_move_actor_and_badge(&snapshot, &runtime_shell.shell, "waterfall")?
    {
        open_visible_contextual_field_move_prompt(
            runtime_shell,
            PartyFieldMove::Waterfall,
            "AskWaterfallText",
        )?;
        return Ok(true);
    }
    if !matches!(snapshot.overworld.mode, MovementMode::Surf | MovementMode::SurfPika)
        && field_move_rule_allows_surf_target(&runtime_shell.shell, permission)?
        && snapshot_has_field_move_actor_and_badge(&snapshot, &runtime_shell.shell, "surf")?
    {
        open_visible_contextual_field_move_prompt(
            runtime_shell,
            PartyFieldMove::Surf,
            "AskSurfText",
        )?;
        return Ok(true);
    }
    if field_move_rule_contains_target_collision(&runtime_shell.shell, "cut", permission)?
        && snapshot_has_field_move_actor_and_badge(&snapshot, &runtime_shell.shell, "cut")?
    {
        open_visible_contextual_field_move_prompt(
            runtime_shell,
            PartyFieldMove::Cut,
            "AskCutText",
        )?;
        return Ok(true);
    }
    if field_move_rule_contains_target_collision(&runtime_shell.shell, "whirlpool", permission)?
        && snapshot_has_field_move_actor_and_badge(&snapshot, &runtime_shell.shell, "whirlpool")?
    {
        open_visible_contextual_field_move_prompt(
            runtime_shell,
            PartyFieldMove::Whirlpool,
            "AskWhirlpoolText",
        )?;
        return Ok(true);
    }
    if runtime_shell_has_smashable_rock_target(runtime_shell, &snapshot)? {
        if snapshot_has_field_move_actor_and_badge(
            &snapshot,
            &runtime_shell.shell,
            "rock_smash",
        )? {
            open_visible_contextual_field_move_prompt(
                runtime_shell,
                PartyFieldMove::RockSmash,
                "AskRockSmashText",
            )?;
        } else {
            runtime_shell.field_notice = Some(visible_asm_text(&snapshot, "MaySmashText")?);
            mark_runtime_snapshot_dirty(runtime_shell);
        }
        return Ok(true);
    }
    if snapshot_has_headbutt_target(&snapshot)?
        && snapshot_has_field_move_actor_and_badge(&snapshot, &runtime_shell.shell, "headbutt")?
    {
        open_visible_contextual_field_move_prompt(
            runtime_shell,
            PartyFieldMove::Headbutt,
            "AskHeadbuttText",
        )?;
        return Ok(true);
    }
    Ok(false)
}

fn open_visible_contextual_field_move_prompt(
    runtime_shell: &mut BevyRuntimeShell,
    field_move: PartyFieldMove,
    text_label: &str,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    runtime_shell.party_cursor = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        party_field_move_rule_id(field_move),
        usize::MAX,
    )?;
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "field_move:contextual:{}:prompt",
            party_field_move_rule_id(field_move)
        ),
    )?;
    runtime_shell.pending_contextual_field_move = Some(field_move);
    runtime_shell.yes_no_cursor = Some(MenuCursor {
        surface_id: "field:move-confirm".to_string(),
        option_index: 0,
    });
    runtime_shell.field_notice = Some(visible_asm_text(&snapshot, text_label)?);
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn resolve_visible_contextual_field_move_prompt(
    runtime_shell: &mut BevyRuntimeShell,
    accepted: bool,
) -> Result<()> {
    let Some(field_move) = runtime_shell.pending_contextual_field_move.take() else {
        anyhow::bail!("contextual field-move confirmation is not active");
    };
    runtime_shell.yes_no_cursor = None;
    runtime_shell.field_notice = None;
    runtime_shell.field_notice_queue.clear();
    runtime_shell.field_notice_scene = None;
    if !accepted {
        record_visible_runtime_action(
            runtime_shell,
            format!(
                "field_move:contextual:{}:decline",
                party_field_move_rule_id(field_move)
            ),
        )?;
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    execute_visible_party_field_move(runtime_shell, field_move)
}

fn field_move_rule_contains_target_collision(
    shell: &RuntimeGameShell,
    rule_id: &str,
    permission: u8,
) -> Result<bool> {
    let Some(key) = shell
        .field_move_rule_keys()
        .into_iter()
        .find(|key| key.rule_id == rule_id)
    else {
        return Ok(false);
    };
    if !matches!(key.move_id.as_deref(), Some(move_id) if !move_id.is_empty()) {
        anyhow::bail!("compiled field move rule {rule_id} has no move_id");
    }
    require_visible_field_move_badge_shape(rule_id, key.badge_region.as_deref(), key.badge_index)?;
    Ok(key.target_collisions.contains(&permission))
}

fn require_visible_field_move_badge_shape(
    rule_id: &str,
    region: Option<&str>,
    index: Option<usize>,
) -> Result<()> {
    match (region, index) {
        (Some("johto" | "kanto"), Some(index)) if index < 8 => Ok(()),
        (region, index) => {
            anyhow::bail!(
                "compiled field move rule {rule_id} has invalid badge requirement {region:?} {index:?}"
            );
        }
    }
}

fn snapshot_has_field_move_actor_and_badge(
    snapshot: &RuntimeShellSnapshot,
    shell: &RuntimeGameShell,
    rule_id: &str,
) -> Result<bool> {
    let Some(rule) = shell
        .field_move_rule_keys()
        .into_iter()
        .find(|key| key.rule_id == rule_id)
    else {
        return Ok(false);
    };
    let Some(move_id) = rule.move_id.as_deref() else {
        anyhow::bail!("compiled field move rule {rule_id} has no move_id");
    };
    if !snapshot.party.slots.iter().any(|slot| {
        slot.pokemon
            .moves
            .iter()
            .any(|learned| learned.name == move_id)
    }) {
        return Ok(false);
    }
    match (rule.badge_region.as_deref(), rule.badge_index) {
        (Some("johto"), Some(index)) => {
            visible_badge_at(&snapshot.progression.badges.johto, rule_id, index)
        }
        (Some("kanto"), Some(index)) => {
            visible_badge_at(&snapshot.progression.badges.kanto, rule_id, index)
        }
        (None, None) => Ok(true),
        (region, index) => {
            anyhow::bail!(
                "compiled field move rule {rule_id} has invalid badge requirement {region:?} {index:?}"
            )
        }
    }
}

fn visible_badge_at(badges: &[bool; 8], rule_id: &str, index: usize) -> Result<bool> {
    badges.get(index).copied().with_context(|| {
        format!("compiled field move rule {rule_id} badge index {index} is out of range")
    })
}

fn field_move_rule_allows_surf_target(shell: &RuntimeGameShell, permission: u8) -> Result<bool> {
    let Some(key) = shell
        .field_move_rule_keys()
        .into_iter()
        .find(|key| key.rule_id == "surf")
    else {
        return Ok(false);
    };
    if !matches!(key.move_id.as_deref(), Some(move_id) if !move_id.is_empty()) {
        anyhow::bail!("compiled field move rule surf has no move_id");
    }
    require_visible_field_move_badge_shape("surf", key.badge_region.as_deref(), key.badge_index)?;
    Ok(
        crate::core::world::collision::describe_collision(permission).terrain
            == crate::core::world::collision::Terrain::Water
            && !key.blocked_collisions.contains(&permission),
    )
}

fn runtime_shell_has_smashable_rock_target(
    runtime_shell: &BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
) -> Result<bool> {
    runtime_shell_has_object_movement_target(
        runtime_shell,
        snapshot,
        "SPRITEMOVEDATA_SMASHABLE_ROCK",
    )
}

fn runtime_shell_has_object_movement_target(
    runtime_shell: &BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
    movement: &str,
) -> Result<bool> {
    let Some(interaction) = runtime_shell
        .shell
        .current_overworld_interaction_checked()?
    else {
        return Ok(false);
    };
    if interaction.map_name != snapshot.overworld.map_name {
        return Ok(false);
    }
    let crate::core::world::session::OverworldInteractionTarget::Object {
        object_identifier, ..
    } = &interaction.target
    else {
        return Ok(false);
    };
    for object in &snapshot.visible_objects {
        if object_identifier
            .as_ref()
            .is_some_and(|id| object.object_identifier.as_ref() == Some(id))
            || snapshot_object_tile_matches_checked(snapshot, object, interaction.target_tile)?
        {
            return Ok(object.spritemovedata == movement);
        }
    }
    Ok(false)
}

fn snapshot_has_headbutt_target(snapshot: &RuntimeShellSnapshot) -> Result<bool> {
    let Some(block) = facing_metatile_block(snapshot)? else {
        return Ok(false);
    };
    let map = snapshot
        .maps
        .iter()
        .find(|map| map.map_name == snapshot.overworld.map_name)
        .with_context(|| {
            format!(
                "active overworld map {} is missing from verified map catalog",
                snapshot.overworld.map_name
            )
        })?;
    let tileset = snapshot
        .tilesets
        .iter()
        .find(|tileset| tileset.tileset_id == map.attributes.tileset_name)
        .with_context(|| {
            format!(
                "active map {} references missing verified tileset {}",
                map.map_name, map.attributes.tileset_name
            )
        })?;
    Ok(
        tileset_collision_tokens(tileset, block).is_some_and(|tokens| {
            tokens
                .iter()
                .any(|token| matches!(token.as_str(), "HEADBUTT_TREE" | "HEADBUTT_TREE_1D"))
        }),
    )
}

fn facing_metatile_block(snapshot: &RuntimeShellSnapshot) -> Result<Option<u16>> {
    let map = snapshot
        .maps
        .iter()
        .find(|map| map.map_name == snapshot.overworld.map_name)
        .with_context(|| {
            format!(
                "active overworld map {} is missing from verified map catalog",
                snapshot.overworld.map_name
            )
        })?;
    let TilePosition {
        x: tile_x,
        y: tile_y,
    } = facing_runtime_tile(snapshot)?;
    if tile_x < 0 || tile_y < 0 {
        return Ok(None);
    }
    let Some((metatile_x, metatile_y)) = facing_metatile_coordinates(tile_x, tile_y)? else {
        return Ok(None);
    };
    if metatile_x >= map.attributes.width || metatile_y >= map.attributes.height {
        return Ok(None);
    }
    let index =
        usize::from(metatile_y) * usize::from(map.attributes.width) + usize::from(metatile_x);
    let block = map.blocks.get(index).copied().with_context(|| {
        format!(
            "active map {} block index {} is outside verified block count {}",
            map.map_name,
            index,
            map.blocks.len()
        )
    })?;
    Ok(Some(block))
}

fn facing_metatile_coordinates(tile_x: i16, tile_y: i16) -> Result<Option<(u16, u16)>> {
    if tile_x < 0 || tile_y < 0 {
        return Ok(None);
    }
    if tile_x % METATILE_WIDTH != 0 || tile_y % METATILE_WIDTH != 0 {
        return Ok(None);
    }
    runtime_tile_to_metatile_u16(tile_x, tile_y, "facing metatile block").map(Some)
}

fn use_visible_cut(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let target_tile = facing_runtime_tile(&snapshot)?;
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "cut",
        runtime_shell.party_cursor,
    )?;
    record_visible_runtime_action(runtime_shell, format!("field_move:cut:{party_index}:front"))?;
    let field_move = runtime_shell
        .shell
        .use_cut_field_move_in_front(party_index)?;
    let metatile_x = field_move.outcome.metatile_x;
    let metatile_y = field_move.outcome.metatile_y;
    runtime_shell.visible_cut_animation = Some(VisibleCutAnimation {
        target_tile,
        facing: snapshot.overworld.facing,
        variant: field_move.outcome.variant.clone(),
        frame: 0,
    });
    runtime_shell.pending_field_notice_sound =
        Some("SFX_PLACE_PUZZLE_PIECE_DOWN".to_string());
    runtime_shell.pending_field_notice_effect_frames = Some(32);
    runtime_shell.last_audio_events.push(format!(
        "field cut party_index={} target=({}, {}) outcome={:?} checksum={:?}",
        party_index, metatile_x, metatile_y, field_move.outcome, field_move.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!(
            "CUT ({}, {}) {:?}",
            metatile_x, metatile_y, field_move.outcome
        ),
    );
    trim_event_log(&mut runtime_shell.last_audio_events);
    retain_visible_field_notice_scene(runtime_shell, &snapshot);
    runtime_shell.field_notice = Some(visible_field_move_use_text(&snapshot, party_index, "CUT")?);
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn use_visible_whirlpool(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let target_tile = facing_runtime_tile(&snapshot)?;
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "whirlpool",
        runtime_shell.party_cursor,
    )?;
    record_visible_runtime_action(
        runtime_shell,
        format!("field_move:whirlpool:{party_index}:front"),
    )?;
    let field_move = runtime_shell
        .shell
        .use_whirlpool_field_move_in_front(party_index)?;
    let metatile_x = field_move.outcome.metatile_x;
    let metatile_y = field_move.outcome.metatile_y;
    runtime_shell.last_audio_events.push(format!(
        "field whirlpool party_index={} target=({}, {}) outcome={:?} checksum={:?}",
        party_index, metatile_x, metatile_y, field_move.outcome, field_move.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!(
            "WHIRLPOOL ({}, {}) {:?}",
            metatile_x, metatile_y, field_move.outcome
        ),
    );
    trim_event_log(&mut runtime_shell.last_audio_events);
    retain_visible_field_notice_scene(runtime_shell, &snapshot);
    runtime_shell.field_notice =
        Some(visible_field_move_use_text(&snapshot, party_index, "WHIRLPOOL")?);
    runtime_shell.pending_field_notice_sound = Some("SFX_SURF".to_string());
    runtime_shell.pending_field_notice_effect_frames = Some(32);
    runtime_shell.visible_whirlpool_animation = Some(VisibleWhirlpoolAnimation {
        target_tile,
        frame: 0,
    });
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn use_visible_strength(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "strength",
        runtime_shell.party_cursor,
    )?;
    record_visible_runtime_action(runtime_shell, format!("field_move:strength:{party_index}"))?;
    let field_move = runtime_shell.shell.use_strength_field_move(party_index)?;
    runtime_shell.last_audio_events.push(format!(
        "field strength party_index={} outcome={:?} checksum={:?}",
        party_index, field_move.outcome, field_move.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!("STRENGTH PARTY #{} {:?}", party_index, field_move.outcome),
    );
    retain_visible_field_notice_scene(runtime_shell, &snapshot);
    runtime_shell.field_notice =
        Some(visible_field_move_use_text(&snapshot, party_index, "STRENGTH")?);
    mark_runtime_snapshot_dirty(runtime_shell);
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn use_visible_flash(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "flash",
        runtime_shell.party_cursor,
    )?;
    record_visible_runtime_action(runtime_shell, format!("field_move:flash:{party_index}"))?;
    let field_move = runtime_shell.shell.use_flash_field_move(party_index)?;
    runtime_shell.last_audio_events.push(format!(
        "field flash party_index={} outcome={:?} checksum={:?}",
        party_index, field_move.outcome, field_move.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!("FLASH PARTY #{} {:?}", party_index, field_move.outcome),
    );
    retain_visible_field_notice_scene(runtime_shell, &snapshot);
    runtime_shell.field_notice = Some(visible_field_move_use_text(&snapshot, party_index, "FLASH")?);
    runtime_shell.visible_flash_animation = Some(VisibleFlashAnimation { frame: 0 });
    runtime_shell.pending_field_notice_sound = Some("SFX_FLASH".to_string());
    runtime_shell.pending_field_notice_effect_frames = Some(16);
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn use_visible_waterfall(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "waterfall",
        runtime_shell.party_cursor,
    )?;
    record_visible_runtime_action(runtime_shell, format!("field_move:waterfall:{party_index}"))?;
    let field_move = runtime_shell.shell.use_waterfall_field_move(party_index)?;
    runtime_shell.last_audio_events.push(format!(
        "field waterfall party_index={} outcome={:?} checksum={:?}",
        party_index, field_move.outcome, field_move.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!("WATERFALL PARTY #{} {:?}", party_index, field_move.outcome),
    );
    retain_visible_field_notice_scene(runtime_shell, &snapshot);
    runtime_shell.field_notice =
        Some(visible_field_move_use_text(&snapshot, party_index, "WATERFALL")?);
    runtime_shell.pending_field_notice_sound = Some("SFX_BUBBLEBEAM".to_string());
    runtime_shell.visible_waterfall_animation = Some(VisibleWaterfallAnimation {
        from_tile: field_move.outcome.from_tile,
        to_tile: field_move.outcome.to_tile,
        steps: field_move.outcome.steps,
        frame: 0,
    });
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn use_visible_fly(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "fly",
        runtime_shell.party_cursor,
    )?;
    let destinations = active_fly_destinations(&snapshot, &runtime_shell.shell);
    if destinations.is_empty() {
        runtime_shell.fly_cursor = None;
        record_visible_runtime_action(runtime_shell, "field_move:fly:no_destinations")?;
        runtime_shell
            .last_audio_events
            .push("no active FLYPOINT engine flags".to_string());
        set_shell_action_status(runtime_shell, "NO FLY DESTINATIONS");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let selected_index = visible_cursor_index(
        &mut runtime_shell.fly_cursor,
        "fly:destinations",
        destinations.len(),
    );
    let destination = destinations[selected_index].clone();
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "field_move:fly:{}:{}:{}:{}",
            party_index,
            selected_index,
            destination.flypoint_flag,
            destination.destination_spawn_identifier
        ),
    )?;
    let field_move = runtime_shell.shell.use_fly_field_move(
        party_index,
        destination.destination_spawn_identifier,
        &destination.flypoint_flag,
    )?;
    runtime_shell.last_audio_events.push(format!(
        "field fly destination {}/{} flag={} party_index={} spawn={} map={} tile=({}, {}) checksum={:?}",
        selected_index + 1,
        destinations.len(),
        field_move.flypoint_flag,
        party_index,
        field_move.destination_spawn_identifier,
        field_move.destination_map,
        field_move.destination_tile.x,
        field_move.destination_tile.y,
        field_move.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!(
            "FLY TO {} ({}, {})",
            field_move.destination_map,
            field_move.destination_tile.x,
            field_move.destination_tile.y
        ),
    );
    retain_visible_field_notice_scene(runtime_shell, &snapshot);
    runtime_shell.pending_field_travel_arrival = false;
    runtime_shell.pending_field_travel_delay_frames = None;
    runtime_shell.visible_fly_animation = Some(VisibleFlyAnimation {
        phase: VisibleFlyAnimationPhase::From,
        frame: 0,
    });
    let BevyRuntimeShell {
        shell,
        pending_audio,
        last_audio_events,
        ..
    } = runtime_shell;
    queue_visible_sound_effect(
        shell.runtime().audio(),
        pending_audio,
        last_audio_events,
        "SFX_FLY",
    )?;
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn active_fly_destinations(
    snapshot: &RuntimeShellSnapshot,
    shell: &RuntimeGameShell,
) -> Vec<RuntimeFlyDestinationKey> {
    let use_kanto_map = visible_pokegear_region(snapshot) == "KANTO"
        && snapshot
            .progression
            .active_engine_flags
            .contains("ENGINE_FLYPOINT_INDIGO_PLATEAU");
    shell
        .fly_destination_keys()
        .into_iter()
        .filter(|destination| {
            let destination_is_kanto = snapshot
                .presentation
                .pokegear_landmarks
                .landmarks
                .iter()
                .find(|landmark| landmark.constant == destination.label)
                .is_some_and(|landmark| landmark.region == "KANTO");
            if destination_is_kanto != use_kanto_map {
                return false;
            }
            let is_default = if use_kanto_map {
                destination.label == "LANDMARK_INDIGO_PLATEAU"
            } else {
                destination.label == "LANDMARK_SILVER_CAVE"
            };
            is_default
                || snapshot
                    .progression
                    .active_engine_flags
                    .contains(&destination.flypoint_flag)
        })
        .collect()
}

fn fly_destination_label(destination: &RuntimeFlyDestinationKey) -> String {
    destination
        .label
        .strip_prefix("LANDMARK_")
        .unwrap_or(&destination.label)
        .replace('_', " ")
}

fn use_visible_dig(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "dig",
        runtime_shell.party_cursor,
    )?;
    record_visible_runtime_action(runtime_shell, format!("field_move:dig:{party_index}"))?;
    let field_move = runtime_shell.shell.use_dig_field_move(party_index)?;
    runtime_shell.last_audio_events.push(format!(
        "field dig party_index={} destination={} warp={} tile=({}, {}) checksum={:?}",
        party_index,
        field_move.destination_map,
        field_move.destination_warp_index,
        field_move.destination_tile.x,
        field_move.destination_tile.y,
        field_move.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!(
            "DIG TO {} WARP {}",
            field_move.destination_map, field_move.destination_warp_index
        ),
    );
    runtime_shell.field_notice = Some(visible_field_move_use_text(&snapshot, party_index, "DIG")?);
    runtime_shell.pending_field_travel_arrival = true;
    runtime_shell.pending_field_travel_delay_frames = None;
    Ok(())
}

fn use_visible_teleport(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "teleport",
        runtime_shell.party_cursor,
    )?;
    record_visible_runtime_action(runtime_shell, format!("field_move:teleport:{party_index}"))?;
    let field_move = runtime_shell.shell.use_teleport_field_move(party_index)?;
    runtime_shell.last_audio_events.push(format!(
        "field teleport party_index={} destination={} spawn={} tile=({}, {}) checksum={:?}",
        party_index,
        field_move.destination_map,
        field_move.destination_spawn_identifier,
        field_move.destination_tile.x,
        field_move.destination_tile.y,
        field_move.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!(
            "TELEPORT TO {} ({}, {})",
            field_move.destination_map,
            field_move.destination_tile.x,
            field_move.destination_tile.y
        ),
    );
    runtime_shell.field_notice = Some(visible_asm_text(&snapshot, "TeleportReturnText")?);
    runtime_shell.pending_field_travel_arrival = true;
    runtime_shell.pending_field_travel_delay_frames = Some(60);
    Ok(())
}

fn use_visible_headbutt(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let target_tile = facing_runtime_tile(&snapshot)?;
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "headbutt",
        runtime_shell.party_cursor,
    )?;
    let player_id = snapshot.trainer.player_id;
    record_visible_runtime_action(
        runtime_shell,
        format!("field_move:headbutt:{party_index}:player:{player_id}"),
    )?;
    let field_move = runtime_shell
        .shell
        .use_headbutt_field_move(party_index, player_id)?;
    runtime_shell.visible_headbutt_animation = Some(VisibleHeadbuttAnimation {
        target_tile,
        facing: snapshot.overworld.facing,
        frame: 0,
    });
    runtime_shell.pending_field_notice_sound = Some("SFX_SANDSTORM".to_string());
    runtime_shell.pending_field_notice_effect_frames = Some(32);
    runtime_shell.last_audio_events.push(format!(
        "field headbutt party_index={} encounter={:?} battle={:?} checksum={:?}",
        party_index, field_move.field_encounter, field_move.wild_battle, field_move.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!(
            "HEADBUTT {:?} {:?}",
            field_move.field_encounter, field_move.wild_battle
        ),
    );
    settle_visible_field_move_after_possible_battle(
        runtime_shell,
        &snapshot,
        visible_field_move_use_text(&snapshot, party_index, "HEADBUTT")?,
    )?;
    Ok(())
}

fn use_visible_rock_smash(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let (dx, dy) = snapshot.overworld.facing.delta();
    let rock_smash_target = TilePosition::new(
        snapshot.overworld.tile.x.checked_add(dx).context("Rock Smash target X overflow")?,
        snapshot.overworld.tile.y.checked_add(dy).context("Rock Smash target Y overflow")?,
    );
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "rock_smash",
        runtime_shell.party_cursor,
    )?;
    record_visible_runtime_action(
        runtime_shell,
        format!("field_move:rock_smash:{party_index}"),
    )?;
    let field_move = runtime_shell.shell.use_rock_smash_field_move(party_index)?;
    runtime_shell.visible_rock_smash_target = Some(rock_smash_target);
    runtime_shell.last_audio_events.push(format!(
        "field rock_smash party_index={} encounter={:?} battle={:?} checksum={:?}",
        party_index, field_move.field_encounter, field_move.wild_battle, field_move.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!(
            "ROCK SMASH {:?} {:?}",
            field_move.field_encounter, field_move.wild_battle
        ),
    );
    runtime_shell.pending_field_notice_sound = Some("SFX_STRENGTH".to_string());
    runtime_shell.pending_field_notice_effect_frames = Some(30);
    settle_visible_field_move_after_possible_battle(
        runtime_shell,
        &snapshot,
        visible_field_move_use_text(&snapshot, party_index, "ROCK SMASH")?,
    )?;
    Ok(())
}

fn use_visible_sweet_scent(
    runtime_shell: &mut BevyRuntimeShell,
    surface: EncounterSurface,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let party_index = party_index_for_field_move_rule(
        &snapshot,
        &runtime_shell.shell,
        "sweet_scent",
        runtime_shell.party_cursor,
    )?;
    record_visible_runtime_action(
        runtime_shell,
        format!("field_move:sweet_scent:{party_index}:{surface:?}"),
    )?;
    let field_move = runtime_shell
        .shell
        .use_sweet_scent_field_move(party_index, surface)?;
    runtime_shell.last_audio_events.push(format!(
        "field sweet_scent party_index={} surface={:?} encounter={:?} battle={:?} checksum={:?}",
        party_index,
        surface,
        field_move.wild_encounter,
        field_move.wild_battle,
        field_move.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!(
            "SWEET SCENT {:?} {:?}",
            field_move.wild_encounter, field_move.wild_battle
        ),
    );
    if field_move.wild_encounter.resolved.is_none() {
        runtime_shell
            .field_notice_queue
            .push_back(visible_asm_text(&snapshot, "SweetScentNothingText")?);
    }
    settle_visible_field_move_after_possible_battle(
        runtime_shell,
        &snapshot,
        visible_field_move_use_text(&snapshot, party_index, "SWEET SCENT")?,
    )?;
    Ok(())
}

fn use_visible_sweet_scent_current_surface(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let Some(surface) = runtime_shell.shell.current_encounter_surface_checked()? else {
        let snapshot = runtime_shell.shell.snapshot()?;
        let party_index = party_index_for_field_move_rule(
            &snapshot,
            &runtime_shell.shell,
            "sweet_scent",
            runtime_shell.party_cursor,
        )?;
        record_visible_runtime_action(runtime_shell, "field_move:sweet_scent:no_surface")?;
        runtime_shell
            .last_audio_events
            .push("Sweet Scent requires grass or surfable water under the player".to_string());
        retain_visible_field_notice_scene(runtime_shell, &snapshot);
        runtime_shell.field_notice = Some(visible_field_move_use_text(
            &snapshot,
            party_index,
            "SWEET SCENT",
        )?);
        runtime_shell
            .field_notice_queue
            .push_back(visible_asm_text(&snapshot, "SweetScentNothingText")?);
        continue_visible_script_after_prompt(runtime_shell)?;
        mark_runtime_snapshot_dirty(runtime_shell);
        set_shell_action_status(runtime_shell, "SWEET SCENT CAN'T BE USED HERE");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    };
    use_visible_sweet_scent(runtime_shell, surface)
}

fn settle_visible_field_action_after_possible_battle(
    runtime_shell: &mut BevyRuntimeShell,
) -> Result<()> {
    if runtime_shell.shell.snapshot()?.battle.is_some() {
        prepare_visible_battle_entry(runtime_shell);
        return settle_visible_battle_after_action(runtime_shell);
    }
    continue_visible_script_after_prompt(runtime_shell)
}

fn settle_visible_field_move_after_possible_battle(
    runtime_shell: &mut BevyRuntimeShell,
    scene: &RuntimeShellSnapshot,
    notice: String,
) -> Result<()> {
    retain_visible_field_notice_scene(runtime_shell, scene);
    if runtime_shell.shell.snapshot()?.battle.is_some() {
        runtime_shell.pending_field_battle_entry = true;
        runtime_shell.field_notice = Some(notice);
        return Ok(());
    }
    runtime_shell.field_notice = Some(notice);
    continue_visible_script_after_prompt(runtime_shell)
}

fn retain_visible_field_notice_scene(
    runtime_shell: &mut BevyRuntimeShell,
    scene: &RuntimeShellSnapshot,
) {
    runtime_shell.field_notice_scene = Some(Arc::new(scene.clone()));
}

fn carried_field_rule_item(
    snapshot: &RuntimeShellSnapshot,
    shell: &RuntimeGameShell,
    rule_id: &str,
) -> Result<String> {
    let Some(item_id) = shell
        .field_move_rule_keys()
        .into_iter()
        .find(|key| key.rule_id == rule_id)
        .and_then(|key| key.item_id)
    else {
        anyhow::bail!("compiled pack has no field item rule {rule_id}");
    };
    if carried_item_ids(snapshot).any(|carried| carried == item_id) {
        Ok(item_id)
    } else {
        anyhow::bail!("bag does not carry field item {item_id} for rule {rule_id}")
    }
}

fn field_rule_item_matches(shell: &RuntimeGameShell, rule_id: &str, item_id: &str) -> bool {
    shell
        .field_move_rule_keys()
        .into_iter()
        .find(|key| key.rule_id == rule_id)
        .and_then(|key| key.item_id)
        .is_some_and(|rule_item_id| rule_item_id == item_id)
}

fn carried_item_ids(snapshot: &RuntimeShellSnapshot) -> impl Iterator<Item = &str> {
    snapshot
        .bag
        .items
        .iter()
        .chain(snapshot.bag.balls.iter())
        .chain(snapshot.bag.key_items.iter())
        .chain(snapshot.bag.pc_items.iter())
        .chain(
            snapshot
                .bag
                .custom_pockets
                .values()
                .flat_map(|items| items.iter()),
        )
        .filter(|item| item.quantity > 0)
        .map(|item| item.item_id.as_str())
}

fn carried_item_quantity(snapshot: &RuntimeShellSnapshot, item_id: &str) -> Option<u16> {
    snapshot
        .bag
        .items
        .iter()
        .chain(snapshot.bag.balls.iter())
        .chain(snapshot.bag.key_items.iter())
        .chain(snapshot.bag.pc_items.iter())
        .chain(
            snapshot
                .bag
                .custom_pockets
                .values()
                .flat_map(|items| items.iter()),
        )
        .find(|item| item.quantity > 0 && item.item_id == item_id)
        .map(|item| item.quantity)
}

fn item_catalog_detail_label(snapshot: &RuntimeShellSnapshot, item_id: &str) -> String {
    let Some(item) = snapshot.items.iter().find(|item| item.item_id == item_id) else {
        return "INVALID ITEM".to_string();
    };
    format!("{} ${}", item.pocket, item.price)
}

fn shop_buy_item_label(snapshot: &RuntimeShellSnapshot, item_id: &str) -> String {
    let Some(item) = snapshot.items.iter().find(|item| item.item_id == item_id) else {
        return format!("{item_id} INVALID ITEM");
    };
    format!("{} ${}", item.name.replace('_', " "), item.price)
}

fn shop_sell_item_label(snapshot: &RuntimeShellSnapshot, item_id: &str) -> String {
    let Some(quantity) = carried_item_quantity(snapshot, item_id) else {
        return format!("{item_id} INVALID INVENTORY");
    };
    let Some(item) = snapshot.items.iter().find(|item| item.item_id == item_id) else {
        return format!("{item_id} x{quantity} INVALID ITEM");
    };
    format!(
        "{} x{} ${}",
        item.name.replace('_', " "),
        quantity,
        u32::from(item.price / 2)
    )
}

fn sellable_carried_item_ids(snapshot: &RuntimeShellSnapshot) -> Vec<String> {
    let mut item_ids = snapshot
        .bag
        .items
        .iter()
        .chain(snapshot.bag.balls.iter())
        .filter(|item| item.quantity > 0)
        .map(|item| item.item_id.clone())
        .collect::<Vec<_>>();
    item_ids.extend(
        snapshot
            .bag
            .custom_pockets
            .values()
            .flat_map(|items| items.iter())
            .filter(|item| item.quantity > 0)
            .map(|item| item.item_id.clone()),
    );
    item_ids
}

fn party_index_for_field_move_rule(
    snapshot: &RuntimeShellSnapshot,
    shell: &RuntimeGameShell,
    rule_id: &str,
    party_cursor: usize,
) -> Result<usize> {
    let Some(move_id) = shell
        .field_move_rule_keys()
        .into_iter()
        .find(|key| key.rule_id == rule_id)
        .and_then(|key| key.move_id)
    else {
        anyhow::bail!("compiled pack has no field move rule {rule_id}");
    };
    if let Some(selected) = snapshot.party.slots.get(party_cursor) {
        if selected
            .pokemon
            .moves
            .iter()
            .any(|learned| learned.name == move_id)
        {
            return Ok(selected.index);
        }
    }
    snapshot
        .party
        .slots
        .iter()
        .find(|slot| {
            slot.pokemon
                .moves
                .iter()
                .any(|learned| learned.name == move_id)
        })
        .map(|slot| slot.index)
        .with_context(|| {
            format!("party has no Pokemon with field move {move_id} for rule {rule_id}")
        })
}

fn visible_field_move_use_text(
    snapshot: &RuntimeShellSnapshot,
    party_index: usize,
    move_name: &str,
) -> Result<String> {
    let nickname = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.index == party_index)
        .map(|slot| slot.pokemon.nickname.as_str())
        .with_context(|| format!("field-move user party index {party_index} is missing"))?;
    Ok(format!("{nickname} used {move_name}!"))
}

fn facing_runtime_tile(snapshot: &RuntimeShellSnapshot) -> Result<TilePosition> {
    facing_runtime_tile_from(snapshot.overworld.tile, snapshot.overworld.facing)
}

fn facing_runtime_tile_from(
    tile: TilePosition,
    facing: crate::core::world::map::Direction,
) -> Result<TilePosition> {
    checked_move_by_stride(tile, facing, StepOptions::default().stride_tiles).with_context(|| {
        format!(
            "facing tile overflows runtime coordinates from ({}, {}) facing {:?}",
            tile.x, tile.y, facing
        )
    })
}

fn runtime_tile_to_metatile_u16(x: i16, y: i16, context: &str) -> Result<(u16, u16)> {
    if x < 0 || y < 0 {
        anyhow::bail!("runtime tile ({x}, {y}) is outside unsigned map coordinates for {context}");
    }
    if x % METATILE_WIDTH != 0 || y % METATILE_WIDTH != 0 {
        anyhow::bail!(
            "runtime tile ({x}, {y}) is not aligned to metatile width {METATILE_WIDTH} for {context}"
        );
    }
    let metatile_x = u16::try_from(x.div_euclid(METATILE_WIDTH)).with_context(|| {
        format!("runtime tile x {x} cannot convert to metatile coordinate for {context}")
    })?;
    let metatile_y = u16::try_from(y.div_euclid(METATILE_WIDTH)).with_context(|| {
        format!("runtime tile y {y} cannot convert to metatile coordinate for {context}")
    })?;
    Ok((metatile_x, metatile_y))
}

fn open_visible_battle_move_target(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(ref battle) = snapshot.battle else {
        return handle_visible_no_active_battle(runtime_shell, "fight_open");
    };
    if battle.commands.player_move_slots.is_empty() {
        runtime_shell.battle_move_cursor = None;
        record_visible_runtime_action(runtime_shell, "battle:fight:no_moves")?;
        runtime_shell
            .last_audio_events
            .push("active battle has no available player moves".to_string());
        set_shell_action_status(runtime_shell, "NO MOVES");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let move_menu_count = battle_move_menu_option_count(&snapshot, battle)
        .unwrap_or_else(|| battle.commands.player_move_slots.len() + 1);
    visible_cursor_index(
        &mut runtime_shell.battle_move_cursor,
        "battle:moves",
        move_menu_count,
    );
    runtime_shell.battle_move_swap_origin = None;
    runtime_shell.battle_switch_cursor = None;
    runtime_shell.pending_battle_move_switch_slot = None;
    runtime_shell.battle_party_action_cursor = None;
    runtime_shell.battle_party_summary_open = false;
    runtime_shell.bag_cursor = None;
    runtime_shell.ball_cursor = None;
    runtime_shell.battle_pack_target_mode = None;
    runtime_shell.party_move_cursor = None;
    runtime_shell.last_audio_events.push(format!(
        "opened battle moves choices={:?}",
        battle.commands.player_move_slots
    ));
    set_shell_action_status(
        runtime_shell,
        format!("FIGHT MOVES {}", battle.commands.player_move_slots.len()),
    );
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn open_visible_battle_switch_target(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(ref battle) = snapshot.battle else {
        return handle_visible_no_active_battle(runtime_shell, "switch_open");
    };
    if snapshot.party.slots.is_empty() {
        runtime_shell.battle_switch_cursor = None;
        runtime_shell.battle_party_action_cursor = None;
        runtime_shell.battle_party_summary_open = false;
        record_visible_runtime_action(runtime_shell, "battle:pokemon:no_switches")?;
        runtime_shell
            .last_audio_events
            .push("active battle has no available party switches".to_string());
        set_shell_action_status(runtime_shell, "NO SWITCHES");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    visible_cursor_index(
        &mut runtime_shell.battle_switch_cursor,
        "battle:switch",
        battle_switch_option_count(&snapshot),
    );
    runtime_shell.battle_party_action_cursor = None;
    runtime_shell.battle_party_summary_open = false;
    runtime_shell.battle_move_cursor = None;
    runtime_shell.battle_move_swap_origin = None;
    runtime_shell.pending_battle_move_switch_slot = None;
    runtime_shell.bag_cursor = None;
    runtime_shell.ball_cursor = None;
    runtime_shell.battle_pack_target_mode = None;
    runtime_shell.field_pack_target_mode = None;
    runtime_shell.party_action_cursor = None;
    runtime_shell.party_switch_cursor = None;
    runtime_shell.party_move_cursor = None;
    runtime_shell.last_audio_events.push(format!(
        "opened battle switch choices={:?}",
        battle.commands.switch_party_indices
    ));
    set_shell_action_status(
        runtime_shell,
        format!(
            "SWITCH CHOICES {}",
            battle.commands.switch_party_indices.len()
        ),
    );
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn open_visible_battle_move_switch_target(
    runtime_shell: &mut BevyRuntimeShell,
    move_slot: usize,
) -> Result<()> {
    open_visible_battle_switch_target(runtime_shell)?;
    if runtime_shell.battle_switch_cursor.is_some() {
        runtime_shell.pending_battle_move_switch_slot = Some(move_slot);
        record_visible_runtime_action(
            runtime_shell,
            format!("battle:move_switch:{move_slot}:target_open"),
        )?;
        runtime_shell.last_audio_events.push(format!(
            "opened battle move-switch target for slot {move_slot}"
        ));
        set_shell_action_status(runtime_shell, "BATON PASS TARGET");
        trim_event_log(&mut runtime_shell.last_audio_events);
    }
    Ok(())
}

fn switch_visible_battle_pokemon(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(ref battle) = snapshot.battle else {
        return handle_visible_no_active_battle(runtime_shell, "switch_confirm");
    };
    let selected_index = visible_cursor_index(
        &mut runtime_shell.battle_switch_cursor,
        "battle:switch",
        battle_switch_option_count(&snapshot),
    );
    if selected_index >= snapshot.party.slots.len() {
        return press_visible_battle_b_button(runtime_shell);
    }
    let slot = &snapshot.party.slots[selected_index];
    let party_index = slot.index;
    if battle.active_player_party_index == Some(party_index) {
        record_visible_runtime_action(runtime_shell, "battle:switch:already_active")?;
        runtime_shell
            .battle_messages
            .push_back(format!("{} is already out!", slot.pokemon.nickname));
        runtime_shell.battle_message_scene = Some(Box::new(snapshot.clone()));
        mark_runtime_snapshot_dirty(runtime_shell);
        set_shell_action_status(
            runtime_shell,
            format!("{} IS ALREADY OUT", slot.pokemon.nickname),
        );
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if slot.pokemon.hp == 0 || slot.pokemon.is_egg {
        record_visible_runtime_action(runtime_shell, "battle:switch:no_will")?;
        runtime_shell
            .battle_messages
            .push_back("There's no will to battle!".to_string());
        runtime_shell.battle_message_scene = Some(Box::new(snapshot.clone()));
        mark_runtime_snapshot_dirty(runtime_shell);
        set_shell_action_status(runtime_shell, "THERE'S NO WILL TO BATTLE");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if let Some(move_slot) = runtime_shell.pending_battle_move_switch_slot {
        return resolve_visible_battle_move_switch_to(runtime_shell, move_slot, party_index);
    }
    if trainer_shift_switch_pending(&snapshot, battle) {
        switch_visible_trainer_shift_party_without_turn(runtime_shell, party_index)?;
        return advance_visible_trainer_battle(runtime_shell);
    }
    if !visible_active_battle_player_fainted(&snapshot)
        && (battle.player_cannot_escape || battle.player_wrapped)
    {
        let active_name = battle
            .active_player_party_index
            .and_then(|active| snapshot.party.slots.iter().find(|slot| slot.index == active))
            .map(|slot| slot.pokemon.nickname.as_str())
            .unwrap_or("POKEMON");
        record_visible_runtime_action(runtime_shell, "battle:switch:trapped")?;
        runtime_shell
            .battle_messages
            .push_back(format!("{active_name} can't be recalled!"));
        runtime_shell.battle_message_scene = Some(Box::new(snapshot.clone()));
        mark_runtime_snapshot_dirty(runtime_shell);
        set_shell_action_status(runtime_shell, "POKEMON CAN'T BE RECALLED");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    switch_visible_battle_pokemon_to(runtime_shell, party_index)
}

fn battle_switch_option_count(snapshot: &RuntimeShellSnapshot) -> usize {
    snapshot.party.slots.len() + usize::from(!visible_active_battle_player_fainted(snapshot))
}

fn resolve_visible_battle_move_switch_to(
    runtime_shell: &mut BevyRuntimeShell,
    move_slot: usize,
    party_index: usize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(ref battle) = snapshot.battle else {
        return handle_visible_no_active_battle(runtime_shell, "move_switch_to");
    };
    let battle_before_turn = battle.clone();
    if !battle.commands.player_move_slots.contains(&move_slot) {
        record_visible_runtime_action(
            runtime_shell,
            format!("battle:move_switch:{move_slot}:unavailable"),
        )?;
        runtime_shell
            .last_audio_events
            .push(format!("player move slot {move_slot} is not available"));
        set_shell_action_status(runtime_shell, "MOVE UNAVAILABLE");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if selected_battle_move_effect(&snapshot, move_slot)? != "BATON_PASS" {
        anyhow::bail!("move_switch action requires BATON_PASS effect at slot {move_slot}");
    }
    if !battle.commands.switch_party_indices.contains(&party_index) {
        record_visible_runtime_action(
            runtime_shell,
            format!("battle:move_switch:{move_slot}:{party_index}:unavailable"),
        )?;
        runtime_shell.last_audio_events.push(format!(
            "party index {party_index} is not an available battle move switch"
        ));
        set_shell_action_status(runtime_shell, "SWITCH UNAVAILABLE");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let (enemy_action, enemy_rng_seed_after) =
        selected_enemy_battle_action(&snapshot, battle, &mut runtime_shell.trainer_items_used)?;
    persist_selected_enemy_trainer_item(runtime_shell, battle, &enemy_action)?;
    let enemy_slot = match &enemy_action {
        BattleAction::Move { slot } => Some(*slot),
        _ => None,
    };
    record_visible_runtime_action(
        runtime_shell,
        format!("battle:move_switch:{move_slot}:{party_index}:enemy:{enemy_action:?}"),
    )?;
    record_visible_battle_action_frame(
        runtime_shell,
        BattleAction::MoveSwitch {
            slot: move_slot,
            party_index,
        },
    )?;
    let turn = resolve_active_battle_turn_with_enemy_rng(
        runtime_shell,
        enemy_rng_seed_after,
        BattleAction::MoveSwitch {
            slot: move_slot,
            party_index,
        },
        enemy_action,
    )?;
    reset_visible_battle_action_cursors(runtime_shell);
    stage_visible_battle_messages(runtime_shell, &snapshot, &turn.outcome.events);
    let events = format_battle_turn_events(&turn.outcome.events);
    runtime_shell.last_audio_events.push(format!(
        "battle move-switch move_slot={} party_index={} enemy_slot={} {} events={} checksum={:?}",
        move_slot,
        party_index,
        enemy_slot.map_or_else(|| "switch".to_string(), |slot| slot.to_string()),
        format_battle_turn_summary(&turn.outcome),
        events,
        turn.state_checksum
    ));
    defer_visible_party_index_cry_after_send_out(
        runtime_shell,
        party_index,
        "battle_move_switch",
    )?;
    trim_event_log(&mut runtime_shell.last_audio_events);
    set_shell_action_status(
        runtime_shell,
        format!(
            "BATON PASS PARTY #{} {}",
            party_index,
            format_battle_turn_summary(&turn.outcome)
        ),
    );
    settle_visible_resolved_battle_turn(runtime_shell, &battle_before_turn)
}

fn switch_visible_battle_pokemon_to(
    runtime_shell: &mut BevyRuntimeShell,
    party_index: usize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(ref battle) = snapshot.battle else {
        return handle_visible_no_active_battle(runtime_shell, "switch_to");
    };
    let battle_before_turn = battle.clone();
    if !battle.commands.switch_party_indices.contains(&party_index) {
        record_visible_runtime_action(
            runtime_shell,
            format!("battle:switch:{party_index}:unavailable"),
        )?;
        runtime_shell.last_audio_events.push(format!(
            "party index {party_index} is not an available battle switch"
        ));
        set_shell_action_status(runtime_shell, "SWITCH UNAVAILABLE");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if visible_active_battle_player_fainted(&snapshot) {
        return switch_visible_battle_party_without_turn(runtime_shell, party_index);
    }
    let (enemy_action, enemy_rng_seed_after) =
        selected_enemy_battle_action(&snapshot, battle, &mut runtime_shell.trainer_items_used)?;
    persist_selected_enemy_trainer_item(runtime_shell, battle, &enemy_action)?;
    let enemy_slot = match &enemy_action {
        BattleAction::Move { slot } => Some(*slot),
        _ => None,
    };
    record_visible_runtime_action(
        runtime_shell,
        format!("battle:switch:{party_index}:enemy:{enemy_action:?}"),
    )?;
    record_visible_battle_action_frame(runtime_shell, BattleAction::Switch { party_index })?;
    let turn = resolve_active_battle_turn_with_enemy_rng(
        runtime_shell,
        enemy_rng_seed_after,
        BattleAction::Switch { party_index },
        enemy_action,
    )?;
    reset_visible_battle_action_cursors(runtime_shell);
    stage_visible_battle_messages(runtime_shell, &snapshot, &turn.outcome.events);
    let events = format_battle_turn_events(&turn.outcome.events);
    runtime_shell.last_audio_events.push(format!(
        "battle switch party_index={} enemy_slot={} {} events={} checksum={:?}",
        party_index,
        enemy_slot.map_or_else(|| "switch".to_string(), |slot| slot.to_string()),
        format_battle_turn_summary(&turn.outcome),
        events,
        turn.state_checksum
    ));
    defer_visible_party_index_cry_after_send_out(runtime_shell, party_index, "battle_switch")?;
    trim_event_log(&mut runtime_shell.last_audio_events);
    set_shell_action_status(
        runtime_shell,
        format!(
            "SWITCH PARTY #{} {}",
            party_index,
            format_battle_turn_summary(&turn.outcome)
        ),
    );
    settle_visible_resolved_battle_turn(runtime_shell, &battle_before_turn)
}

fn settle_visible_battle_after_action(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    const MAX_BATTLE_SETTLE_STEPS: usize = 8;
    for _ in 0..MAX_BATTLE_SETTLE_STEPS {
        let snapshot = runtime_shell.shell.snapshot()?;
        let Some(battle) = snapshot.battle.as_ref() else {
            runtime_shell.trainer_items_used.clear();
            return Ok(());
        };
        if visible_active_battle_player_fainted(&snapshot) {
            handle_visible_player_fainted_battle_boundary(runtime_shell, &snapshot, battle)?;
            return Ok(());
        }
        if battle.enemy_pokemon.hp != 0 {
            return Ok(());
        }
        match battle.kind {
            RuntimeBattleKind::Trainer { .. } => {
                let Some(enemy_index) = battle.active_enemy_party_index else {
                    anyhow::bail!("active trainer battle has no active enemy party index");
                };
                if battle.rewarded_enemy_party_indices.contains(&enemy_index) {
                    advance_visible_trainer_battle(runtime_shell)?;
                } else {
                    claim_visible_battle_rewards(runtime_shell)?;
                    // Keep the defeated battler on screen for the complete
                    // EXP/level/move/evolution narration. The final text
                    // acknowledgement resumes settlement and reveals the
                    // replacement at its send-out boundary.
                    return Ok(());
                }
            }
            RuntimeBattleKind::Wild { .. } | RuntimeBattleKind::StaticWild { .. } => {
                claim_visible_battle_rewards(runtime_shell)?;
            }
        }
    }
    anyhow::bail!(
        "battle settlement exceeded {MAX_BATTLE_SETTLE_STEPS} reward/advance steps before reaching player control"
    )
}

fn visible_active_battle_player_fainted(snapshot: &RuntimeShellSnapshot) -> bool {
    let active = snapshot
        .party
        .slots
        .iter()
        .find(|slot| slot.is_active_battle_pokemon);
    if snapshot.battle.is_some() {
        active.is_some_and(|slot| slot.pokemon.hp == 0)
    } else {
        active.is_some_and(|slot| slot.pokemon.hp == 0)
    }
}

fn should_offer_trainer_shift_switch(
    snapshot: &RuntimeShellSnapshot,
    battle: &crate::RuntimeBattleSnapshot,
) -> bool {
    snapshot.trainer.options.battle_style == BattleStyle::Shift
        && trainer_shift_switch_pending(snapshot, battle)
        && !battle.commands.switch_party_indices.is_empty()
}

fn trainer_shift_switch_pending(
    _snapshot: &RuntimeShellSnapshot,
    battle: &crate::RuntimeBattleSnapshot,
) -> bool {
    if !matches!(battle.kind, RuntimeBattleKind::Trainer { .. }) {
        return false;
    }
    if battle.enemy_pokemon.hp != 0 {
        return false;
    }
    let Some(enemy_index) = battle.active_enemy_party_index else {
        return false;
    };
    if !battle.rewarded_enemy_party_indices.contains(&enemy_index) {
        return false;
    }
    battle
        .enemy_party
        .iter()
        .enumerate()
        .any(|(index, pokemon)| index != enemy_index && pokemon.hp > 0)
}

fn confirm_visible_trainer_shift_prompt(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let selected = strict_readonly_cursor_index(
        &runtime_shell.battle_shift_prompt_cursor,
        "battle:shift-prompt",
        2,
    )
    .context("trainer Shift prompt requires a valid YES/NO cursor")?;
    resolve_visible_trainer_shift_prompt(runtime_shell, selected == 0)
}

fn confirm_visible_wild_faint_prompt(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let selected = strict_readonly_cursor_index(
        &runtime_shell.battle_faint_prompt_cursor,
        "battle:faint-prompt",
        2,
    )
    .context("wild faint prompt requires a valid YES/NO cursor")?;
    resolve_visible_wild_faint_prompt(runtime_shell, selected == 0)
}

fn resolve_visible_wild_faint_prompt(
    runtime_shell: &mut BevyRuntimeShell,
    use_next_pokemon: bool,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let battle = snapshot
        .battle
        .as_ref()
        .context("wild faint prompt requires an active battle")?;
    if matches!(&battle.kind, RuntimeBattleKind::Trainer { .. })
        || !visible_active_battle_player_fainted(&snapshot)
        || battle.commands.switch_party_indices.is_empty()
    {
        anyhow::bail!("wild faint prompt is active outside a replaceable wild faint boundary");
    }
    runtime_shell.battle_faint_prompt_cursor = None;
    if use_next_pokemon {
        record_visible_runtime_action(runtime_shell, "battle:faint_prompt:yes")?;
        open_visible_battle_switch_target(runtime_shell)?;
        set_shell_action_status(runtime_shell, "CHOOSE NEXT POKEMON");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let scripted_static_wild = visible_static_wild_source(&snapshot, battle);
    record_visible_runtime_action(runtime_shell, "battle:faint_prompt:no")?;
    let escape = runtime_shell.shell.attempt_escape_active_wild_battle()?;
    runtime_shell.last_audio_events.push(format!(
        "wild faint replacement escape outcome={:?} checksum={:?}",
        escape.outcome, escape.state_checksum
    ));
    runtime_shell.battle_message_scene = Some(Box::new(snapshot));
    if escape.outcome.escaped {
        runtime_shell
            .battle_messages
            .push_back("Got away safely!".to_string());
        finish_visible_wild_battle_exit(
            runtime_shell,
            scripted_static_wild,
            "wild_faint_replacement_escape",
        )
    } else {
        runtime_shell
            .battle_messages
            .push_back("Can't escape!".to_string());
        open_visible_battle_switch_target(runtime_shell)?;
        set_shell_action_status(runtime_shell, "CHOOSE NEXT POKEMON");
        mark_runtime_snapshot_dirty(runtime_shell);
        trim_event_log(&mut runtime_shell.last_audio_events);
        Ok(())
    }
}

fn resolve_visible_trainer_shift_prompt(
    runtime_shell: &mut BevyRuntimeShell,
    change_pokemon: bool,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let battle = snapshot
        .battle
        .as_ref()
        .context("trainer Shift prompt requires an active battle")?;
    if !trainer_shift_switch_pending(&snapshot, battle) {
        anyhow::bail!("trainer Shift prompt is active outside the enemy replacement boundary");
    }
    runtime_shell.battle_shift_prompt_cursor = None;
    if change_pokemon {
        record_visible_runtime_action(runtime_shell, "battle:shift_prompt:yes")?;
        open_visible_battle_switch_target(runtime_shell)?;
        set_shell_action_status(runtime_shell, "SHIFT: CHOOSE POKEMON");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    record_visible_runtime_action(runtime_shell, "battle:shift_prompt:no")?;
    reset_visible_battle_action_cursors(runtime_shell);
    set_shell_action_status(runtime_shell, "SHIFT: KEEP CURRENT");
    trim_event_log(&mut runtime_shell.last_audio_events);
    advance_visible_trainer_battle(runtime_shell)
}

fn handle_visible_player_fainted_battle_boundary(
    runtime_shell: &mut BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
    battle: &crate::RuntimeBattleSnapshot,
) -> Result<()> {
    match battle.commands.switch_party_indices.as_slice() {
        [] => {
            reset_visible_battle_action_cursors(runtime_shell);
            if battle.battle_type == "BATTLETYPE_CANLOSE" {
                let RuntimeBattleKind::Trainer {
                    source_script,
                    loss_text,
                    ..
                } = &battle.kind
                else {
                    anyhow::bail!("BATTLETYPE_CANLOSE is only valid for a trainer battle");
                };
                let map_name = snapshot.overworld.map_name.clone();
                queue_visible_trainer_result_text(runtime_shell, snapshot, loss_text)?;
                reset_visible_battle_exit_state(runtime_shell);
                complete_visible_scripted_trainer_battle(
                    runtime_shell,
                    &map_name,
                    source_script,
                    false,
                    true,
                )
            } else {
                resolve_visible_blackout(runtime_shell)
            }
        }
        _ => {
            if !matches!(&battle.kind, RuntimeBattleKind::Trainer { .. }) {
                runtime_shell.battle_action_cursor = None;
                runtime_shell.battle_switch_cursor = None;
                visible_cursor_index(
                    &mut runtime_shell.battle_faint_prompt_cursor,
                    "battle:faint-prompt",
                    2,
                );
                set_shell_action_status(runtime_shell, "USE NEXT POKEMON?");
                trim_event_log(&mut runtime_shell.last_audio_events);
                return Ok(());
            }
            let actions = visible_battle_action_ids(snapshot, battle);
            let pokemon_action_index = actions
                .iter()
                .position(|action| *action == VisibleBattleAction::Pokemon)
                .context("fainted active Pokemon has switch targets but no Pokemon action")?;
            runtime_shell.battle_action_cursor = Some(MenuCursor {
                surface_id: "battle:actions".to_string(),
                option_index: pokemon_action_index,
            });
            visible_cursor_index(
                &mut runtime_shell.battle_switch_cursor,
                "battle:switch",
                battle_switch_option_count(snapshot),
            );
            runtime_shell.last_audio_events.push(format!(
                "active Pokemon fainted; awaiting replacement choices={}",
                battle.commands.switch_party_indices.len()
            ));
            set_shell_action_status(
                runtime_shell,
                format!(
                    "CHOOSE REPLACEMENT {}",
                    battle.commands.switch_party_indices.len()
                ),
            );
            trim_event_log(&mut runtime_shell.last_audio_events);
            Ok(())
        }
    }
}

fn switch_visible_battle_party_without_turn(
    runtime_shell: &mut BevyRuntimeShell,
    party_index: usize,
) -> Result<()> {
    record_visible_runtime_action(runtime_shell, format!("battle:replacement:{party_index}"))?;
    let switched = runtime_shell
        .shell
        .switch_active_battle_party(party_index)?;
    reset_visible_battle_action_cursors(runtime_shell);
    runtime_shell.last_audio_events.push(format!(
        "battle replacement party_index={} checksum={:?}",
        switched.party_index, switched.state_checksum
    ));
    let replacement = runtime_shell.shell.snapshot()?;
    let nickname = replacement
        .party
        .slots
        .iter()
        .find(|slot| slot.index == switched.party_index)
        .map(|slot| slot.pokemon.nickname.as_str())
        .context("battle replacement is missing its selected party slot")?;
    let send_out_message = format!("Go! {nickname}!");
    runtime_shell.battle_messages.push_back(send_out_message.clone());
    runtime_shell.battle_player_send_out_pending = true;
    runtime_shell.battle_message_scene = Some(Box::new(replacement));
    mark_runtime_snapshot_dirty(runtime_shell);
    let species_id = runtime_shell
        .shell
        .snapshot()?
        .party
        .slots
        .iter()
        .find(|slot| slot.index == switched.party_index)
        .map(|slot| slot.pokemon.species.id.clone())
        .context("battle replacement is missing its selected party species")?;
    defer_visible_battle_cry_after_message(
        runtime_shell,
        species_id,
        "battle_replacement",
        send_out_message,
    );
    set_shell_action_status(
        runtime_shell,
        format!("SENT PARTY #{}", switched.party_index),
    );
    trim_event_log(&mut runtime_shell.last_audio_events);
    settle_visible_battle_after_action(runtime_shell)?;
    Ok(())
}

fn switch_visible_trainer_shift_party_without_turn(
    runtime_shell: &mut BevyRuntimeShell,
    party_index: usize,
) -> Result<()> {
    record_visible_runtime_action(runtime_shell, format!("battle:shift_switch:{party_index}"))?;
    let switched = runtime_shell
        .shell
        .switch_active_battle_party(party_index)?;
    reset_visible_battle_action_cursors(runtime_shell);
    runtime_shell.last_audio_events.push(format!(
        "trainer shift switch party_index={} checksum={:?}",
        switched.party_index, switched.state_checksum
    ));
    let replacement = runtime_shell.shell.snapshot()?;
    let nickname = replacement
        .party
        .slots
        .iter()
        .find(|slot| slot.index == switched.party_index)
        .map(|slot| slot.pokemon.nickname.as_str())
        .context("trainer Shift switch is missing its selected party slot")?;
    let send_out_message = format!("Go! {nickname}!");
    runtime_shell.battle_messages.push_back(send_out_message.clone());
    runtime_shell.battle_player_send_out_pending = true;
    runtime_shell.battle_message_scene = Some(Box::new(replacement));
    mark_runtime_snapshot_dirty(runtime_shell);
    let species_id = runtime_shell
        .shell
        .snapshot()?
        .party
        .slots
        .iter()
        .find(|slot| slot.index == switched.party_index)
        .map(|slot| slot.pokemon.species.id.clone())
        .context("trainer Shift switch is missing its selected party species")?;
    defer_visible_battle_cry_after_message(
        runtime_shell,
        species_id,
        "trainer_shift_switch",
        send_out_message,
    );
    set_shell_action_status(
        runtime_shell,
        format!("SHIFT SENT PARTY #{}", switched.party_index),
    );
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn throw_visible_battle_ball(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.battle.is_none() {
        return handle_visible_no_active_battle(runtime_shell, "ball_throw");
    }
    if carried_ball_item_ids(&snapshot).is_empty() {
        runtime_shell.ball_cursor = None;
        record_visible_runtime_action(runtime_shell, "battle:ball:throw:no_items")?;
        runtime_shell
            .last_audio_events
            .push("bag has no carried ball".to_string());
        runtime_shell
            .battle_messages
            .push_back("You don't have any BALLS.".to_string());
        runtime_shell.battle_message_scene = Some(Box::new(snapshot));
        mark_runtime_snapshot_dirty(runtime_shell);
        set_shell_action_status(runtime_shell, "NO BALLS");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let (ball_index, ball_id) = selected_battle_ball_id(runtime_shell)?;
    throw_visible_battle_ball_id(runtime_shell, ball_index, ball_id)
}

fn throw_visible_battle_ball_at(
    runtime_shell: &mut BevyRuntimeShell,
    ball_index: usize,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(ref battle) = snapshot.battle else {
        return handle_visible_no_active_battle(runtime_shell, "ball_throw_at");
    };
    if !battle.commands.can_use_items {
        record_visible_runtime_action(runtime_shell, "battle:ball:items_unavailable")?;
        runtime_shell
            .last_audio_events
            .push("active battle does not allow item use".to_string());
        runtime_shell
            .battle_messages
            .push_back("Items can't be used here.".to_string());
        runtime_shell.battle_message_scene = Some(Box::new(snapshot.clone()));
        mark_runtime_snapshot_dirty(runtime_shell);
        set_shell_action_status(runtime_shell, "ITEMS UNAVAILABLE");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let ball_ids = carried_ball_item_ids(&snapshot);
    let Some(ball_id) = ball_ids.get(ball_index).cloned() else {
        record_visible_runtime_action(
            runtime_shell,
            format!("battle:ball:index:{}:unavailable", ball_index + 1),
        )?;
        runtime_shell
            .last_audio_events
            .push(format!("bag has no ball at index {}", ball_index + 1));
        set_shell_action_status(runtime_shell, "BALL UNAVAILABLE");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    };
    throw_visible_battle_ball_id(runtime_shell, ball_index, ball_id)
}

fn throw_visible_battle_ball_id(
    runtime_shell: &mut BevyRuntimeShell,
    ball_index: usize,
    ball_id: String,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.battle.is_none() {
        return handle_visible_no_active_battle(runtime_shell, "ball_throw_id");
    }
    let scripted_static_wild = snapshot
        .battle
        .as_ref()
        .and_then(|battle| visible_static_wild_source(&snapshot, battle));
    record_visible_runtime_action(
        runtime_shell,
        format!("battle:ball:{ball_id}:index:{}", ball_index + 1),
    )?;
    let capture = runtime_shell.shell.throw_ball_at_active_battle(&ball_id)?;
    if capture.outcome.as_ref().is_some_and(|outcome| outcome.storage_full) {
        runtime_shell
            .battle_messages
            .push_back("The POKéMON BOX\nis full. That\ncan't be used now.".to_string());
        runtime_shell.battle_message_scene = Some(Box::new(snapshot));
        record_visible_runtime_action(
            runtime_shell,
            format!("battle:capture_storage_full:{ball_id}:index:{}", ball_index + 1),
        )?;
        mark_runtime_snapshot_dirty(runtime_shell);
        set_shell_action_status(runtime_shell, "BOX FULL");
        return Ok(());
    }
    record_visible_battle_item_action_frame(runtime_shell, &ball_id)?;
    let use_message = format!(
        "{} used the {}.",
        visible_battle_player_name(&snapshot),
        item_display_name(&snapshot, &ball_id)
    );
    runtime_shell.battle_messages.push_back(use_message.clone());
    runtime_shell.battle_message_scene = Some(Box::new(snapshot.clone()));
    mark_runtime_snapshot_dirty(runtime_shell);
    runtime_shell.last_audio_events.push(format!(
        "threw ball_index={} ball={} outcome={:?} checksum={:?}",
        ball_index + 1,
        ball_id,
        capture.outcome,
        capture.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        visible_capture_attempt_status(&ball_id, capture.outcome.as_ref()),
    );
    reset_visible_battle_action_cursors(runtime_shell);
    if let Some(outcome) = capture.outcome.as_ref() {
        runtime_shell.visible_capture_animation = Some(VisibleCaptureAnimation {
            trigger_message: use_message,
            ball_id: ball_id.clone(),
            animation_shakes: outcome.animation_shakes,
            blocked: outcome.blocked,
            caught: outcome.caught,
            started: false,
            complete: false,
            frame: 0,
        });
        if outcome.blocked {
            runtime_shell
                .battle_messages
                .push_back("The trainer blocked the BALL!".to_string());
            runtime_shell
                .battle_messages
                .push_back("Don't be a thief!".to_string());
            runtime_shell.pending_enemy_response_after_capture =
                Some((ball_id.clone(), "Don't be a thief!".to_string()));
            record_visible_runtime_action(
                runtime_shell,
                format!("battle:capture_blocked:{ball_id}:index:{}", ball_index + 1),
            )?;
        } else if outcome.caught {
            let enemy_name = snapshot
                .battle
                .as_ref()
                .map(|battle| battle.enemy_pokemon.nickname.as_str())
                .context("caught outcome is missing its active enemy")?;
            let enemy_species = snapshot
                .battle
                .as_ref()
                .map(|battle| battle.enemy_pokemon.species.id.as_str())
                .context("caught outcome is missing its active enemy species")?;
            runtime_shell
                .battle_messages
                .push_back(format!("Gotcha! {enemy_name}\nwas caught!"));
            let battle_type = snapshot
                .battle
                .as_ref()
                .map(|battle| battle.battle_type.as_str())
                .unwrap_or_default();
            if battle_type != "BATTLETYPE_TUTORIAL"
                && !snapshot
                    .progression
                    .pokedex_caught_species
                    .contains(enemy_species)
            {
                runtime_shell.battle_messages.push_back(format!(
                    "{enemy_name}'s data\nwas newly added to\nthe POKéDEX."
                ));
            }
            runtime_shell.battle_message_scene = Some(Box::new(snapshot.clone()));
            record_visible_runtime_action(
                runtime_shell,
                format!("battle:capture_complete:{ball_id}:index:{}", ball_index + 1),
            )?;
            if matches!(
                battle_type,
                "BATTLETYPE_TUTORIAL"
                    | "BATTLETYPE_CONTEST"
                    | "BATTLETYPE_BUG_CONTEST"
                    | "BATTLETYPE_PARK"
            ) {
                complete_visible_standard_capture(
                    runtime_shell,
                    outcome.clone(),
                    None,
                    scripted_static_wild,
                )?;
            } else {
                runtime_shell.pending_standard_capture = Some(PendingStandardCapture {
                    outcome: outcome.clone(),
                    scripted_static_wild,
                    default_name: enemy_name.to_string(),
                });
            }
        } else {
            let enemy_name = snapshot
                .battle
                .as_ref()
                .map(|battle| battle.enemy_pokemon.nickname.as_str())
                .unwrap_or("POKéMON");
            let text = match outcome.wobble_count.min(3) {
                0 => format!("Oh no! The {enemy_name}\nbroke free!"),
                1 => "Aww! It appeared\nto be caught!".to_string(),
                2 => "Aargh!\nAlmost had it!".to_string(),
                _ => "Shoot! It was so\nclose too!".to_string(),
            };
            runtime_shell.battle_messages.push_back(text.clone());
            runtime_shell.pending_enemy_response_after_capture = Some((ball_id.clone(), text));
        }
        mark_runtime_snapshot_dirty(runtime_shell);
    }
    Ok(())
}

fn visible_capture_attempt_status(
    ball_id: &str,
    outcome: Option<&crate::core::battle::capture::CaptureOutcome>,
) -> String {
    let Some(outcome) = outcome else {
        return format!("THREW {ball_id}");
    };
    if outcome.blocked {
        if outcome.storage_full {
            return format!("{ball_id} STORAGE FULL");
        }
        return format!("{ball_id} BLOCKED");
    }
    if outcome.caught {
        return format!("{ball_id} CAUGHT");
    }
    format!("{ball_id} SHAKES {}", outcome.animation_shakes)
}

fn visible_capture_completion_status(completion: &crate::RuntimeCaptureCompletion) -> String {
    let Some(stored) = completion.stored.as_ref() else {
        return "CAPTURE COMPLETE".to_string();
    };
    let location = match stored.location {
        crate::core::models::CaptureStorageLocation::Party { slot } => {
            format!("PARTY #{}", slot + 1)
        }
        crate::core::models::CaptureStorageLocation::Pc { box_index, slot } => {
            format!("BOX {} #{}", box_index + 1, slot + 1)
        }
    };
    compact_scene_label(
        &format!("CAUGHT {} -> {location}", stored.pokemon.species.id),
        76,
    )
}

fn complete_visible_standard_capture(
    runtime_shell: &mut BevyRuntimeShell,
    outcome: crate::core::battle::capture::CaptureOutcome,
    nickname: Option<String>,
    scripted_static_wild: Option<(String, String)>,
) -> Result<()> {
    let battle_before_completion = runtime_shell.shell.snapshot()?;
    let completion = runtime_shell
        .shell
        .complete_active_wild_capture(&outcome, nickname)?;
    if let Some(stored) = completion.stored.as_ref() {
        if matches!(
            stored.location,
            crate::core::models::CaptureStorageLocation::Pc { .. }
        ) {
            runtime_shell.battle_messages.push_back(format!(
                "{} was\nsent to BILL's PC.",
                stored.pokemon.nickname
            ));
            runtime_shell.battle_message_scene = Some(Box::new(battle_before_completion.clone()));
        }
    }
    runtime_shell.last_audio_events.push(format!(
        "capture complete stored={:?} checksum={:?}",
        completion.stored, completion.state_checksum
    ));
    if let Some(caught_species) = completion
        .stored
        .as_ref()
        .map(|stored| stored.pokemon.species.id.clone())
        .or_else(|| {
            completion
                .contest_pokemon
                .as_ref()
                .map(|pokemon| pokemon.species.id.clone())
        })
    {
        queue_visible_pokemon_cry(runtime_shell, &caught_species, "battle_capture_complete")?;
    }
    set_shell_action_status(
        runtime_shell,
        visible_capture_completion_status(&completion),
    );
    queue_visible_pay_day_payout(runtime_shell, &battle_before_completion);
    finish_visible_wild_battle_exit(runtime_shell, scripted_static_wild, "battle_capture")
}

fn queue_visible_pay_day_payout(
    runtime_shell: &mut BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
) {
    let Some(battle) = snapshot.battle.as_ref() else {
        return;
    };
    queue_visible_pay_day_payout_for_battle(
        runtime_shell,
        battle,
        &snapshot.trainer.player_name,
    );
}

fn queue_visible_pay_day_payout_for_battle(
    runtime_shell: &mut BevyRuntimeShell,
    battle: &crate::RuntimeBattleSnapshot,
    _player_name: &str,
) {
    let mut payout = battle.pay_day_money.min(0x00ff_ffff);
    if battle.amulet_coin_active {
        payout = payout.saturating_mul(2).min(0x00ff_ffff);
    }
    if payout > 0 {
        runtime_shell
            .battle_messages
            .push_back(format!("You picked up ¥{payout}!"));
    }
}

fn claim_visible_battle_rewards(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(battle) = snapshot.battle.clone() else {
        return handle_visible_no_active_battle(runtime_shell, "claim_rewards");
    };
    let reward_recipient_name = battle
        .active_player_party_index
        .and_then(|index| snapshot.party.slots.iter().find(|slot| slot.index == index))
        .map(|slot| slot.pokemon.nickname.clone())
        .context("battle rewards are missing the active player Pokemon")?;
    let map_name = snapshot.overworld.map_name.clone();
    let plain_wild_battle = matches!(battle.kind, crate::RuntimeBattleKind::Wild { .. });
    let trainer_battle = matches!(battle.kind, crate::RuntimeBattleKind::Trainer { .. });
    let reward_action = match &battle.kind {
        crate::RuntimeBattleKind::Wild { map_name, .. } => {
            format!("battle:claim_rewards:wild:{map_name}")
        }
        crate::RuntimeBattleKind::StaticWild {
            source_script,
            species,
            level,
            ..
        } => format!("battle:claim_rewards:static_wild:{source_script}:{species}:{level}"),
        crate::RuntimeBattleKind::Trainer {
            source_script,
            trainer_class,
            trainer_id,
            ..
        } => format!("battle:claim_rewards:trainer:{source_script}:{trainer_class}:{trainer_id}"),
    };
    record_visible_runtime_action(runtime_shell, reward_action)?;
    let message = match battle.kind {
        crate::RuntimeBattleKind::Wild { .. } => {
            let rewards = runtime_shell.shell.claim_active_wild_battle_rewards()?;
            set_shell_action_status(
                runtime_shell,
                visible_battle_reward_status(&rewards.outcome),
            );
            push_visible_battle_reward_events(
                runtime_shell,
                &rewards.outcome,
                &reward_recipient_name,
            )?;
            queue_visible_pay_day_payout(runtime_shell, &snapshot);
            retain_visible_pre_reward_battle_scene(runtime_shell, &snapshot);
            format!(
                "claimed wild rewards {:?} checksum={:?}",
                rewards.outcome, rewards.state_checksum
            )
        }
        crate::RuntimeBattleKind::StaticWild { source_script, .. } => {
            let rewards = runtime_shell.shell.claim_active_wild_battle_rewards()?;
            set_shell_action_status(
                runtime_shell,
                visible_battle_reward_status(&rewards.outcome),
            );
            push_visible_battle_reward_events(
                runtime_shell,
                &rewards.outcome,
                &reward_recipient_name,
            )?;
            queue_visible_pay_day_payout(runtime_shell, &snapshot);
            retain_visible_pre_reward_battle_scene(runtime_shell, &snapshot);
            let message = format!(
                "claimed wild rewards {:?} checksum={:?}",
                rewards.outcome, rewards.state_checksum
            );
            runtime_shell.last_audio_events.push(message);
            finish_visible_wild_battle_exit(
                runtime_shell,
                Some((map_name, source_script)),
                "wild_battle_victory",
            )?;
            return Ok(());
        }
        crate::RuntimeBattleKind::Trainer { .. } => {
            let rewards = runtime_shell.shell.claim_active_trainer_battle_rewards()?;
            set_shell_action_status(
                runtime_shell,
                visible_battle_reward_status(&rewards.outcome),
            );
            push_visible_battle_reward_events(
                runtime_shell,
                &rewards.outcome,
                &reward_recipient_name,
            )?;
            retain_visible_pre_reward_battle_scene(runtime_shell, &snapshot);
            let message = format!(
                "claimed trainer rewards {:?} checksum={:?}",
                rewards.outcome, rewards.state_checksum
            );
            let latest = runtime_shell.shell.snapshot()?;
            if let Some(battle) = latest.battle.as_ref() {
                if should_offer_trainer_shift_switch(&latest, battle) {
                    runtime_shell.battle_action_cursor = None;
                    runtime_shell.battle_move_cursor = None;
                    runtime_shell.battle_move_swap_origin = None;
                    reset_visible_battle_item_cursors(runtime_shell);
                    visible_cursor_index(
                        &mut runtime_shell.battle_shift_prompt_cursor,
                        "battle:shift-prompt",
                        2,
                    );
                    runtime_shell.battle_switch_cursor = None;
                    let next_enemy = next_unresolved_trainer_enemy_label(battle)
                        .context("trainer Shift prompt requires the next enemy party member")?;
                    runtime_shell
                        .battle_messages
                        .push_back(format!("Enemy is about to use {next_enemy}."));
                    runtime_shell.last_audio_events.push(format!(
                        "trainer shift switch prompt choices={}",
                        battle.commands.switch_party_indices.len()
                    ));
                    set_shell_action_status(runtime_shell, "SHIFT: SWITCH POKEMON?");
                }
            }
            message
        }
    };
    runtime_shell.last_audio_events.push(message);
    trim_event_log(&mut runtime_shell.last_audio_events);
    if trainer_battle {
        if runtime_shell.battle_shift_prompt_cursor.is_none()
            && runtime_shell.battle_switch_cursor.is_none()
        {
            reset_visible_battle_action_cursors(runtime_shell);
        }
        return Ok(());
    }
    if plain_wild_battle {
        finish_visible_wild_battle_exit(runtime_shell, None, "wild_battle_victory")?;
    } else {
        reset_visible_battle_exit_state(runtime_shell);
        queue_visible_current_music(runtime_shell)?;
    }
    Ok(())
}

fn retain_visible_pre_reward_battle_scene(
    runtime_shell: &mut BevyRuntimeShell,
    battle_before_rewards: &RuntimeShellSnapshot,
) {
    // Reward authority commits EXP, levels, moves, and evolution atomically,
    // but Crystal does not reveal the resulting battler before narrating it.
    // Keep the faint/reward boundary frame until every queued message clears.
    runtime_shell.battle_message_scene = Some(Box::new(battle_before_rewards.clone()));
    mark_runtime_snapshot_dirty(runtime_shell);
}

fn visible_battle_reward_status(
    outcome: &crate::core::systems::battle_rewards::BattleRewardOutcome,
) -> String {
    let mut parts = vec![format!(
        "{} EXP {}",
        outcome.defeated_species, outcome.experience_awarded
    )];
    if outcome.level_after > outcome.level_before {
        parts.push(format!("LEVEL {}", outcome.level_after));
    }
    if !outcome.learned_moves.is_empty() {
        parts.push(format!("LEARNED {}", outcome.learned_moves.join(",")));
    }
    if !outcome.pending_move_learns.is_empty() {
        let pending = outcome
            .pending_move_learns
            .iter()
            .map(|learned| learned.name.as_str())
            .collect::<Vec<_>>()
            .join(",");
        parts.push(format!("WANTS {pending}"));
    }
    if let Some(target_species) = outcome.evolution.target_species.as_ref() {
        parts.push(format!("EVOLVED {target_species}"));
    }
    compact_scene_label(&parts.join(" / "), 76)
}

fn push_visible_battle_reward_events(
    runtime_shell: &mut BevyRuntimeShell,
    outcome: &crate::core::systems::battle_rewards::BattleRewardOutcome,
    recipient_name: &str,
) -> Result<()> {
    if !outcome.recipient_outcomes.is_empty() {
        for recipient in &outcome.recipient_outcomes {
            let projected = crate::core::systems::battle_rewards::BattleRewardOutcome {
                defeated_species: outcome.defeated_species.clone(),
                experience_awarded: recipient.experience_awarded,
                level_before: recipient.level_before,
                level_after: recipient.level_after,
                learned_moves: recipient.learned_moves.clone(),
                pending_move_learns: recipient.pending_move_learns.clone(),
                deferred_level_evolution: false,
                evolution: recipient.evolution.clone(),
                recipient_outcomes: Vec::new(),
            };
            push_visible_battle_reward_events(
                runtime_shell,
                &projected,
                &recipient.nickname,
            )?;
        }
        return Ok(());
    }
    let snapshot = runtime_shell.shell.snapshot()?;
    if outcome.experience_awarded > 0 {
        runtime_shell.battle_messages.push_back(format!(
            "{} gained {} EXP!",
            recipient_name, outcome.experience_awarded
        ));
    }
    runtime_shell.last_audio_events.push(format!(
        "battle reward exp defeated={} exp={}",
        outcome.defeated_species, outcome.experience_awarded
    ));
    if outcome.level_after > outcome.level_before {
        for level in outcome.level_before.saturating_add(1)..=outcome.level_after {
            runtime_shell.battle_messages.push_back(format!(
                "{} grew to level {}!",
                recipient_name, level
            ));
        }
        runtime_shell.last_audio_events.push(format!(
            "battle reward level {}->{}",
            outcome.level_before, outcome.level_after
        ));
    }
    for move_id in &outcome.learned_moves {
        let move_name = battle_move_display_name(&snapshot, move_id);
        runtime_shell
            .battle_messages
            .push_back(format!("{} learned {}!", recipient_name, move_name));
        runtime_shell
            .last_audio_events
            .push(format!("battle reward learned move {move_id}"));
    }
    for learned in &outcome.pending_move_learns {
        let move_name = battle_move_display_name(&snapshot, &learned.name);
        runtime_shell.battle_messages.push_back(format!(
            "{} is trying to learn {}.",
            recipient_name, move_name
        ));
        runtime_shell
            .last_audio_events
            .push(format!("battle reward pending move learn {}", learned.name));
    }
    if let Some(target_species) = outcome.evolution.target_species.as_ref() {
        let species_name = crate::core::models::pokemon_species_display_name(target_species);
        runtime_shell
            .battle_messages
            .push_back(format!("What? {} is evolving!", recipient_name));
        runtime_shell.battle_messages.push_back(format!(
            "Congratulations! {} evolved into {}!",
            recipient_name, species_name
        ));
        runtime_shell
            .last_audio_events
            .push(format!("battle reward evolved {target_species}"));
        queue_visible_pokemon_cry(runtime_shell, target_species, "battle_reward_evolution")?;
    }
    for event in &outcome.evolution.events {
        let label = match event {
            crate::core::systems::evolution::EvolutionEvent::Text(text) => {
                format!("battle reward evolution text {text}")
            }
            crate::core::systems::evolution::EvolutionEvent::ItemConsumed(item_id) => {
                format!("battle reward evolution consumed {item_id}")
            }
            crate::core::systems::evolution::EvolutionEvent::MoveLearned(move_id) => {
                format!("battle reward evolution learned move {move_id}")
            }
        };
        runtime_shell.last_audio_events.push(label);
    }
    Ok(())
}

fn advance_visible_trainer_battle(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(battle) = snapshot.battle.clone() else {
        return handle_visible_no_active_battle(runtime_shell, "advance_trainer");
    };
    let crate::RuntimeBattleKind::Trainer {
        source_script,
        trainer_name,
        win_text,
        ..
    } = battle.kind
    else {
        anyhow::bail!("active battle is not a trainer battle");
    };
    let map_name = snapshot.overworld.map_name.clone();
    record_visible_runtime_action(
        runtime_shell,
        format!("battle:advance_trainer:{map_name}:{source_script}"),
    )?;
    let advance = runtime_shell.shell.advance_active_trainer_battle()?;
    runtime_shell.last_audio_events.push(format!(
        "advanced trainer battle defeated={} next_enemy={:?} checksum={:?}",
        advance.trainer_defeated, advance.next_enemy, advance.state_checksum
    ));
    set_shell_action_status(
        runtime_shell,
        format!(
            "TRAINER BATTLE defeated={} next={:?}",
            advance.trainer_defeated, advance.next_enemy
        ),
    );
    trim_event_log(&mut runtime_shell.last_audio_events);
    if advance.trainer_defeated {
        queue_visible_trainer_result_text(runtime_shell, &snapshot, &win_text)?;
        reset_visible_battle_exit_state(runtime_shell);
        complete_visible_scripted_trainer_battle(
            runtime_shell,
            &map_name,
            &source_script,
            true,
            false,
        )?;
    } else {
        reset_visible_battle_action_cursors(runtime_shell);
        let replacement = runtime_shell.shell.snapshot()?;
        let replacement_battle = replacement
            .battle
            .as_ref()
            .context("trainer replacement advance removed the active battle")?;
        let send_out_message = format!(
            "{} sent out {}!",
            trainer_name, replacement_battle.enemy_pokemon.nickname
        );
        runtime_shell.battle_messages.push_back(send_out_message.clone());
        runtime_shell.battle_enemy_send_out_pending = true;
        defer_visible_battle_cry_after_message(
            runtime_shell,
            replacement_battle.enemy_pokemon.species.id.clone(),
            "trainer_replacement",
            send_out_message,
        );
        runtime_shell.battle_message_scene = Some(Box::new(replacement));
        mark_runtime_snapshot_dirty(runtime_shell);
    }
    Ok(())
}

fn queue_visible_trainer_result_text(
    runtime_shell: &mut BevyRuntimeShell,
    snapshot: &RuntimeShellSnapshot,
    text_label: &str,
) -> Result<()> {
    if runtime_shell.battle_message_scene.is_none() {
        runtime_shell.battle_message_scene = Some(Box::new(snapshot.clone()));
    }
    let text = runtime_shell.shell.text_snapshot(text_label)?;
    let pages = if let Some(asm_text) = text.asm_text.as_deref() {
        vec![normalize_visible_script_text_with_context(
            asm_text,
            &snapshot.trainer.player_name,
            visible_rival_name(snapshot),
            snapshot.progression.time.day_of_week,
        )]
    } else if let Some(body) = text.body.as_ref() {
        render_visible_script_text_pages(
            body,
            &snapshot.script_events.named_buffers,
            &snapshot.trainer.player_name,
            visible_rival_name(snapshot),
            snapshot.progression.time.day_of_week,
        )
    } else {
        anyhow::bail!("trainer result text '{text_label}' has no renderable body");
    };
    runtime_shell
        .battle_messages
        .extend(pages.into_iter().filter(|page| !page.trim().is_empty()));
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn confirm_visible_shop_top_menu(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let selected = strict_readonly_cursor_index(
        &runtime_shell.shop_top_cursor,
        "shop:top",
        3,
    )
    .context("shop top menu requires a valid cursor")?;
    match selected {
        0 => {
            let snapshot = runtime_shell.shell.snapshot()?;
            let shop = snapshot.pending_shop.as_ref().context("shop top menu has no shop")?;
            runtime_shell.shop_top_cursor = None;
            runtime_shell.sell_cursor = None;
            visible_cursor_index(
                &mut runtime_shell.menu_cursor,
                &shop_cursor_surface_id(shop),
                shop.inventory.len(),
            );
            Ok(())
        }
        1 => {
            let snapshot = runtime_shell.shell.snapshot()?;
            let sellable = sellable_carried_item_ids(&snapshot);
            if sellable.is_empty() {
                let notice = "You don't have anything to sell.".to_string();
                set_shell_action_status(runtime_shell, notice.clone());
                runtime_shell.shop_notice = Some(notice);
                runtime_shell.shop_return_to_top_after_notice = true;
                mark_runtime_snapshot_dirty(runtime_shell);
                return Ok(());
            }
            runtime_shell.shop_top_cursor = None;
            visible_cursor_index(&mut runtime_shell.sell_cursor, "sell:bag", sellable.len());
            Ok(())
        }
        _ => close_visible_shop(runtime_shell),
    }
}

fn buy_visible_shop_cursor_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let Some(shop) = snapshot.pending_shop else {
        return handle_visible_no_active_shop(runtime_shell, "buy_confirm");
    };
    if shop.inventory.is_empty() {
        anyhow::bail!("shop {} has no compiled inventory", shop.mart_id);
    }
    let surface_id = shop_cursor_surface_id(&shop);
    let selected_index = visible_cursor_index(
        &mut runtime_shell.menu_cursor,
        &surface_id,
        shop.inventory.len(),
    );
    begin_visible_shop_quantity(runtime_shell, &shop, selected_index, false)
}

fn begin_visible_shop_quantity(
    runtime_shell: &mut BevyRuntimeShell,
    shop: &crate::core::state::ScriptShopRequest,
    selected_index: usize,
    selling: bool,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let item_id = if selling {
        sellable_carried_item_ids(&snapshot).get(selected_index).cloned()
    } else {
        shop.inventory.get(selected_index).cloned()
    }
    .context("shop quantity selection has no item")?;
    let item = snapshot.items.iter().find(|item| item.item_id == item_id)
        .with_context(|| format!("shop quantity item {item_id} is missing from catalog"))?;
    let unit_price = if selling { item.price / 2 } else { item.price };
    let max_quantity = if selling {
        carried_item_quantity(&snapshot, &item_id).unwrap_or(0).min(99)
    } else if unit_price == 0 {
        0
    } else {
        let owned = carried_item_quantity(&snapshot, &item_id).unwrap_or(0).min(99);
        let stack_limit: u16 = match item.pocket.as_str() {
            "KEY_ITEM" | "TM_HM" => 1,
            _ => 99,
        };
        let pocket_has_room = owned > 0
            || match item.pocket.as_str() {
                "ITEM" => snapshot.bag.items.iter().filter(|entry| entry.quantity > 0).count() < 20,
                "BALL" => snapshot.bag.balls.iter().filter(|entry| entry.quantity > 0).count() < 12,
                "KEY_ITEM" => snapshot.bag.key_items.iter().filter(|entry| entry.quantity > 0).count() < 25,
                "TM_HM" => true,
                _ => true,
            };
        let capacity = if pocket_has_room {
            stack_limit.saturating_sub(owned)
        } else {
            0
        };
        capacity
            .min((snapshot.trainer.money / u32::from(unit_price)).min(99) as u16)
    };
    if max_quantity == 0 {
        let notice = if selling { "You don't have any left." } else { "You can't buy any." };
        set_shell_action_status(runtime_shell, notice);
        runtime_shell.shop_notice = Some(notice.to_string());
        runtime_shell.shop_return_to_top_after_notice = true;
        mark_runtime_snapshot_dirty(runtime_shell);
        return Ok(());
    }
    runtime_shell.shop_quantity = Some(VisibleShopQuantity {
        item_id,
        selling,
        quantity: 1,
        max_quantity,
        unit_price,
    });
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn adjust_visible_shop_quantity(runtime_shell: &mut BevyRuntimeShell, delta: i16) -> Result<()> {
    let Some(quantity) = runtime_shell.shop_quantity.as_mut() else {
        return Ok(());
    };
    quantity.quantity = (i32::from(quantity.quantity) + i32::from(delta))
        .clamp(1, i32::from(quantity.max_quantity)) as u16;
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn confirm_visible_shop_quantity(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let quantity = runtime_shell.shop_quantity.take().context("shop quantity prompt is not open")?;
    let snapshot = runtime_shell.shell.snapshot()?;
    let shop = snapshot
        .pending_shop
        .clone()
        .context("shop quantity prompt has no active shop")?;
    if quantity.selling {
        let sellable = sellable_carried_item_ids(&snapshot);
        let selected_index = sellable.iter().position(|item| item == &quantity.item_id)
            .context("selected sell item disappeared during quantity prompt")?;
        sell_visible_shop_item_from_list(runtime_shell, &sellable, selected_index, quantity.quantity)
    } else {
        let selected_index = shop.inventory.iter().position(|item| item == &quantity.item_id)
            .context("selected buy item disappeared during quantity prompt")?;
        buy_visible_shop_item_from_snapshot(runtime_shell, &shop, selected_index, quantity.quantity)
    }
}

fn buy_visible_shop_item_from_snapshot(
    runtime_shell: &mut BevyRuntimeShell,
    shop: &crate::core::state::ScriptShopRequest,
    selected_index: usize,
    quantity: u16,
) -> Result<()> {
    if shop.inventory.is_empty() {
        anyhow::bail!("shop {} has no compiled inventory", shop.mart_id);
    }
    let item_id = shop
        .inventory
        .get(selected_index)
        .cloned()
        .with_context(|| {
            format!(
                "shop {} selected item index {} outside compiled inventory length {}",
                shop.mart_id,
                selected_index,
                shop.inventory.len()
            )
        })?;
    record_visible_runtime_action(
        runtime_shell,
        format!("shop:buy:{}:{}:{quantity}", shop.mart_id, item_id),
    )?;
    let transaction = runtime_shell.shell.buy_shop_item(&item_id, quantity)?;
    runtime_shell.last_audio_events.push(format!(
        "shop buy {}/{} item={} outcome={:?} checksum={:?}",
        selected_index + 1,
        shop.inventory.len(),
        item_id,
        transaction.outcome,
        transaction.state_checksum
    ));
    let notice = visible_shop_transaction_status("BOUGHT", &item_id, &transaction.outcome);
    set_shell_action_status(runtime_shell, notice.clone());
    runtime_shell.shop_notice = Some(notice);
    runtime_shell.shop_return_to_top_after_notice = true;
    let snapshot = runtime_shell.shell.snapshot()?;
    if let Some(shop) = snapshot.pending_shop.as_ref() {
        if shop.inventory.is_empty() {
            runtime_shell.menu_cursor = None;
        } else {
            runtime_shell.menu_cursor = Some(MenuCursor {
                surface_id: shop_cursor_surface_id(shop),
                option_index: selected_index.min(shop.inventory.len().saturating_sub(1)),
            });
        }
    }
    Ok(())
}

fn shop_cursor_surface_id(shop: &crate::core::state::ScriptShopRequest) -> String {
    format!(
        "shop:{}:{}:{}",
        shop.source_script, shop.command_index, shop.mart_id
    )
}

fn sell_selected_bag_item(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.pending_shop.is_none() {
        return handle_visible_no_active_shop(runtime_shell, "sell_confirm");
    }
    let sellable = sellable_carried_item_ids(&snapshot);
    if sellable.is_empty() {
        runtime_shell.sell_cursor = None;
        record_visible_runtime_action(runtime_shell, "shop:sell:confirm:no_items")?;
        runtime_shell
            .last_audio_events
            .push("bag has no sellable carried item".to_string());
        let notice = "You don't have anything to sell.".to_string();
        set_shell_action_status(runtime_shell, notice.clone());
        runtime_shell.shop_notice = Some(notice);
        runtime_shell.shop_return_to_top_after_notice = true;
        mark_runtime_snapshot_dirty(runtime_shell);
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    let selected_index =
        visible_cursor_index(&mut runtime_shell.sell_cursor, "sell:bag", sellable.len());
    begin_visible_shop_quantity(
        runtime_shell,
        snapshot.pending_shop.as_ref().context("no active shop")?,
        selected_index,
        true,
    )
}

fn sell_visible_shop_item_from_list(
    runtime_shell: &mut BevyRuntimeShell,
    sellable: &[String],
    selected_index: usize,
    quantity: u16,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let mart_id = snapshot
        .pending_shop
        .as_ref()
        .map(|shop| shop.mart_id.clone())
        .context("no active shop")?;
    let item_id = sellable.get(selected_index).cloned().with_context(|| {
        format!(
            "sell cursor selected item index {} outside sellable item count {}",
            selected_index,
            sellable.len()
        )
    })?;
    record_visible_runtime_action(runtime_shell, format!("shop:sell:{mart_id}:{item_id}:{quantity}"))?;
    let transaction = runtime_shell.shell.sell_shop_item(&item_id, quantity)?;
    runtime_shell.last_audio_events.push(format!(
        "shop sell {}/{} item={} outcome={:?} checksum={:?}",
        selected_index + 1,
        sellable.len(),
        item_id,
        transaction.outcome,
        transaction.state_checksum
    ));
    let notice = visible_shop_transaction_status("SOLD", &item_id, &transaction.outcome);
    set_shell_action_status(runtime_shell, notice.clone());
    runtime_shell.shop_notice = Some(notice);
    runtime_shell.shop_return_to_top_after_notice = true;
    let snapshot = runtime_shell.shell.snapshot()?;
    let sellable = sellable_carried_item_ids(&snapshot);
    if sellable.is_empty() {
        runtime_shell.sell_cursor = None;
    } else {
        runtime_shell.sell_cursor = Some(MenuCursor {
            surface_id: "sell:bag".to_string(),
            option_index: selected_index.min(sellable.len().saturating_sub(1)),
        });
    }
    Ok(())
}

fn close_visible_shop(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    runtime_shell.shop_notice = Some("Please come again!".to_string());
    runtime_shell.shop_return_to_top_after_notice = false;
    runtime_shell.shop_close_after_notice = true;
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn finalize_visible_shop_close(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    runtime_shell.shop_quantity = None;
    runtime_shell.shop_notice = None;
    runtime_shell.shop_welcome_seen = false;
    runtime_shell.shop_return_to_top_after_notice = false;
    runtime_shell.shop_close_after_notice = false;
    runtime_shell.shop_top_cursor = Some(MenuCursor {
        surface_id: "shop:top".to_string(),
        option_index: 0,
    });
    let close = runtime_shell.shell.close_script_shop()?;
    reset_visible_selection_cursors(runtime_shell);
    runtime_shell.last_audio_events.push(format!(
        "shop closed mart={} checksum={:?}",
        close.shop.mart_id, close.state_checksum
    ));
    set_shell_action_status(runtime_shell, format!("CLOSED SHOP {}", close.shop.mart_id));
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn dismiss_visible_shop_notice(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    runtime_shell.shop_notice = None;
    if runtime_shell.shop_close_after_notice {
        return finalize_visible_shop_close(runtime_shell);
    }
    if runtime_shell.shop_return_to_top_after_notice {
        runtime_shell.shop_return_to_top_after_notice = false;
        runtime_shell.menu_cursor = None;
        runtime_shell.sell_cursor = None;
        runtime_shell.shop_quantity = None;
        runtime_shell.shop_top_cursor = Some(MenuCursor {
            surface_id: "shop:top".to_string(),
            option_index: 0,
        });
        runtime_shell.shop_notice = Some("Can I do anything\nelse for you?".to_string());
    }
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn visible_shop_transaction_status(
    verb: &str,
    _item_id: &str,
    outcome: &crate::core::systems::shop::ShopResult,
) -> String {
    if outcome.success {
        match verb {
            "BOUGHT" => "Here you are.\nThank you!".to_string(),
            "SOLD" => format!("Sold for {}!", outcome.message),
            _ => outcome.message.clone(),
        }
    } else {
        outcome.message.clone()
    }
}

fn close_visible_pc_surface(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "pc:submenu:close")?;
    runtime_shell.storage_cursor = None;
    runtime_shell.pc_item_cursor = None;
    runtime_shell.pc_item_action = None;
    runtime_shell.pc_item_quantity = None;
    runtime_shell.player_pc_action_cursor = None;
    runtime_shell.mailbox_cursor = None;
    runtime_shell.mailbox_action_cursor = None;
    runtime_shell.mailbox_attach_index = None;
    runtime_shell.pc_confirmation = None;
    runtime_shell.bill_pc_move_open = false;
    runtime_shell.bill_pc_move_source = None;
    runtime_shell.bill_pc_pokemon_action_cursor = None;
    runtime_shell.bill_pc_box_summary = None;
    runtime_shell.pending_pc_release = None;
    runtime_shell.pc_notice = None;
    runtime_shell.bill_pc_box_cursor = None;
    if runtime_shell.bill_pc_session_open {
        runtime_shell.party_menu_open = false;
        runtime_shell.bill_pc_action_cursor = Some(MenuCursor {
            surface_id: "pc:bill-actions".to_string(),
            option_index: 0,
        });
        set_shell_action_status(runtime_shell, "BILL'S PC");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    if runtime_shell.pc_hub_session_open {
        runtime_shell.pc_hub_cursor = Some(MenuCursor {
            surface_id: "pc:hub".to_string(),
            option_index: 0,
        });
        set_shell_action_status(runtime_shell, "ACCESS WHOSE PC?");
        trim_event_log(&mut runtime_shell.last_audio_events);
        return Ok(());
    }
    runtime_shell
        .last_audio_events
        .push("closed PC surface".to_string());
    set_shell_action_status(runtime_shell, "CLOSED PC");
    trim_event_log(&mut runtime_shell.last_audio_events);
    continue_visible_script_after_prompt(runtime_shell)?;
    Ok(())
}

fn visible_pc_hub_actions(snapshot: &RuntimeShellSnapshot) -> Vec<VisiblePcHubAction> {
    let mut actions = vec![VisiblePcHubAction::BillsPc, VisiblePcHubAction::PlayerPc];
    if snapshot.progression.active_engine_flags.contains("ENGINE_POKEDEX") {
        actions.push(VisiblePcHubAction::OakPc);
    }
    if snapshot.progression.hall_of_fame.count > 0
        || !snapshot.progression.hall_of_fame.entries.is_empty()
    {
        if !actions.contains(&VisiblePcHubAction::OakPc) {
            actions.push(VisiblePcHubAction::OakPc);
        }
        actions.push(VisiblePcHubAction::HallOfFame);
    }
    actions.push(VisiblePcHubAction::TurnOff);
    actions
}

fn visible_pc_hub_action_label(
    snapshot: &RuntimeShellSnapshot,
    action: VisiblePcHubAction,
) -> String {
    match action {
        VisiblePcHubAction::BillsPc => "BILL'S PC".to_string(),
        VisiblePcHubAction::PlayerPc => format!("{}'S PC", snapshot.trainer.player_name),
        VisiblePcHubAction::OakPc => "PROF.OAK'S PC".to_string(),
        VisiblePcHubAction::HallOfFame => "HALL OF FAME".to_string(),
        VisiblePcHubAction::TurnOff => "TURN OFF".to_string(),
    }
}

fn visible_bill_pc_action_label(action: VisibleBillPcAction) -> &'static str {
    match action {
        VisibleBillPcAction::Withdraw => "WITHDRAW POKEMON",
        VisibleBillPcAction::Deposit => "DEPOSIT POKEMON",
        VisibleBillPcAction::ChangeBox => "CHANGE BOX",
        VisibleBillPcAction::MoveWithoutMail => "MOVE PKMN W/O MAIL",
        VisibleBillPcAction::SeeYa => "SEE YA!",
    }
}

fn confirm_visible_bill_pc_action(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let selected = strict_readonly_cursor_index(
        &runtime_shell.bill_pc_action_cursor,
        "pc:bill-actions",
        VISIBLE_BILL_PC_ACTIONS.len(),
    )
    .context("Bill's PC action menu is open without a valid cursor")?;
    let action = VISIBLE_BILL_PC_ACTIONS[selected];
    record_visible_runtime_action(
        runtime_shell,
        format!("pc:bill:{}", visible_bill_pc_action_label(action)),
    )?;
    runtime_shell.bill_pc_action_cursor = None;
    match action {
        VisibleBillPcAction::Withdraw => {
            let snapshot = runtime_shell.shell.snapshot()?;
            runtime_shell.storage_cursor = Some(MenuCursor {
                surface_id: storage_cursor_surface_id(snapshot.storage.current_pc_box),
                option_index: 0,
            });
            runtime_shell.party_menu_open = false;
            set_shell_action_status(runtime_shell, "WITHDRAW POKEMON");
        }
        VisibleBillPcAction::Deposit => {
            let snapshot = runtime_shell.shell.snapshot()?;
            runtime_shell.storage_cursor = Some(MenuCursor {
                surface_id: storage_cursor_surface_id(snapshot.storage.current_pc_box),
                option_index: 0,
            });
            open_visible_party_menu(runtime_shell)?;
            set_shell_action_status(runtime_shell, "DEPOSIT POKEMON");
        }
        VisibleBillPcAction::ChangeBox => {
            let snapshot = runtime_shell.shell.snapshot()?;
            runtime_shell.bill_pc_box_cursor = Some(MenuCursor {
                surface_id: "pc:bill-boxes".to_string(),
                option_index: snapshot.storage.current_pc_box,
            });
            set_shell_action_status(runtime_shell, "CHOOSE A BOX");
        }
        VisibleBillPcAction::MoveWithoutMail => {
            let snapshot = runtime_shell.shell.snapshot()?;
            runtime_shell.bill_pc_move_open = true;
            runtime_shell.bill_pc_move_source = None;
            runtime_shell.storage_cursor = Some(MenuCursor {
                surface_id: storage_cursor_surface_id(snapshot.storage.current_pc_box),
                option_index: 0,
            });
            set_shell_action_status(runtime_shell, "CHOOSE A POKEMON TO MOVE");
        }
        VisibleBillPcAction::SeeYa => return close_visible_bill_pc_actions(runtime_shell),
    }
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn confirm_visible_bill_pc_box(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let selected = strict_readonly_cursor_index(
        &runtime_shell.bill_pc_box_cursor,
        "pc:bill-boxes",
        crate::core::models::MAX_PC_BOXES,
    )
    .context("Bill's PC box menu is open without a valid cursor")?;
    let switched = runtime_shell.shell.switch_current_pc_box(selected)?;
    runtime_shell.bill_pc_box_cursor = None;
    runtime_shell.bill_pc_action_cursor = Some(MenuCursor {
        surface_id: "pc:bill-actions".to_string(),
        option_index: 2,
    });
    mark_runtime_snapshot_dirty(runtime_shell);
    set_shell_action_status(
        runtime_shell,
        format!("BOX {} SELECTED", switched.box_index_after + 1),
    );
    Ok(())
}

fn confirm_visible_bill_pc_move(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let box_index = snapshot.storage.current_pc_box;
    let slot = selected_current_box_slot_index(runtime_shell)?;
    let Some((source_box, source_slot)) = runtime_shell.bill_pc_move_source else {
        let occupied = current_storage_box(&snapshot)?
            .slots
            .iter()
            .any(|candidate| candidate.index == slot);
        if !occupied {
            set_shell_action_status(runtime_shell, "NO POKEMON THERE");
            return Ok(());
        }
        runtime_shell.bill_pc_move_source = Some((box_index, slot));
        set_shell_action_status(runtime_shell, "CHOOSE A DESTINATION");
        return Ok(());
    };
    if source_box == box_index && source_slot == slot {
        runtime_shell.bill_pc_move_source = None;
        set_shell_action_status(runtime_shell, "MOVE CANCELLED");
        return Ok(());
    }
    let moved = runtime_shell.shell.move_pc_pokemon_without_mail(
        source_box,
        source_slot,
        box_index,
        slot,
    )?;
    runtime_shell.bill_pc_move_source = None;
    runtime_shell.storage_cursor = Some(MenuCursor {
        surface_id: storage_cursor_surface_id(moved.target_box),
        option_index: 0,
    });
    mark_runtime_snapshot_dirty(runtime_shell);
    set_shell_action_status(
        runtime_shell,
        if moved.swapped {
            "POKEMON SWAPPED"
        } else {
            "POKEMON MOVED"
        },
    );
    Ok(())
}

fn close_visible_bill_pc_actions(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "pc:bill:see_ya")?;
    runtime_shell.bill_pc_session_open = false;
    runtime_shell.bill_pc_action_cursor = None;
    runtime_shell.bill_pc_box_cursor = None;
    runtime_shell.bill_pc_move_open = false;
    runtime_shell.bill_pc_move_source = None;
    runtime_shell.storage_cursor = None;
    runtime_shell.party_menu_open = false;
    runtime_shell.pc_hub_cursor = Some(MenuCursor {
        surface_id: "pc:hub".to_string(),
        option_index: 0,
    });
    set_shell_action_status(runtime_shell, "ACCESS WHOSE PC?");
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn confirm_visible_pc_hub(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let actions = visible_pc_hub_actions(&snapshot);
    let selected = strict_readonly_cursor_index(&runtime_shell.pc_hub_cursor, "pc:hub", actions.len())
        .context("Pokemon Center PC hub is open without a valid cursor")?;
    let action = actions[selected];
    record_visible_runtime_action(
        runtime_shell,
        format!("pc:hub:{}", visible_pc_hub_action_label(&snapshot, action)),
    )?;
    runtime_shell.pc_hub_cursor = None;
    match action {
        VisiblePcHubAction::BillsPc => {
            runtime_shell.bill_pc_session_open = true;
            runtime_shell.bill_pc_box_cursor = None;
            runtime_shell.bill_pc_action_cursor = Some(MenuCursor {
                surface_id: "pc:bill-actions".to_string(),
                option_index: 0,
            });
            set_shell_action_status(runtime_shell, "BILL'S PC");
        }
        VisiblePcHubAction::PlayerPc => {
            runtime_shell.bill_pc_session_open = false;
            runtime_shell.bill_pc_action_cursor = None;
            runtime_shell.bill_pc_box_cursor = None;
            runtime_shell.player_pc_action_cursor = Some(MenuCursor {
                surface_id: "pc:player-actions".to_string(),
                option_index: 0,
            });
            set_shell_action_status(runtime_shell, "PLAYER'S PC");
        }
        VisiblePcHubAction::OakPc => {
            let oak = runtime_shell.shell.show_prof_oaks_pc_boot()?;
            let SpecialRoutineEffect::ProfOaksPcBoot {
                seen_count,
                caught_count,
                rating_label,
            } = &oak.outcome.effect
            else {
                anyhow::bail!("Prof. Oak PC returned the wrong special effect");
            };
            open_visible_prof_oak_rating(
                runtime_shell,
                *seen_count,
                *caught_count,
                rating_label,
            )?;
        }
        VisiblePcHubAction::HallOfFame => {
            let mut details = vec!["HALL OF FAME".to_string()];
            if let Some(record) = snapshot.progression.hall_of_fame.entries.last() {
                details.push(format!("VICTORY {}", record.win_count));
                details.extend(record.team.iter().flatten().map(|pokemon| {
                    format!("{} L{}", pokemon.nickname, pokemon.level)
                }));
            } else {
                details.push("NO RECORD".to_string());
            }
            runtime_shell.special_boundary = Some(SpecialBoundaryDisplay {
                label: "HallOfFamePC".to_string(),
                details,
            });
            set_shell_action_status(runtime_shell, "HALL OF FAME");
        }
        VisiblePcHubAction::TurnOff => return turn_off_visible_pc_hub(runtime_shell),
    }
    trim_event_log(&mut runtime_shell.last_audio_events);
    Ok(())
}

fn open_visible_prof_oak_rating(
    runtime_shell: &mut BevyRuntimeShell,
    seen_count: usize,
    caught_count: usize,
    rating_label: &str,
) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let rating = snapshot
        .presentation
        .asm_text
        .get(rating_label)
        .with_context(|| format!("Prof. Oak rating text {rating_label} is missing"))?;
    let rating = normalize_visible_script_text_with_context(
        rating,
        &snapshot.trainer.player_name,
        visible_rival_name(&snapshot),
        snapshot.progression.time.day_of_week,
    );
    runtime_shell.special_boundary = Some(SpecialBoundaryDisplay {
        label: "ProfOaksPcBoot".to_string(),
        details: vec![
            "PROF.OAK'S PC".to_string(),
            format!("SEEN {seen_count}  OWN {caught_count}"),
            rating,
        ],
    });
    set_shell_action_status(runtime_shell, "PROF.OAK'S RATING");
    mark_runtime_snapshot_dirty(runtime_shell);
    Ok(())
}

fn visible_player_pc_action_label(action: VisiblePlayerPcAction) -> &'static str {
    match action {
        VisiblePlayerPcAction::WithdrawItem => "WITHDRAW ITEM",
        VisiblePlayerPcAction::DepositItem => "DEPOSIT ITEM",
        VisiblePlayerPcAction::TossItem => "TOSS ITEM",
        VisiblePlayerPcAction::MailBox => "MAIL BOX",
        VisiblePlayerPcAction::Decoration => "DECORATION",
        VisiblePlayerPcAction::LogOff => "LOG OFF",
        VisiblePlayerPcAction::TurnOff => "TURN OFF",
    }
}

fn visible_player_pc_actions(runtime_shell: &BevyRuntimeShell) -> &'static [VisiblePlayerPcAction] {
    if runtime_shell.pc_hub_session_open {
        &VISIBLE_PLAYER_PC_ACTIONS
    } else {
        &VISIBLE_PLAYERS_HOUSE_PC_ACTIONS
    }
}

fn confirm_visible_player_pc_action(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let actions = visible_player_pc_actions(runtime_shell);
    let selected = strict_readonly_cursor_index(
        &runtime_shell.player_pc_action_cursor,
        "pc:player-actions",
        actions.len(),
    )
    .context("Player PC action menu requires a valid cursor")?;
    let action = actions[selected];
    record_visible_runtime_action(runtime_shell, format!("pc:player:{}", visible_player_pc_action_label(action)))?;
    runtime_shell.player_pc_action_cursor = None;
    match action {
        VisiblePlayerPcAction::WithdrawItem | VisiblePlayerPcAction::TossItem => {
            let snapshot = runtime_shell.shell.snapshot()?;
            if carried_item_count(&snapshot.bag.pc_items) == 0 {
                runtime_shell.pc_notice = Some("No items here!".to_string());
                runtime_shell.player_pc_action_cursor = Some(MenuCursor {
                    surface_id: "pc:player-actions".to_string(), option_index: selected,
                });
            } else {
                runtime_shell.pc_item_action = Some(action);
                runtime_shell.pc_item_cursor = Some(MenuCursor { surface_id: "pc:items".to_string(), option_index: 0 });
            }
        }
        VisiblePlayerPcAction::DepositItem => {
            runtime_shell.pc_item_action = Some(action);
            open_visible_pc_item_deposit_pack(runtime_shell)?;
            if !visible_field_pack_is_open(runtime_shell) {
                runtime_shell.pc_item_action = None;
                runtime_shell.player_pc_action_cursor = Some(MenuCursor {
                    surface_id: "pc:player-actions".to_string(), option_index: selected,
                });
                runtime_shell.pc_notice = Some("No items here!".to_string());
            }
        }
        VisiblePlayerPcAction::MailBox => {
            let snapshot = runtime_shell.shell.snapshot()?;
            if snapshot.mailbox.is_empty() {
                runtime_shell.pc_notice = Some("There's no MAIL here.".to_string());
                runtime_shell.player_pc_action_cursor = Some(MenuCursor {
                    surface_id: "pc:player-actions".to_string(), option_index: selected,
                });
            } else {
                runtime_shell.mailbox_cursor = Some(MenuCursor { surface_id: "pc:mailbox".to_string(), option_index: 0 });
            }
        }
        VisiblePlayerPcAction::Decoration => {
            runtime_shell.pc_notice = Some("DECORATION".to_string());
            runtime_shell.player_pc_action_cursor = Some(MenuCursor {
                surface_id: "pc:player-actions".to_string(),
                option_index: selected,
            });
        }
        VisiblePlayerPcAction::LogOff | VisiblePlayerPcAction::TurnOff => {
            return close_visible_player_pc(runtime_shell)
        }
    }
    Ok(())
}

fn close_visible_player_pc(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    runtime_shell.player_pc_action_cursor = None;
    runtime_shell.pc_item_cursor = None;
    runtime_shell.pc_item_action = None;
    runtime_shell.pc_item_quantity = None;
    runtime_shell.mailbox_cursor = None;
    runtime_shell.mailbox_action_cursor = None;
    runtime_shell.mailbox_attach_index = None;
    if runtime_shell.pc_hub_session_open {
        runtime_shell.pc_hub_cursor = Some(MenuCursor { surface_id: "pc:hub".to_string(), option_index: 0 });
        set_shell_action_status(runtime_shell, "ACCESS WHOSE PC?");
        return Ok(());
    }
    runtime_shell.pc_hub_cursor = None;
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.ui.menu.is_some() {
        let _ = runtime_shell.shell.close_active_menu()?;
    } else if snapshot.ui.window_open {
        let _ = runtime_shell.shell.close_runtime_window()?;
    }
    set_shell_action_status(runtime_shell, "LOGGED OFF");
    continue_visible_script_after_prompt(runtime_shell)
}

fn confirm_visible_mailbox_selection(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    let selected = strict_readonly_cursor_index(&runtime_shell.mailbox_cursor, "pc:mailbox", snapshot.mailbox.len())
        .context("mailbox requires a valid cursor")?;
    runtime_shell.mailbox_action_cursor = Some(MenuCursor {
        surface_id: "pc:mailbox-actions".to_string(), option_index: 0,
    });
    record_visible_runtime_action(runtime_shell, format!("pc:mailbox:select:{selected}"))?;
    Ok(())
}

fn confirm_visible_mailbox_action(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let action = strict_readonly_cursor_index(
        &runtime_shell.mailbox_action_cursor,
        "pc:mailbox-actions",
        VISIBLE_MAILBOX_ACTIONS.len(),
    )
    .context("mailbox action menu requires a valid cursor")?;
    let snapshot = runtime_shell.shell.snapshot()?;
    let mailbox_index = strict_readonly_cursor_index(&runtime_shell.mailbox_cursor, "pc:mailbox", snapshot.mailbox.len())
        .context("mailbox action has no selected message")?;
    let entry = &snapshot.mailbox[mailbox_index];
    match action {
        0 => runtime_shell.pc_notice = Some(format!("{}\n\nFrom {}", entry.mail.message, entry.mail.author)),
        1 => {
            runtime_shell.pc_confirmation = Some(VisiblePcConfirmation::PutMailInPack(mailbox_index));
            runtime_shell.yes_no_cursor = Some(MenuCursor { surface_id: "pc:confirmation".to_string(), option_index: 1 });
            runtime_shell.pc_notice = Some("The MAIL's message will be lost. Is that OK?".to_string());
            return Ok(());
        }
        2 => {
            runtime_shell.mailbox_attach_index = Some(mailbox_index);
            runtime_shell.mailbox_action_cursor = None;
            open_visible_party_menu(runtime_shell)?;
            set_shell_action_status(runtime_shell, "ATTACH MAIL TO WHICH POKEMON?");
            return Ok(());
        }
        _ => {}
    }
    runtime_shell.mailbox_action_cursor = None;
    let latest = runtime_shell.shell.snapshot()?;
    if latest.mailbox.is_empty() {
        runtime_shell.mailbox_cursor = None;
        runtime_shell.player_pc_action_cursor = Some(MenuCursor { surface_id: "pc:player-actions".to_string(), option_index: 3 });
    } else if let Some(cursor) = runtime_shell.mailbox_cursor.as_mut() {
        cursor.option_index = cursor.option_index.min(latest.mailbox.len() - 1);
    }
    Ok(())
}

fn resolve_visible_pc_confirmation(runtime_shell: &mut BevyRuntimeShell, accepted: bool) -> Result<()> {
    let confirmation = runtime_shell.pc_confirmation.take().context("no PC confirmation is active")?;
    runtime_shell.yes_no_cursor = None;
    runtime_shell.pc_notice = None;
    if !accepted
        && !matches!(
            &confirmation,
            VisiblePcConfirmation::NpcTrade(_)
                | VisiblePcConfirmation::ScriptPartyIntro(_)
                | VisiblePcConfirmation::MoveDeletion { .. }
                | VisiblePcConfirmation::MoveTutorForget { .. }
                | VisiblePcConfirmation::MoveTutorStop { .. }
                | VisiblePcConfirmation::DayCareWithdraw { .. }
        )
    {
        return Ok(());
    }
    match confirmation {
        VisiblePcConfirmation::TossItem { item_id, quantity } => {
            record_visible_runtime_action(
                runtime_shell,
                format!("pc:toss_item:{item_id}:{quantity}"),
            )?;
            let transfer = runtime_shell.shell.toss_pc_item(&item_id, quantity)?;
            runtime_shell.pc_notice = Some(format!(
                "Discarded\n{}(S).",
                item_display_name(&runtime_shell.shell.snapshot()?, &transfer.item_id)
            ));
            let snapshot = runtime_shell.shell.snapshot()?;
            let count = carried_item_count(&snapshot.bag.pc_items);
            if count == 0 {
                runtime_shell.pc_item_cursor = None;
                runtime_shell.pc_item_action = None;
                runtime_shell.player_pc_action_cursor = Some(MenuCursor {
                    surface_id: "pc:player-actions".to_string(),
                    option_index: 2,
                });
            } else if let Some(cursor) = runtime_shell.pc_item_cursor.as_mut() {
                cursor.option_index = cursor.option_index.min(count - 1);
            }
        }
        VisiblePcConfirmation::PutMailInPack(mailbox_index) => {
            match runtime_shell.shell.move_mailbox_mail_to_bag(mailbox_index) {
                Ok(_) => runtime_shell.pc_notice = Some("The MAIL was put in the PACK.".to_string()),
                Err(error) if error.to_string().contains("bag") => runtime_shell.pc_notice = Some("The PACK is full.".to_string()),
                Err(error) => return Err(error),
            }
            runtime_shell.mailbox_action_cursor = None;
            let snapshot = runtime_shell.shell.snapshot()?;
            if snapshot.mailbox.is_empty() {
                runtime_shell.mailbox_cursor = None;
                runtime_shell.player_pc_action_cursor = Some(MenuCursor { surface_id: "pc:player-actions".to_string(), option_index: 3 });
            } else if let Some(cursor) = runtime_shell.mailbox_cursor.as_mut() {
                cursor.option_index = cursor.option_index.min(snapshot.mailbox.len() - 1);
            }
        }
        VisiblePcConfirmation::NpcTrade(pending) => {
            let PendingScriptPartySelection::NpcTrade {
                origin_map_name,
                source_script,
                command_index,
                trade_id,
            } = pending
            else {
                anyhow::bail!("NPC trade confirmation contains a non-trade party request");
            };
            if accepted {
                runtime_shell.pending_script_party_selection = Some(
                    PendingScriptPartySelection::NpcTrade {
                        origin_map_name,
                        source_script,
                        command_index,
                        trade_id,
                    },
                );
                open_visible_party_menu(runtime_shell)?;
                set_shell_action_status(runtime_shell, "CHOOSE A POKEMON");
            } else {
                apply_visible_npc_trade_selection(
                    runtime_shell,
                    origin_map_name,
                    source_script,
                    command_index,
                    trade_id,
                    None,
                )?;
            }
        }
        VisiblePcConfirmation::ScriptPartyIntro(pending) => {
            if accepted {
                let status = match &pending {
                    PendingScriptPartySelection::NameRater => "WHICH POKéMON'S NICKNAME?",
                    PendingScriptPartySelection::MoveDeletion { party_index: None } => {
                        "WHICH POKéMON?"
                    }
                    PendingScriptPartySelection::DayCareDeposit { .. } => "WHICH POKéMON?",
                    _ => anyhow::bail!(
                        "script party intro contains an unsupported selection request"
                    ),
                };
                runtime_shell.pending_script_party_selection = Some(pending);
                open_visible_party_menu(runtime_shell)?;
                set_shell_action_status(runtime_shell, status);
            } else {
                let (label, details) = match pending {
                    PendingScriptPartySelection::NameRater => (
                        "NameRaterComeAgainText",
                        vec!["OK, then. Come".to_string(), "again sometime.".to_string()],
                    ),
                    PendingScriptPartySelection::MoveDeletion { party_index: None } => (
                        "DeleterNoComeAgainText",
                        vec!["No? Come visit me".to_string(), "again.".to_string()],
                    ),
                    PendingScriptPartySelection::DayCareDeposit { .. } => (
                        "DayCareOhFineText",
                        vec!["Oh, fine then.".to_string(), "Come again.".to_string()],
                    ),
                    _ => anyhow::bail!(
                        "script party intro contains an unsupported selection request"
                    ),
                };
                runtime_shell.special_boundary = Some(SpecialBoundaryDisplay {
                    label: label.to_string(),
                    details,
                });
            }
            mark_runtime_snapshot_dirty(runtime_shell);
        }
        VisiblePcConfirmation::MoveDeletion {
            party_index,
            move_index,
        } => {
            if accepted {
                record_visible_runtime_action(
                    runtime_shell,
                    format!("script:special:move_deletion:{party_index}:{move_index}"),
                )?;
                let special = runtime_shell
                    .shell
                    .delete_party_move_special(party_index, move_index)?;
                runtime_shell.last_audio_events.push(format!(
                    "move deletion outcome={:?} checksum={:?}",
                    special.outcome.effect, special.state_checksum
                ));
                queue_visible_shell_sound_effect(runtime_shell, "SFX_MOVE_DELETED")?;
                runtime_shell.special_boundary = Some(SpecialBoundaryDisplay {
                    label: "DeleterForgotMoveText".to_string(),
                    details: vec![
                        "Done! Your POKéMON".to_string(),
                        "forgot the move.".to_string(),
                    ],
                });
            } else {
                runtime_shell.special_boundary = Some(SpecialBoundaryDisplay {
                    label: "DeleterNoComeAgainText".to_string(),
                    details: vec![
                        "No? Come visit me".to_string(),
                        "again.".to_string(),
                    ],
                });
            }
            mark_runtime_snapshot_dirty(runtime_shell);
        }
        VisiblePcConfirmation::MoveTutorForget {
            move_id,
            party_index,
        } => {
            if accepted {
                runtime_shell.pending_script_party_selection = Some(
                    PendingScriptPartySelection::MoveTutor {
                        move_id,
                        party_index: Some(party_index),
                    },
                );
                runtime_shell.party_move_cursor = Some(MenuCursor {
                    surface_id: party_move_cursor_surface_id(party_index),
                    option_index: 0,
                });
                set_shell_action_status(runtime_shell, "WHICH MOVE SHOULD BE FORGOTTEN?");
            } else {
                runtime_shell.pc_notice = Some("Stop learning this move?".to_string());
                runtime_shell.pc_confirmation = Some(VisiblePcConfirmation::MoveTutorStop {
                    move_id,
                    party_index,
                });
                runtime_shell.yes_no_cursor = Some(MenuCursor {
                    surface_id: "yes-no".to_string(),
                    option_index: 0,
                });
                set_shell_action_status(runtime_shell, "STOP LEARNING?");
            }
            mark_runtime_snapshot_dirty(runtime_shell);
        }
        VisiblePcConfirmation::MoveTutorStop {
            move_id,
            party_index,
        } => {
            if accepted {
                runtime_shell.party_move_cursor = None;
                runtime_shell.pending_script_party_selection = None;
                set_visible_script_numeric_value(runtime_shell, u8::MAX);
                close_visible_party_menu(runtime_shell);
                let snapshot = runtime_shell.shell.snapshot()?;
                let nickname = snapshot
                    .party
                    .slots
                    .iter()
                    .find(|slot| slot.index == party_index)
                    .map(|slot| {
                        if slot.pokemon.nickname.trim().is_empty() {
                            canonical_species_display_name(&slot.pokemon.species.id)
                        } else {
                            slot.pokemon.nickname.clone()
                        }
                    })
                    .context("Move Tutor party selection is no longer present")?;
                runtime_shell.special_boundary = Some(SpecialBoundaryDisplay {
                    label: "DidNotLearnMoveText".to_string(),
                    details: vec![format!(
                        "{nickname} did not learn\n{}.",
                        move_id.replace('_', " ")
                    )],
                });
            } else {
                let snapshot = runtime_shell.shell.snapshot()?;
                let nickname = snapshot
                    .party
                    .slots
                    .iter()
                    .find(|slot| slot.index == party_index)
                    .map(|slot| {
                        if slot.pokemon.nickname.trim().is_empty() {
                            canonical_species_display_name(&slot.pokemon.species.id)
                        } else {
                            slot.pokemon.nickname.clone()
                        }
                    })
                    .context("Move Tutor party selection is no longer present")?;
                runtime_shell.pc_notice = Some(format!(
                    "{nickname} is trying to learn\n{}.\n\nBut {nickname} can't learn\nmore than four moves.\n\nDelete an older move\nto make room?",
                    move_id.replace('_', " ")
                ));
                runtime_shell.pc_confirmation = Some(
                    VisiblePcConfirmation::MoveTutorForget {
                        move_id,
                        party_index,
                    },
                );
                runtime_shell.yes_no_cursor = Some(MenuCursor {
                    surface_id: "yes-no".to_string(),
                    option_index: 0,
                });
                set_shell_action_status(runtime_shell, "DELETE A MOVE?");
            }
            mark_runtime_snapshot_dirty(runtime_shell);
        }
        VisiblePcConfirmation::DayCareWithdraw { caretaker } => {
            if !accepted {
                runtime_shell.special_boundary = Some(SpecialBoundaryDisplay {
                    label: "DayCareOhFineText".to_string(),
                    details: vec!["Oh, fine then.".to_string(), "Come again.".to_string()],
                });
                mark_runtime_snapshot_dirty(runtime_shell);
                return Ok(());
            }
            let caretaker_kind = if caretaker == "man" {
                RuntimeDayCareCaretaker::Man
            } else {
                RuntimeDayCareCaretaker::Lady
            };
            let used = runtime_shell.shell.use_day_care(
                caretaker_kind,
                RuntimeDayCareAction::Withdraw,
                None,
            )?;
            let snapshot = runtime_shell.shell.snapshot()?;
            let interaction = snapshot.day_care.last_interaction.as_ref().context("Day-Care withdrawal produced no result")?;
            runtime_shell.special_boundary_queue.clear();
            runtime_shell.special_boundary = Some(match interaction.reason.as_deref() {
                None if interaction.success => SpecialBoundaryDisplay {
                    label: "DayCareWithdrawText".to_string(),
                    details: vec![format!(
                        "Perfect! Here's your\n{} back.",
                        interaction.pokemon.as_deref().map(canonical_species_display_name).unwrap_or_else(|| "POKéMON".to_string())
                    )],
                },
                Some("party_full") => SpecialBoundaryDisplay {
                    label: "DayCarePartyFullText".to_string(),
                    details: vec!["You have no room\nfor it.".to_string()],
                },
                Some("not_enough_money") => SpecialBoundaryDisplay {
                    label: "DayCareNotEnoughMoneyText".to_string(),
                    details: vec!["You don't have\nenough money.".to_string()],
                },
                _ => SpecialBoundaryDisplay {
                    label: "DayCareOhFineText".to_string(),
                    details: vec!["Oh, fine then.".to_string()],
                },
            });
            if interaction.success {
                runtime_shell.pending_special_cry = interaction.pokemon.clone();
                runtime_shell.special_boundary_queue.push_back(SpecialBoundaryDisplay {
                    label: "DayCareGotBackText".to_string(),
                    details: vec!["Come again.".to_string()],
                });
            }
            runtime_shell.last_audio_events.push(format!("day-care withdrawal outcome={:?}", used.outcome.effect));
            mark_runtime_snapshot_dirty(runtime_shell);
        }
        VisiblePcConfirmation::BuenaPrize { item_id } => {
            if accepted {
                let balance_before = runtime_shell.shell.snapshot()?.trainer.blue_card_balance;
                let used = runtime_shell.shell.use_buena_prize(item_id.clone(), 1)?;
                runtime_shell.last_audio_events.push(format!(
                    "Buena prize item={item_id} outcome={:?} checksum={:?}",
                    used.outcome.effect, used.state_checksum
                ));
                let snapshot = runtime_shell.shell.snapshot()?;
                if snapshot.trainer.blue_card_balance < balance_before {
                    queue_visible_shell_sound_effect(runtime_shell, "SFX_TRANSACTION")?;
                    runtime_shell.pc_notice = Some("Here you go!".to_string());
                } else {
                    let cost = snapshot
                        .special
                        .buena_prizes
                        .get(&item_id)
                        .copied()
                        .with_context(|| format!("Buena prize {item_id} disappeared"))?;
                    runtime_shell.pc_notice = Some(if u16::from(cost) > balance_before {
                        "You don't have\nenough points.".to_string()
                    } else {
                        "You have no room\nfor it.".to_string()
                    });
                }
            }
            mark_runtime_snapshot_dirty(runtime_shell);
        }
    }
    Ok(())
}

fn attach_visible_mailbox_mail(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let mailbox_index = runtime_shell.mailbox_attach_index.context("no mailbox message selected")?;
    let party_index = selected_party_index(runtime_shell)?;
    match runtime_shell.shell.attach_mailbox_mail_to_party(mailbox_index, party_index) {
        Ok(_) => {
            runtime_shell.pc_notice = Some("The MAIL was attached.".to_string());
            runtime_shell.mailbox_attach_index = None;
            close_visible_party_menu(runtime_shell);
            let snapshot = runtime_shell.shell.snapshot()?;
            if snapshot.mailbox.is_empty() {
                runtime_shell.mailbox_cursor = None;
                runtime_shell.player_pc_action_cursor = Some(MenuCursor { surface_id: "pc:player-actions".to_string(), option_index: 3 });
            } else {
                runtime_shell.mailbox_cursor = Some(MenuCursor { surface_id: "pc:mailbox".to_string(), option_index: mailbox_index.min(snapshot.mailbox.len() - 1) });
            }
        }
        Err(error) if error.to_string().contains("Egg") => runtime_shell.pc_notice = Some("An EGG can't hold MAIL.".to_string()),
        Err(error) if error.to_string().contains("already holding") => runtime_shell.pc_notice = Some("That Pokemon is holding an item.".to_string()),
        Err(error) => return Err(error),
    }
    Ok(())
}

fn turn_off_visible_pc_hub(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    record_visible_runtime_action(runtime_shell, "pc:hub:turn_off")?;
    runtime_shell.pc_hub_session_open = false;
    runtime_shell.pc_hub_cursor = None;
    runtime_shell.bill_pc_session_open = false;
    runtime_shell.bill_pc_action_cursor = None;
    runtime_shell.bill_pc_box_cursor = None;
    runtime_shell.bill_pc_move_open = false;
    runtime_shell.bill_pc_move_source = None;
    runtime_shell.storage_cursor = None;
    runtime_shell.pc_item_cursor = None;
    runtime_shell.pc_item_action = None;
    runtime_shell.pc_item_quantity = None;
    runtime_shell.player_pc_action_cursor = None;
    runtime_shell.mailbox_cursor = None;
    runtime_shell.mailbox_action_cursor = None;
    runtime_shell.mailbox_attach_index = None;
    runtime_shell.pc_confirmation = None;
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.ui.menu.is_some() {
        let _ = runtime_shell.shell.close_active_menu()?;
    } else if snapshot.ui.window_open {
        let _ = runtime_shell.shell.close_runtime_window()?;
    }
    set_shell_action_status(runtime_shell, "TURNED OFF THE PC");
    trim_event_log(&mut runtime_shell.last_audio_events);
    continue_visible_script_after_prompt(runtime_shell)
}

fn close_shop_or_teleport(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.pending_shop.is_some() {
        close_visible_shop(runtime_shell)
    } else {
        use_visible_teleport(runtime_shell)
    }
}

fn run_or_rock_smash(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let snapshot = runtime_shell.shell.snapshot()?;
    if snapshot.battle.is_some() {
        attempt_visible_battle_run(runtime_shell)
    } else {
        use_visible_rock_smash(runtime_shell)
    }
}

fn visible_static_wild_source(
    snapshot: &RuntimeShellSnapshot,
    battle: &crate::RuntimeBattleSnapshot,
) -> Option<(String, String)> {
    match &battle.kind {
        crate::RuntimeBattleKind::StaticWild { source_script, .. } => {
            Some((snapshot.overworld.map_name.clone(), source_script.clone()))
        }
        _ => None,
    }
}

fn finish_visible_wild_battle_exit(
    runtime_shell: &mut BevyRuntimeShell,
    scripted_static_wild: Option<(String, String)>,
    plain_reason: &str,
) -> Result<()> {
    reset_visible_battle_exit_state(runtime_shell);
    if let Some((map_name, source_script)) = scripted_static_wild {
        complete_visible_scripted_wild_battle(runtime_shell, &map_name, &source_script)
    } else {
        restore_visible_overworld_after_battle_exit(runtime_shell, plain_reason)
    }
}

fn restore_visible_overworld_after_battle_exit(
    runtime_shell: &mut BevyRuntimeShell,
    reason: &str,
) -> Result<()> {
    if runtime_shell.battle_messages.is_empty() {
        queue_visible_current_music(runtime_shell)?;
    }
    let snapshot = runtime_shell.shell.snapshot()?;
    record_visible_runtime_action(
        runtime_shell,
        format!(
            "battle:exit:{}:{}:{}:{}",
            reason,
            snapshot.overworld.map_name,
            snapshot.overworld.tile.x,
            snapshot.overworld.tile.y
        ),
    )?;
    set_shell_action_status(
        runtime_shell,
        format!(
            "BATTLE EXIT {} ({},{})",
            snapshot.overworld.map_name, snapshot.overworld.tile.x, snapshot.overworld.tile.y
        ),
    );
    Ok(())
}
