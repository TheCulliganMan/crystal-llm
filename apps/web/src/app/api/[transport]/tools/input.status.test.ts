const mockLoadSession = jest.fn();
const mockResolveSessionId = jest.fn(() => "status-surface-test");

jest.mock("./common", () => ({
  MAX_ADVANCE_FRAMES: 8,
  invalidateObserveSnapshotCache: jest.fn(),
  loadSession: (...args: unknown[]) => mockLoadSession(...args),
  resolveSessionId: (...args: unknown[]) => mockResolveSessionId(...args),
  reportSnapshot: jest.fn(),
  withRequestIdentity: (_extra: unknown, fn: () => unknown) => fn(),
}));

jest.mock("@/app/mcp/play-training-recorder", () => ({
  recordWebTrainingTurn: jest.fn(),
}));

import { statusHandler } from "./input";

describe("statusHandler", () => {
  beforeEach(() => {
    mockLoadSession.mockReset();
    mockResolveSessionId.mockClear();
  });

  it("includes live interaction tile and target in the compact agent-facing status surface", async () => {
    mockLoadSession.mockResolvedValue({
      observeText: jest.fn(() => "OVERWORLD\n@"),
      status: jest.fn().mockResolvedValue({
        mode: "overworld",
        map: "ElmsLab",
        map_details: undefined,
        location_name: "ElmsLab",
        map_id: "24:5",
        coords: { x: 11, y: 7 },
        interaction_tile: { x: 13, y: 7 },
        interaction_target: {
          x: 13,
          y: 7,
          kind: "bg_event",
          label: "Cyndaquil Poke Ball",
          token: "!",
          hotspot_type: "objective",
          script: "CyndaquilPokeBallScript",
        },
        scene: {
          active_script: "LabTryToLeaveScript",
          scene_owner: {
            kind: "bg_event",
            x: 9,
            y: 13,
            label: "Route trigger",
            token: "!",
            hotspot_type: "trigger",
            script: "LabTryToLeaveScript",
          },
        },
        facing: "right",
        badges_count: 0,
        in_menu: false,
        in_battle: false,
        in_dialog: false,
        textbox_open: false,
        text_box_open: false,
        prompt_pending: false,
        movement_locked: false,
        script_busy: false,
        can_move: true,
        input_blocked_reason: undefined,
        flow_state: undefined,
        party_summary: { count: 0 },
        last_action_result: undefined,
        last_n_events: [],
      }),
    });

    const response = await statusHandler({}, {});
    const payload = response.content[0]?.type === "text" ? JSON.parse(response.content[0].text) : null;

    expect(response.content[0]).toMatchObject({ mimeType: "application/json" });
    expect(payload).toMatchObject({
      directionConvention: {
        coord: "x+ right, x- left, y+ down, y- up",
        move: {
          up: [0, -1],
          down: [0, 1],
          left: [-1, 0],
          right: [1, 0],
        },
      },
      interactionTile: [13, 7],
      interactionTarget: {
        coords: [13, 7],
        kind: "bg_event",
        label: "Cyndaquil Poke Ball",
        token: "!",
        hotspotType: "objective",
        script: "CyndaquilPokeBallScript",
      },
      scene: {
        activeScript: "LabTryToLeaveScript",
      },
    });
    expect(payload.scene.owner).toBeUndefined();
  });

  it("includes the audio playback snapshot for TUI sound", async () => {
    mockLoadSession.mockResolvedValue({
      observeText: jest.fn(() => "OVERWORLD\n@"),
      status: jest.fn().mockResolvedValue({
        mode: "overworld",
        map: "NewBarkTown",
        map_details: undefined,
        location_name: "NewBarkTown",
        map_id: "24:4",
        coords: { x: 4, y: 7 },
        facing: "down",
        badges_count: 0,
        in_menu: false,
        in_battle: false,
        in_dialog: false,
        textbox_open: false,
        text_box_open: false,
        prompt_pending: false,
        movement_locked: false,
        script_busy: false,
        can_move: true,
        input_blocked_reason: undefined,
        flow_state: undefined,
        party_summary: { count: 0 },
        last_action_result: undefined,
        last_n_events: [],
        audio: {
          musicToken: "MUSIC_NEW_BARK_TOWN",
          musicRole: "map",
          musicSource: "/api/audio/newbarktown.mp3",
          musicFrame: 12,
          fadedVolume: 1,
          activeChannels: [{ channel: 0, ownerToken: "MUSIC_NEW_BARK_TOWN", category: "music", role: "map" }],
          recentEvents: [
            {
              sequence: 7,
              kind: "sfx",
              token: "SFX_READ_TEXT_2",
              source: "/api/audio/sfx/readtext2.mp3",
            },
          ],
        },
      }),
    });

    const response = await statusHandler({}, {});
    const payload = response.content[0]?.type === "text" ? JSON.parse(response.content[0].text) : null;

    expect(payload).toMatchObject({
      audio: {
        musicToken: "MUSIC_NEW_BARK_TOWN",
        musicRole: "map",
        musicSource: "/api/audio/newbarktown.mp3",
        recentEvents: [
          {
            sequence: 7,
            kind: "sfx",
            token: "SFX_READ_TEXT_2",
            source: "/api/audio/sfx/readtext2.mp3",
          },
        ],
      },
    });
    expect(payload.audio).not.toHaveProperty("activeChannels");
  });

  it("uses visible title screen state instead of stale overworld memory context", async () => {
    mockLoadSession.mockResolvedValue({
      observeText: jest.fn(() => [
        "TITLE",
        "POKEMON CRYSTAL",
        "TITLE SCREEN",
        "",
        "TITLE",
        "STATE: entrance",
      ].join("\n")),
      status: jest.fn().mockResolvedValue({
        mode: "overworld",
        map: "PlayersHouse2F",
        map_details: undefined,
        location_name: "PlayersHouse2F",
        map_id: "24:7",
        coords: { x: 3, y: 3 },
        interaction_tile: { x: 3, y: 5 },
        interaction_lane: {
          hotspot: { x: 1, y: 3, label: "NPC", token: "N", hotspot_type: "npc" },
          lane: {
            x: 3,
            y: 3,
            facing: "left",
            facing_aligned: false,
            facing_move_leaves_lane: true,
            target_confirmed: false,
          },
        },
        local_focus: {
          source: "interaction_lane",
          target: { kind: "npc", x: 1, y: 3, label: "NPC", token: "N", hotspot_type: "npc" },
        },
        facing: "down",
        badges_count: 0,
        in_menu: false,
        in_battle: false,
        in_dialog: false,
        textbox_open: false,
        text_box_open: false,
        prompt_pending: false,
        movement_locked: false,
        script_busy: false,
        can_move: true,
        input_blocked_reason: undefined,
        flow_state: undefined,
        party_summary: { count: 0 },
        last_action_result: undefined,
        last_n_events: [],
      }),
    });

    const response = await statusHandler({}, {});
    const payload = response.content[0]?.type === "text" ? JSON.parse(response.content[0].text) : null;

    expect(payload).toMatchObject({
      mode: "title",
      map: "TITLE",
      location: "TITLE",
      mapId: "title",
      canMove: false,
      blockedReason: "title_screen",
      partyCount: 0,
    });
    expect(payload.coords).toBeUndefined();
    expect(payload.interactionLane).toBeUndefined();
    expect(payload.localFocus).toBeUndefined();
  });

  it("uses structured surface status for Oak intro and suppresses stale overworld context", async () => {
    mockLoadSession.mockResolvedValue({
      observeText: jest.fn(() => "OAK INTRO\nSPRITE: OAK"),
      status: jest.fn().mockResolvedValue({
        mode: "oak_intro",
        surface: {
          kind: "oak_intro",
          title: "Oak Intro",
          state: "oak_intro",
          phase: "text",
          waiting: true,
          dialogue_open: true,
          primary_text: "Hello! Sorry to keep you waiting!",
        },
        map: "OAK INTRO",
        map_details: undefined,
        location_name: "OAK INTRO",
        map_id: "oak_intro",
        coords: { x: 3, y: 3 },
        interaction_tile: { x: 3, y: 5 },
        facing: "down",
        badges_count: 0,
        in_menu: true,
        in_battle: false,
        in_dialog: true,
        textbox_open: false,
        text_box_open: false,
        prompt_pending: true,
        movement_locked: true,
        script_busy: false,
        can_move: false,
        input_blocked_reason: "oak_intro",
        flow_state: undefined,
        party_summary: { count: 0 },
        last_action_result: undefined,
        last_n_events: [],
      }),
    });

    const response = await statusHandler({}, {});
    const payload = response.content[0]?.type === "text" ? JSON.parse(response.content[0].text) : null;

    expect(payload).toMatchObject({
      mode: "oak_intro",
      surface: {
        kind: "oak_intro",
        title: "Oak Intro",
        phase: "text",
        waiting: true,
        dialogueOpen: true,
      },
      map: "OAK INTRO",
      mapId: "oak_intro",
      canMove: false,
      blockedReason: "oak_intro",
      promptPending: true,
    });
    expect(payload.coords).toBeUndefined();
    expect(payload.interactionTile).toBeUndefined();
  });

  it("uses structured PC surface status as a menu without leaking stale overworld lanes", async () => {
    mockLoadSession.mockResolvedValue({
      observeText: jest.fn(() => "PC\nACCESS WHOSE PC?\nMENU\n▶ BILL'S PC"),
      status: jest.fn().mockResolvedValue({
        mode: "menu",
        surface: {
          kind: "pc",
          title: "PC",
          menu_open: true,
          selected: "BILL'S PC",
          controls: ["D-Pad=Move A=Select B=Back"],
          primary_text: "BILL'S PC",
        },
        map: "PlayersHouse2F",
        map_details: undefined,
        location_name: "PlayersHouse2F",
        map_id: "24:7",
        coords: { x: 3, y: 3 },
        interaction_lane: {
          hotspot: { x: 1, y: 3, label: "NPC", token: "N", hotspot_type: "npc" },
          lane: {
            x: 3,
            y: 3,
            facing: "left",
            facing_aligned: false,
            facing_move_leaves_lane: true,
            target_confirmed: false,
          },
        },
        local_focus: {
          source: "interaction_lane",
          target: { kind: "npc", x: 1, y: 3, label: "NPC", token: "N", hotspot_type: "npc" },
        },
        facing: "down",
        badges_count: 0,
        in_menu: true,
        in_battle: false,
        in_dialog: false,
        textbox_open: false,
        text_box_open: false,
        prompt_pending: false,
        movement_locked: false,
        script_busy: false,
        can_move: false,
        input_blocked_reason: "menu",
        flow_state: undefined,
        party_summary: { count: 0 },
        last_action_result: undefined,
        last_n_events: [],
      }),
    });

    const response = await statusHandler({}, {});
    const payload = response.content[0]?.type === "text" ? JSON.parse(response.content[0].text) : null;

    expect(payload).toMatchObject({
      mode: "menu",
      surface: {
        kind: "pc",
        selected: "BILL'S PC",
      },
      map: "PC",
      mapId: "pc",
      inMenu: true,
      blockedReason: "pc",
      canMove: false,
    });
    expect(payload.coords).toBeUndefined();
    expect(payload.interactionLane).toBeUndefined();
    expect(payload.localFocus).toBeUndefined();
  });

  it("includes surfaced local focus so the agent can keep the active scene target above ambient targets", async () => {
    mockLoadSession.mockResolvedValue({
      observeText: jest.fn(() => "OVERWORLD\n@"),
      status: jest.fn().mockResolvedValue({
        mode: "overworld",
        map: "ElmsLab",
        map_details: undefined,
        location_name: "ElmsLab",
        map_id: "24:5",
        coords: { x: 11, y: 3 },
        interaction_tile: { x: 13, y: 3 },
        interaction_target: {
          x: 13,
          y: 3,
          kind: "bg_event",
          label: "Bookshelf",
          token: "B",
          hotspot_type: "sign",
          script: "ElmsLabBookshelf",
        },
        interaction_lane: {
          hotspot: {
            x: 11,
            y: 5,
            label: "Elm",
            token: "N",
            hotspot_type: "npc",
          },
          lane: {
            x: 11,
            y: 3,
            facing: "down",
            facing_aligned: false,
            facing_move_leaves_lane: true,
            target_confirmed: false,
          },
        },
        local_focus: {
          source: "interaction_lane",
          target: {
            kind: "npc",
            x: 11,
            y: 5,
            label: "Elm",
            token: "N",
            hotspot_type: "npc",
          },
        },
        facing: "right",
        badges_count: 0,
        in_menu: false,
        in_battle: false,
        in_dialog: false,
        textbox_open: false,
        text_box_open: false,
        prompt_pending: false,
        movement_locked: false,
        script_busy: false,
        can_move: true,
        input_blocked_reason: undefined,
        flow_state: undefined,
        party_summary: { count: 0 },
        last_action_result: undefined,
        last_n_events: [],
      }),
    });

    const response = await statusHandler({}, {});
    const payload = response.content[0]?.type === "text" ? JSON.parse(response.content[0].text) : null;

    expect(payload).toMatchObject({
      interactionTarget: {
        label: "Bookshelf",
        hotspotType: "sign",
      },
      interactionLane: {
        lane: {
          targetConfirmed: false,
        },
      },
      localFocus: {
        source: "interaction_lane",
        target: {
          kind: "npc",
          coords: [11, 5],
          label: "Elm",
          token: "N",
          hotspotType: "npc",
        },
      },
    });
  });

  it("serializes interaction-pivot local focus when an objective lane proved inert and the scene should fall back to a nearby npc", async () => {
    mockLoadSession.mockResolvedValue({
      observeText: jest.fn(() => "OVERWORLD\n@"),
      status: jest.fn().mockResolvedValue({
        mode: "overworld",
        map: "ElmsLab",
        map_details: { coord_stride: 2 },
        location_name: "ElmsLab",
        map_id: "24:5",
        coords: { x: 13, y: 5 },
        interaction_tile: { x: 13, y: 7 },
        interaction_target: {
          x: 13,
          y: 7,
          kind: "bg_event",
          label: "Cyndaquil Poke Ball",
          token: "!",
          hotspot_type: "objective",
        },
        interaction_lane: {
          hotspot: {
            x: 13,
            y: 7,
            label: "Cyndaquil Poke Ball",
            token: "!",
            hotspot_type: "objective",
          },
          lane: {
            x: 13,
            y: 5,
            facing: "down",
            facing_aligned: true,
            facing_move_leaves_lane: true,
            target_confirmed: true,
          },
        },
        local_focus: {
          source: "interaction_pivot",
          target: {
            kind: "npc",
            x: 11,
            y: 5,
            label: "Elm",
            token: "N",
            hotspot_type: "npc",
          },
          recommended_approach: {
            x: 11,
            y: 3,
            facing: "down",
          },
        },
        facing: "down",
        badges_count: 0,
        in_menu: false,
        in_battle: false,
        in_dialog: false,
        textbox_open: false,
        text_box_open: false,
        prompt_pending: false,
        movement_locked: false,
        script_busy: false,
        can_move: true,
        input_blocked_reason: undefined,
        flow_state: undefined,
        party_summary: { count: 0 },
        last_action_result: undefined,
        last_n_events: [],
      }),
    });

    const response = await statusHandler({}, {});
    const payload = response.content[0]?.type === "text" ? JSON.parse(response.content[0].text) : null;

    expect(payload).toMatchObject({
      interactionLane: {
        lane: {
          targetConfirmed: true,
        },
      },
      localFocus: {
        source: "interaction_pivot",
        target: {
          kind: "npc",
          coords: [11, 5],
          label: "Elm",
          token: "N",
          hotspotType: "npc",
        },
        recommendedApproach: {
          coords: [11, 3],
          facing: "down",
          setupFrom: [11, 1],
        },
      },
    });
  });

  it("includes more than eight structured hotspots so local interactables are not crowded out", async () => {
    mockLoadSession.mockResolvedValue({
      status: jest.fn().mockResolvedValue({
        mode: "overworld",
        map: "ElmsLab",
        map_details: {
          map: "ElmsLab",
          map_id: "24:5",
          warps: [],
          hotspots: Array.from({ length: 12 }, (_, index) => ({
            id: `hs-${index + 1}`,
            type: index === 11 ? "sign" : "objective",
            label: index === 11 ? "Trash can" : `Hotspot ${index + 1}`,
            coords: { x: index + 1, y: index + 2 },
            visible: true,
            interactable: true,
            token: index === 11 ? "T" : "!",
          })),
        },
        location_name: "ElmsLab",
        map_id: "24:5",
        coords: { x: 11, y: 7 },
        interaction_tile: undefined,
        interaction_target: undefined,
        facing: "right",
        badges_count: 0,
        in_menu: false,
        in_battle: false,
        in_dialog: false,
        textbox_open: false,
        text_box_open: false,
        prompt_pending: false,
        movement_locked: false,
        script_busy: false,
        can_move: true,
        input_blocked_reason: undefined,
        flow_state: undefined,
        party_summary: { count: 0 },
        last_action_result: undefined,
        last_n_events: [],
      }),
      observeText: jest.fn(() => ""),
    });

    const response = await statusHandler({}, {});
    const payload = response.content[0]?.type === "text" ? JSON.parse(response.content[0].text) : null;

    expect(response.content[0]).toMatchObject({ mimeType: "application/json" });
    expect(payload).toMatchObject({
      map: "ElmsLab",
      location: "ElmsLab",
      coords: [11, 7],
    });
  });

  it("includes structured adjacent travel directions from the live observe snapshot", async () => {
    mockLoadSession.mockResolvedValue({
      status: jest.fn().mockResolvedValue({
        mode: "overworld",
        map: "PlayersHouse2F",
        map_details: undefined,
        location_name: "PlayersHouse2F",
        map_id: "24:7",
        coords: { x: 3, y: 3 },
        interaction_tile: undefined,
        interaction_target: undefined,
        facing: "down",
        badges_count: 0,
        in_menu: false,
        in_battle: false,
        in_dialog: false,
        textbox_open: false,
        text_box_open: false,
        prompt_pending: false,
        movement_locked: false,
        script_busy: false,
        can_move: true,
        input_blocked_reason: undefined,
        flow_state: undefined,
        party_summary: { count: 0 },
        last_action_result: undefined,
        last_n_events: [],
      }),
      observeText: jest.fn(() =>
        [
          "OVERWORLD",
          '{"ow":{"g":[["#","#","#"],[".","@v","#"],[".",".","."]],"p":[1,1]}}',
        ].join("\n")
      ),
    });

    const response = await statusHandler({}, {});
    const payload = response.content[0]?.type === "text" ? JSON.parse(response.content[0].text) : null;

    expect(payload).toMatchObject({
      localMovement: {
        openDirections: [
          { direction: "down", tile: "." },
          { direction: "left", tile: "." },
        ],
        blockedDirections: [
          { direction: "up", tile: "#" },
          { direction: "right", tile: "#" },
        ],
      },
    });
  });

  it("includes wallet and Mom-bank money in the compact agent-facing status", async () => {
    mockLoadSession.mockResolvedValue({
      observeText: jest.fn(() => "OVERWORLD\n@"),
      status: jest.fn().mockResolvedValue({
        mode: "overworld",
        map: "GoldenrodCity",
        map_details: undefined,
        location_name: "GoldenrodCity",
        map_id: "16:14",
        coords: { x: 20, y: 8 },
        facing: "down",
        badges_count: 3,
        money: 4321,
        moms_money: 987,
        mom_saving_some_money: true,
        in_menu: false,
        in_battle: false,
        in_dialog: false,
        textbox_open: false,
        text_box_open: false,
        prompt_pending: false,
        movement_locked: false,
        script_busy: false,
        can_move: true,
        input_blocked_reason: undefined,
        flow_state: undefined,
        party_summary: { count: 2 },
        last_action_result: undefined,
        last_n_events: [],
      }),
    });

    const response = await statusHandler({}, {});
    const payload = response.content[0]?.type === "text" ? JSON.parse(response.content[0].text) : null;

    expect(payload).toMatchObject({
      money: 4321,
      momsMoney: 987,
      momSavingSomeMoney: true,
    });
  });
});
