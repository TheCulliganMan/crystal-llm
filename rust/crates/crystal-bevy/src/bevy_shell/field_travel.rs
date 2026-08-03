fn begin_visible_dig_travel_animation(
    runtime_shell: &mut BevyRuntimeShell,
    returning: bool,
) -> Result<()> {
    let scene = visible_field_travel_scene(runtime_shell, returning, "Dig")?;
    let mut phases = VecDeque::new();
    if returning {
        phases.push_back(VisibleScriptMovementPhase::Visibility { hidden: false });
    }
    phases.push_back(VisibleScriptMovementPhase::Stationary {
        duration: 32,
        effect: VisibleStationaryMovementEffect::DigSpin,
    });
    if !returning {
        phases.push_back(VisibleScriptMovementPhase::Visibility { hidden: true });
    }
    begin_visible_field_travel_movement(runtime_shell, scene, phases)
}

fn begin_visible_teleport_travel_animation(
    runtime_shell: &mut BevyRuntimeShell,
    arriving: bool,
) -> Result<()> {
    let scene = visible_field_travel_scene(runtime_shell, arriving, "Teleport")?;
    let phases = if arriving {
        VecDeque::from([
            VisibleScriptMovementPhase::Stationary {
                duration: 17,
                effect: VisibleStationaryMovementEffect::TeleportWait,
            },
            VisibleScriptMovementPhase::Stationary {
                duration: 16,
                effect: VisibleStationaryMovementEffect::TeleportDescent,
            },
            VisibleScriptMovementPhase::Stationary {
                duration: 16,
                effect: VisibleStationaryMovementEffect::TeleportSpin,
            },
        ])
    } else {
        VecDeque::from([
            VisibleScriptMovementPhase::Stationary {
                duration: 16,
                effect: VisibleStationaryMovementEffect::TeleportSpin,
            },
            VisibleScriptMovementPhase::Stationary {
                duration: 16,
                effect: VisibleStationaryMovementEffect::TeleportRise,
            },
        ])
    };
    begin_visible_field_travel_movement(runtime_shell, scene, phases)
}

fn begin_visible_pitfall_landing(runtime_shell: &mut BevyRuntimeShell) -> Result<()> {
    let scene = runtime_shell.shell.snapshot()?;
    let phases = VecDeque::from([
        VisibleScriptMovementPhase::Sound {
            audio_id: "SFX_KINESIS".to_string(),
        },
        VisibleScriptMovementPhase::Stationary {
            duration: 16,
            effect: VisibleStationaryMovementEffect::SkyfallWait,
        },
        VisibleScriptMovementPhase::Stationary {
            duration: 16,
            effect: VisibleStationaryMovementEffect::SkyfallFall,
        },
        VisibleScriptMovementPhase::Sound {
            audio_id: "SFX_STRENGTH".to_string(),
        },
        VisibleScriptMovementPhase::ScreenShake { parameter: 16 },
        // The script `earthquake` opcode owns its complete duration before
        // FallIntoMapScript returns to destination-map scene processing.
        VisibleScriptMovementPhase::Hold { duration: 16 },
    ]);
    runtime_shell.visible_field_travel_animation = Some(VisibleFieldTravelAnimation::Pitfall);
    begin_visible_field_travel_movement(runtime_shell, scene, phases)
}

fn visible_field_travel_scene(
    runtime_shell: &BevyRuntimeShell,
    arriving: bool,
    move_name: &str,
) -> Result<RuntimeShellSnapshot> {
    if arriving {
        runtime_shell.shell.snapshot()
    } else {
        runtime_shell
            .field_notice_scene
            .as_ref()
            .with_context(|| format!("{move_name} has no retained departure scene"))
            .map(|scene| scene.as_ref().clone())
    }
}

fn begin_visible_field_travel_movement(
    runtime_shell: &mut BevyRuntimeShell,
    scene: RuntimeShellSnapshot,
    phases: VecDeque<VisibleScriptMovementPhase>,
) -> Result<()> {
    runtime_shell.visible_script_movement_scene = Some(Arc::new(scene.clone()));
    runtime_shell.visible_script_movement = Some(VisibleScriptMovement {
        object_id: "PLAYER".to_string(),
        phases,
        pending_programs: VecDeque::new(),
        hold_frames_remaining: 0,
        active_jump_duration: None,
        active_uses_standing_frame: false,
        active_tree_shake_duration: None,
        active_stationary_effect: None,
        active_stationary_duration: 0,
        stationary_y_offset: 0,
        stationary_initial_facing: scene.overworld.facing,
        follower_object_id: None,
        follower_queued_step: None,
        follower_active_jump_duration: None,
        follower_active_uses_standing_frame: false,
    });
    start_next_visible_script_movement_phase(runtime_shell)?;
    Ok(())
}
