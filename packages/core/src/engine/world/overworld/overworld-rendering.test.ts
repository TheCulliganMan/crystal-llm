import {
  __test__buildCrowdMarkerPositions,
  __test__computeCameraRect,
  __test__findTextUi,
  __test__formatAxisHeader,
  __test__formatRowLabel,
  OverworldRenderingMixin,
} from "@pokecrystal/core/engine/world/overworld/overworld-rendering";
import { resolveCollisionValue } from "@pokecrystal/core/engine/world/overworld/collision-data";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { CompositeUI } from "@pokecrystal/core/ui/composite-ui";

describe("overworld rendering helpers", () => {
  describe("camera rect helper", () => {
    it("caps the visible viewport to map dimensions", () => {
      const rect = __test__computeCameraRect(80, 90, 160, 120, 40, 30);
      expect(rect.visibleWidth).toBe(80);
      expect(rect.visibleHeight).toBe(90);
      expect(rect.cameraX).toBe(0);
      expect(rect.cameraY).toBe(0);
    });

    it("keeps the camera within bounds when the map is larger than the viewport", () => {
      const rect = __test__computeCameraRect(200, 200, 100, 80, 195, 195);
      expect(rect.visibleWidth).toBe(100);
      expect(rect.visibleHeight).toBe(80);
      expect(rect.cameraX).toBe(100);
      expect(rect.cameraY).toBe(120);
    });
  });

  it("prefers text children over composite wrappers", () => {
    const textChild = { renderSnapshot: jest.fn() };
    const primaryChild = { clearScreen: jest.fn() };
    const composite = new CompositeUI(primaryChild, textChild);

    const result = __test__findTextUi(composite);

    expect(result).toBe(textChild);
  });

  it("returns a text target when the ui itself is text-capable", () => {
    const textTarget = { renderSnapshot: jest.fn() };

    const result = __test__findTextUi(textTarget);

    expect(result).toBe(textTarget);
  });

  it("skips ascii snapshot emission when the caller suppresses text snapshots", () => {
    const textTarget = { renderSnapshot: jest.fn(), renderOverworldOverlay: jest.fn() };
    const mapData = {
      mapName: "TEST_MAP",
      width: 1,
      height: 1,
      getMetatileAt: () => 0,
    };
    const tileset = {
      tilesetName: "TEST",
      metatiles: [{ collision: [0, 0, 0, 0] }],
    };

    class TestOverworld extends OverworldRenderingMixin {}
    const overworld = new TestOverworld() as OverworldRenderingMixin & Record<string, unknown>;
    Object.assign(overworld, {
      ui: textTarget,
      _suppress_text_snapshot: true,
      map: mapData,
      tileset,
      _map_events: { warps: [], coord_events: [], bg_events: [] },
      player_x: 3,
      player_y: 3,
      player_direction: "down",
      TILES_PER_COLLISION: 2,
      _text_ui_color: false,
      _ascii_overlay_cache_key: null,
      _ascii_overlay_cached_viewport: null,
      _ascii_overlay_cached_info: null,
      _ascii_overlay_last_npc_positions: [],
      _ascii_overlay_last_event_identity: null,
      _ascii_overlay_last_event_counts: null,
      _last_block_feedback: null,
      npcs: [],
      game_state: null,
      screen: {
        fill: jest.fn(),
        get_width: jest.fn(() => 160),
        get_height: jest.fn(() => 144),
        blit: jest.fn(),
      },
      _composite_surface: {
        get_size: () => [160, 144] as [number, number],
      },
      map_surface: null,
      _composite_origin: [0, 0],
      _composite_priority_surface: null,
      priority_surface: null,
      _field_move_animation_renderer: null,
      _active_emotes: new Map(),
      _grass_rustle: null,
      _phone_call_overlay: null,
      dialogue: null,
      _town_map_overlay: null,
      _egg_hatch_animation: null,
      _map_sign: null,
      pokepic_overlay: null,
      _fade_alpha: 0,
      _poison_overlay_alpha: 0,
      _debug_sightlines: false,
      player_animations: {
        down: { currentFrame: { get_width: () => 16, get_height: () => 16 } },
      },
      player_px_x: 0,
      player_px_y: 0,
      target_px_x: 0,
      target_px_y: 0,
      player_object: null,
      _npc_pixel_position: () => [0, 0],
    });

    overworld.draw();

    expect(textTarget.renderOverworldOverlay).not.toHaveBeenCalled();
    expect(textTarget.renderSnapshot).not.toHaveBeenCalled();
  });

  it("does not draw off-map NPC placeholders into the black viewport margin", () => {
    class TestOverworld extends OverworldRenderingMixin {}

    const mapSurface = new gameEngine.Surface(160, 128);
    const screen = new gameEngine.Surface(320, 288);
    const playerSprite = new gameEngine.Surface(16, 16);
    const visibleNpcSprite = new gameEngine.Surface(16, 16);
    const offMapNpcSprite = new gameEngine.Surface(16, 16);
    const blitSpy = jest.spyOn(screen, "blitAt");

    const visibleNpc = {
      x: 5,
      y: 5,
      direction: "down",
      spriteId: "VISIBLE",
      animations: { down: { currentFrame: visibleNpcSprite } },
    };
    const offMapNpc = {
      x: 21,
      y: 5,
      direction: "down",
      spriteId: "OFF_MAP",
      animations: { down: { currentFrame: offMapNpcSprite } },
    };

    const overworld = new TestOverworld() as OverworldRenderingMixin & Record<string, unknown>;
    Object.assign(overworld, {
      screen,
      map: {
        mapName: "GOLDENROD_POKECENTER_1F",
        width: 5,
        height: 4,
        getMetatileAt: () => 0,
      },
      tileset: {
        tilesetName: "TEST",
        metatiles: [{ collision: [0, 0, 0, 0] }],
      },
      _composite_surface: null,
      map_surface: mapSurface,
      _composite_origin: [0, 0],
      _composite_priority_surface: null,
      priority_surface: null,
      _field_move_animation_renderer: null,
      _active_emotes: new Map(),
      _grass_rustle: null,
      _phone_call_overlay: null,
      dialogue: null,
      _town_map_overlay: null,
      _egg_hatch_animation: null,
      _map_sign: null,
      pokepic_overlay: null,
      _fade_alpha: 0,
      _poison_overlay_alpha: 0,
      _debug_sightlines: false,
      player_animations: {
        down: { currentFrame: playerSprite },
      },
      player_direction: "down",
      player_x: 3,
      player_y: 3,
      player_px_x: 24,
      player_px_y: 24,
      target_px_x: 24,
      target_px_y: 24,
      player_object: null,
      TILES_PER_COLLISION: 2,
      npcs: [visibleNpc, offMapNpc],
      _npc_pixel_position: (npc: { x: number; y: number }) => [
        (npc.x - 1) * 8,
        (npc.y - 1) * 8,
      ],
      _map_events: { warps: [], coord_events: [], bg_events: [] },
      game_state: null,
    });

    overworld.draw();

    const blittedSources = blitSpy.mock.calls.map((call) => call[0]);
    expect(blittedSources).toContain(visibleNpcSprite);
    expect(blittedSources).not.toContain(offMapNpcSprite);
  });

  it("draws lower-index overlapping NPC sprites above higher-index ones", () => {
    class TestOverworld extends OverworldRenderingMixin {}

    const mapSurface = new gameEngine.Surface(160, 144);
    const screen = new gameEngine.Surface(160, 144);
    const playerSprite = new gameEngine.Surface(16, 16);
    const lowerIndexSprite = new gameEngine.Surface(16, 16);
    const higherIndexSprite = new gameEngine.Surface(16, 16);
    const blitSpy = jest.spyOn(screen, "blitAt");
    playerSprite.fill([0, 0, 0, 0]);
    lowerIndexSprite.fill([255, 57, 8, 255]);
    higherIndexSprite.fill([106, 106, 106, 255]);

    const lowerIndexNpc = {
      x: 5,
      y: 5,
      objectIndex: 2,
      direction: "down",
      spriteId: "BURNEDTOWERB1F_ENTEI1",
      animations: { down: { currentFrame: lowerIndexSprite } },
    };
    const higherIndexNpc = {
      x: 5,
      y: 5,
      objectIndex: 5,
      direction: "down",
      spriteId: "BURNEDTOWERB1F_ENTEI2",
      animations: { down: { currentFrame: higherIndexSprite } },
    };

    const overworld = new TestOverworld() as OverworldRenderingMixin & Record<string, unknown>;
    Object.assign(overworld, {
      screen,
      map: {
        mapName: "BURNED_TOWER_B1F",
        width: 10,
        height: 10,
        getMetatileAt: () => 0,
      },
      tileset: {
        tilesetName: "TEST",
        metatiles: [{ collision: [0, 0, 0, 0] }],
      },
      _composite_surface: null,
      map_surface: mapSurface,
      _composite_origin: [0, 0],
      _composite_priority_surface: null,
      priority_surface: null,
      _field_move_animation_renderer: null,
      _active_emotes: new Map(),
      _grass_rustle: null,
      _phone_call_overlay: null,
      dialogue: null,
      _town_map_overlay: null,
      _egg_hatch_animation: null,
      _map_sign: null,
      pokepic_overlay: null,
      _fade_alpha: 0,
      _poison_overlay_alpha: 0,
      _debug_sightlines: false,
      player_animations: {
        down: { currentFrame: playerSprite },
      },
      player_direction: "down",
      player_x: 1,
      player_y: 1,
      player_px_x: 8,
      player_px_y: 8,
      target_px_x: 8,
      target_px_y: 8,
      player_object: null,
      TILES_PER_COLLISION: 2,
      npcs: [lowerIndexNpc, higherIndexNpc],
      _npc_pixel_position: () => [40, 40],
      _map_events: { warps: [], coord_events: [], bg_events: [] },
      game_state: null,
    });

    overworld.draw();

    const blittedSources = blitSpy.mock.calls.map((call) => call[0]);
    const higherIndexPosition = blittedSources.indexOf(higherIndexSprite);
    const lowerIndexPosition = blittedSources.indexOf(lowerIndexSprite);
    expect(higherIndexPosition).toBeGreaterThanOrEqual(0);
    expect(lowerIndexPosition).toBeGreaterThanOrEqual(0);
    expect(higherIndexPosition).toBeLessThan(lowerIndexPosition);
  });

  it("formats axis headers using tile coordinates", () => {
    const header = __test__formatAxisHeader(8, 5);

    expect(header).toEqual(["08 09 10 11 12"]);
  });

  it("formats axis headers for scaled collision strides", () => {
    const header = __test__formatAxisHeader(1, 6);

    expect(header).toEqual(["01 02 03 04 05 06"]);
  });

  it("formats row labels from scaled coordinates", () => {
    const label = __test__formatRowLabel(5, 2);

    expect(label).toBe("05");
  });

  it("keeps the ascii overlay legend minimal", () => {
    const textTarget = { renderOverworldOverlay: jest.fn() };
    const mapData = {
      mapName: "TEST_MAP",
      width: 1,
      height: 1,
      getMetatileAt: () => 0,
    };
    const tileset = {
      tilesetName: "TEST",
      metatiles: [{ collision: [0, 0, 0, 0] }],
    };

    class TestOverworld extends OverworldRenderingMixin {}
    const overworld = new TestOverworld() as OverworldRenderingMixin & Record<string, unknown>;
    Object.assign(overworld, {
      map: mapData,
      tileset,
      _map_events: { warps: [], coord_events: [], bg_events: [] },
      player_x: 3,
      player_y: 3,
      player_direction: "down",
      TILES_PER_COLLISION: 2,
      _text_ui_color: false,
      _ascii_overlay_cache_key: null,
      _ascii_overlay_cached_viewport: null,
      _ascii_overlay_cached_info: null,
      _ascii_overlay_last_npc_positions: [],
      _ascii_overlay_last_event_identity: null,
      _ascii_overlay_last_event_counts: null,
      _last_block_feedback: null,
      npcs: [],
      game_state: null,
    });

    (overworld as any)._draw_ascii_overworld(textTarget);

    const [viewportLines, infoLines] = (textTarget.renderOverworldOverlay as jest.Mock).mock.calls[0];
    const viewportText = viewportLines.join("\n");

    expect(viewportText).toContain(".");
    expect(infoLines).toEqual([
      "D-Pad=Move A=Talk Start=Menu Select=Item B=Back",
      "Pos: (1,1)",
      "Legend: @=Player .=Floor v=Down",
    ]);
  });

  it("labels doors and NPCs distinctly in the ascii overlay legend", () => {
    const textTarget = { renderOverworldOverlay: jest.fn() };
    const mapData = {
      mapName: "TEST_MAP",
      width: 2,
      height: 2,
      getMetatileAt: () => 0,
    };
    const tileset = {
      tilesetName: "TEST",
      metatiles: [{ collision: [0, 0, 0, 0] }],
    };

    class TestOverworld extends OverworldRenderingMixin {}
    const overworld = new TestOverworld() as OverworldRenderingMixin & Record<string, unknown>;
    Object.assign(overworld, {
      map: mapData,
      tileset,
      _map_events: {
        warps: [
          { x: 0, y: 0, index: 1, target_map: "Pokecenter1F", target_warp_id: 1 },
          { x: 1, y: 0, index: 2, target_map: "VioletGym", target_warp_id: 1 },
        ],
        coord_events: [],
        bg_events: [],
      },
      player_x: 7,
      player_y: 7,
      player_direction: "down",
      TILES_PER_COLLISION: 2,
      _text_ui_color: false,
      _ascii_overlay_cache_key: null,
      _ascii_overlay_cached_viewport: null,
      _ascii_overlay_cached_info: null,
      _ascii_overlay_last_npc_positions: [],
      _ascii_overlay_last_event_identity: null,
      _ascii_overlay_last_event_counts: null,
      _last_block_feedback: null,
      npcs: [{ x: 0, y: 2, direction: "left" }],
      game_state: null,
    });

    (overworld as any)._draw_ascii_overworld(textTarget);

    const [, infoLines] = (textTarget.renderOverworldOverlay as jest.Mock).mock.calls[0];
    const legendText = infoLines.join("\n");

    expect(legendText).toContain("D=Door");
    expect(legendText).toContain("N=Person");
    expect(legendText).toContain("DG=Door to Gym/House");
    expect(legendText).toContain("N<=NPC facing left");
  });

  it("tracks current and last NPC footprints in the ascii overlay cache key", () => {
    const textTarget = { renderOverworldOverlay: jest.fn() };
    const mapData = {
      mapName: "TEST_MAP",
      width: 4,
      height: 2,
      getMetatileAt: () => 0,
    };
    const tileset = {
      tilesetName: "TEST",
      metatiles: [{ collision: [0, 0, 0, 0] }],
    };

    class TestOverworld extends OverworldRenderingMixin {}
    const overworld = new TestOverworld() as OverworldRenderingMixin & Record<string, unknown>;
    Object.assign(overworld, {
      map: mapData,
      tileset,
      _map_events: { warps: [], coord_events: [], bg_events: [] },
      player_x: 1,
      player_y: 1,
      player_direction: "down",
      TILES_PER_COLLISION: 2,
      _text_ui_color: false,
      _ascii_overlay_cache_key: null,
      _ascii_overlay_cached_viewport: null,
      _ascii_overlay_cached_info: null,
      _ascii_overlay_last_npc_positions: [],
      _ascii_overlay_last_event_identity: null,
      _ascii_overlay_last_event_counts: null,
      _last_block_feedback: null,
      npcs: [{
        x: 3,
        y: 1,
        prev_x: 1,
        prev_y: 1,
        collisionStride: 2,
        direction: "right",
      }],
      game_state: null,
    });

    (overworld as any)._draw_ascii_overworld(textTarget);

    expect((overworld as any)._ascii_overlay_last_npc_positions).toEqual([
      [1, 1, 0, 0, 0, 2],
    ]);
  });

  it("renders stride-scaled NPCs as a single ASCII marker", () => {
    const textTarget = { renderOverworldOverlay: jest.fn() };
    const mapData = {
      mapName: "TEST_MAP",
      width: 4,
      height: 2,
      getMetatileAt: () => 0,
    };
    const tileset = {
      tilesetName: "TEST",
      metatiles: [{ collision: [0, 0, 0, 0] }],
    };

    class TestOverworld extends OverworldRenderingMixin {}
    const overworld = new TestOverworld() as OverworldRenderingMixin & Record<string, unknown>;
    Object.assign(overworld, {
      map: mapData,
      tileset,
      _map_events: { warps: [], coord_events: [], bg_events: [] },
      player_x: 13,
      player_y: 5,
      player_direction: "down",
      TILES_PER_COLLISION: 2,
      _text_ui_color: false,
      _ascii_overlay_cache_key: null,
      _ascii_overlay_cached_viewport: null,
      _ascii_overlay_cached_info: null,
      _ascii_overlay_last_npc_positions: [],
      _ascii_overlay_last_event_identity: null,
      _ascii_overlay_last_event_counts: null,
      _last_block_feedback: null,
      npcs: [{
        x: 5,
        y: 5,
        prev_x: 1,
        prev_y: 5,
        collisionStride: 2,
        direction: "right",
      }],
      game_state: null,
    });

    (overworld as any)._draw_ascii_overworld(textTarget);

    const [viewportLines, infoLines] = (textTarget.renderOverworldOverlay as jest.Mock).mock.calls[0];
    const viewportText = viewportLines.join("\n");
    const legendText = infoLines.join("\n");

    expect(viewportText).toContain("02 .  .  N> .  .  .  @v");
    expect(viewportText.match(/N/g)).toHaveLength(1);
    expect(legendText).toContain("N>=NPC facing right");
  });

  it("scrolls the ASCII viewport with the player like the canvas camera", () => {
    const textTarget = { renderOverworldOverlay: jest.fn() };
    const mapData = {
      mapName: "LONG_ROUTE",
      width: 12,
      height: 6,
      getMetatileAt: () => 0,
    };
    const tileset = {
      tilesetName: "TEST",
      metatiles: [{ collision: [0, 0, 0, 0] }],
    };

    class TestOverworld extends OverworldRenderingMixin {}
    const overworld = new TestOverworld() as OverworldRenderingMixin & Record<string, unknown>;
    Object.assign(overworld, {
      map: mapData,
      tileset,
      _map_events: { warps: [], coord_events: [], bg_events: [] },
      player_x: 39,
      player_y: 19,
      player_direction: "right",
      TILES_PER_COLLISION: 2,
      _text_ui_color: false,
      _ascii_overlay_cache_key: null,
      _ascii_overlay_cached_viewport: null,
      _ascii_overlay_cached_info: null,
      _ascii_overlay_last_npc_positions: [],
      _ascii_overlay_last_event_identity: null,
      _ascii_overlay_last_event_counts: null,
      _last_block_feedback: null,
      npcs: [],
      game_state: null,
    });

    (overworld as any)._draw_ascii_overworld(textTarget);

    const [viewportLines] = (textTarget.renderOverworldOverlay as jest.Mock).mock.calls[0];
    const viewportText = viewportLines.join("\n");

    expect(viewportLines[0]).toContain("04 05 06 07 08");
    expect(viewportText).toContain("09 ");
    expect(viewportText).toContain("@>");
    expect(viewportText).not.toContain("00 01 02 03 04");
  });

  it("scrolls the ASCII viewport across composite map segments", () => {
    const textTarget = { renderOverworldOverlay: jest.fn() };
    const mapData = {
      mapName: "BASE",
      width: 10,
      height: 10,
      getMetatileAt: () => 0,
    };
    const tileset = {
      tilesetName: "TEST",
      metatiles: [{ collision: [0, 0, 0, 0] }],
    };
    const farSegment = {
      name: "FAR",
      dest: [20 * 8, 20 * 8],
      map: {
        mapName: "FAR",
        width: 10,
        height: 10,
        getMetatileAt: () => 0,
      },
      tileset,
    };

    class TestOverworld extends OverworldRenderingMixin {}
    const overworld = new TestOverworld() as OverworldRenderingMixin & Record<string, unknown>;
    Object.assign(overworld, {
      map: mapData,
      tileset,
      _map_events: { warps: [], coord_events: [], bg_events: [] },
      player_x: 23,
      player_y: 23,
      player_direction: "down",
      TILES_PER_COLLISION: 1,
      _text_ui_color: false,
      _composite_origin: [0, 0],
      _composite_segments: [farSegment],
      _ascii_overlay_cache_key: null,
      _ascii_overlay_cached_viewport: null,
      _ascii_overlay_cached_info: null,
      _ascii_overlay_last_npc_positions: [],
      _ascii_overlay_last_event_identity: null,
      _ascii_overlay_last_event_counts: null,
      _last_block_feedback: null,
      npcs: [],
      game_state: null,
    });

    (overworld as any)._draw_ascii_overworld(textTarget);

    const [viewportLines] = (textTarget.renderOverworldOverlay as jest.Mock).mock.calls[0];
    const viewportText = viewportLines.join("\n");

    expect(viewportLines[0]).not.toContain("00 01 02 03 04");
    expect(viewportText).toContain("@v");
    expect(viewportText).toContain("23");
  });

  it("keeps a collision-scaled player visible when map metadata would otherwise clamp the viewport", () => {
    const textTarget = { renderOverworldOverlay: jest.fn() };
    const mapData = {
      mapName: "NEW_BARK_TOWN",
      width: 10,
      height: 6,
      getMetatileAt: () => 0,
    };
    const tileset = {
      tilesetName: "TEST",
      metatiles: [{ collision: [0, 0, 0, 0] }],
    };

    class TestOverworld extends OverworldRenderingMixin {}
    const overworld = new TestOverworld() as OverworldRenderingMixin & Record<string, unknown>;
    Object.assign(overworld, {
      map: mapData,
      tileset,
      _map_events: { warps: [], coord_events: [], bg_events: [] },
      player_x: 19,
      player_y: 25,
      player_direction: "down",
      TILES_PER_COLLISION: 2,
      _text_ui_color: false,
      _ascii_overlay_cache_key: null,
      _ascii_overlay_cached_viewport: null,
      _ascii_overlay_cached_info: null,
      _ascii_overlay_last_npc_positions: [],
      _ascii_overlay_last_event_identity: null,
      _ascii_overlay_last_event_counts: null,
      _last_block_feedback: null,
      npcs: [],
      game_state: null,
    });

    (overworld as any)._draw_ascii_overworld(textTarget);

    const [viewportLines] = (textTarget.renderOverworldOverlay as jest.Mock).mock.calls[0];
    const viewportText = viewportLines.join("\n");

    expect(viewportText).toContain("@v");
    expect(viewportText).toContain("12");
    expect(viewportText).not.toContain("00 #");
  });

  it("keeps the New Bark bottom edge player row visible at raw position 27,23", () => {
    const textTarget = { renderOverworldOverlay: jest.fn() };
    const mapData = {
      mapName: "NEW_BARK_TOWN",
      width: 10,
      height: 6,
      getMetatileAt: () => 0,
    };
    const tileset = {
      tilesetName: "TEST",
      metatiles: [{ collision: [0, 0, 0, 0] }],
    };

    class TestOverworld extends OverworldRenderingMixin {}
    const overworld = new TestOverworld() as OverworldRenderingMixin & Record<string, unknown>;
    Object.assign(overworld, {
      map: mapData,
      tileset,
      _map_events: { warps: [], coord_events: [], bg_events: [] },
      player_x: 27,
      player_y: 23,
      player_direction: "down",
      TILES_PER_COLLISION: 2,
      _text_ui_color: false,
      _ascii_overlay_cache_key: null,
      _ascii_overlay_cached_viewport: null,
      _ascii_overlay_cached_info: null,
      _ascii_overlay_last_npc_positions: [],
      _ascii_overlay_last_event_identity: null,
      _ascii_overlay_last_event_counts: null,
      _last_block_feedback: null,
      npcs: [],
      game_state: null,
    });

    (overworld as any)._draw_ascii_overworld(textTarget);

    const [viewportLines, infoLines] = (textTarget.renderOverworldOverlay as jest.Mock).mock.calls[0];
    const viewportText = viewportLines.join("\n");

    expect(infoLines).toContain("Pos: (13,11)");
    expect(viewportText).toContain("11");
    expect(viewportText).toContain("@v");
  });

  it("uses WRAM player coordinates for the ASCII viewport when renderer internals lag behind status", () => {
    const textTarget = { renderOverworldOverlay: jest.fn() };
    const mapData = {
      mapName: "NEW_BARK_TOWN",
      width: 10,
      height: 6,
      getMetatileAt: () => 0,
    };
    const tileset = {
      tilesetName: "TEST",
      metatiles: [{ collision: [0, 0, 0, 0] }],
    };

    class TestOverworld extends OverworldRenderingMixin {}
    const overworld = new TestOverworld() as OverworldRenderingMixin & Record<string, unknown>;
    Object.assign(overworld, {
      map: mapData,
      tileset,
      _map_events: { warps: [], coord_events: [], bg_events: [] },
      player_x: 7,
      player_y: 21,
      player_direction: "down",
      TILES_PER_COLLISION: 2,
      _text_ui_color: false,
      _ascii_overlay_cache_key: null,
      _ascii_overlay_cached_viewport: null,
      _ascii_overlay_cached_info: null,
      _ascii_overlay_last_npc_positions: [],
      _ascii_overlay_last_event_identity: null,
      _ascii_overlay_last_event_counts: null,
      _last_block_feedback: null,
      npcs: [],
      game_state: { wram: { wXCoord: 15, wYCoord: 31 } },
    });

    (overworld as any)._draw_ascii_overworld(textTarget);

    const [viewportLines, infoLines] = (textTarget.renderOverworldOverlay as jest.Mock).mock.calls[0];
    const viewportText = viewportLines.join("\n");

    expect(infoLines).toContain("Pos: (7,15)");
    expect(viewportText).toContain("15");
    expect(viewportText).toContain("@v");
    expect(viewportText).not.toContain("10 #");
  });

  it("gives berry trees and item balls distinct markers instead of NPC", () => {
    const textTarget = { renderOverworldOverlay: jest.fn() };
    const mapData = {
      mapName: "TEST_MAP",
      width: 2,
      height: 2,
      getMetatileAt: () => 0,
    };
    const tileset = {
      tilesetName: "TEST",
      metatiles: [{ collision: [0, 0, 0, 0] }],
    };

    class TestOverworld extends OverworldRenderingMixin {}
    const overworld = new TestOverworld() as OverworldRenderingMixin & Record<string, unknown>;
    Object.assign(overworld, {
      map: mapData,
      tileset,
      _map_events: { warps: [], coord_events: [], bg_events: [] },
      player_x: 7,
      player_y: 7,
      player_direction: "down",
      TILES_PER_COLLISION: 2,
      _text_ui_color: false,
      _ascii_overlay_cache_key: null,
      _ascii_overlay_cached_viewport: null,
      _ascii_overlay_cached_info: null,
      _ascii_overlay_last_npc_positions: [],
      _ascii_overlay_last_event_identity: null,
      _ascii_overlay_last_event_counts: null,
      _last_block_feedback: null,
      npcs: [
        { x: 1, y: 1, collisionStride: 2, direction: "down", event: { sprite: "SPRITE_FRUIT_TREE", object_type: "OBJECTTYPE_SCRIPT" } },
        { x: 5, y: 1, collisionStride: 2, direction: "down", event: { sprite: "SPRITE_POKE_BALL", object_type: "OBJECTTYPE_ITEMBALL" } },
      ],
      game_state: null,
    });

    (overworld as any)._draw_ascii_overworld(textTarget);

    const [viewportLines, infoLines] = (textTarget.renderOverworldOverlay as jest.Mock).mock.calls[0];
    const viewportText = viewportLines.join("\n");
    const legendText = infoLines.join("\n");

    expect(viewportText).toContain("B");
    expect(viewportText).toContain("I");
    expect(legendText).toContain("B=Berry tree");
    expect(legendText).toContain("I=Item ball");
    expect(legendText).not.toContain("B=Person");
  });

  it("gives vendors and healers distinct markers instead of generic NPCs", () => {
    const textTarget = { renderOverworldOverlay: jest.fn() };
    const mapData = {
      mapName: "TEST_MAP",
      width: 2,
      height: 2,
      getMetatileAt: () => 0,
    };
    const tileset = {
      tilesetName: "TEST",
      metatiles: [{ collision: [0, 0, 0, 0] }],
    };

    class TestOverworld extends OverworldRenderingMixin {}
    const overworld = new TestOverworld() as OverworldRenderingMixin & Record<string, unknown>;
    Object.assign(overworld, {
      map: mapData,
      tileset,
      _map_events: { warps: [], coord_events: [], bg_events: [] },
      player_x: 7,
      player_y: 7,
      player_direction: "down",
      TILES_PER_COLLISION: 2,
      _text_ui_color: false,
      _ascii_overlay_cache_key: null,
      _ascii_overlay_cached_viewport: null,
      _ascii_overlay_cached_info: null,
      _ascii_overlay_last_npc_positions: [],
      _ascii_overlay_last_event_identity: null,
      _ascii_overlay_last_event_counts: null,
      _last_block_feedback: null,
      npcs: [
        { x: 0, y: 0, direction: "down", event: { script: "PokecenterNurseScript", sprite: "SPRITE_NURSE" } },
        { x: 2, y: 0, direction: "left", event: { script: "MartClerkScript", sprite: "SPRITE_CLERK" } },
      ],
      game_state: null,
    });

    (overworld as any)._draw_ascii_overworld(textTarget);

    const [viewportLines, infoLines] = (textTarget.renderOverworldOverlay as jest.Mock).mock.calls[0];
    const viewportText = viewportLines.join("\n");
    const legendText = infoLines.join("\n");

    expect(viewportText).toContain("+");
    expect(viewportText).toContain("V");
    expect(legendText).toContain("+=Healer");
    expect(legendText).toContain("V=Vendor");
  });

  it("renders Pokecenter signs as signs instead of healers", () => {
    const textTarget = { renderOverworldOverlay: jest.fn() };
    const mapData = {
      mapName: "TEST_MAP",
      width: 2,
      height: 2,
      getMetatileAt: () => 0,
    };
    const tileset = {
      tilesetName: "TEST",
      metatiles: [{ collision: [0, 0, 0, 0] }],
    };

    class TestOverworld extends OverworldRenderingMixin {}
    const overworld = new TestOverworld() as OverworldRenderingMixin & Record<string, unknown>;
    Object.assign(overworld, {
      map: mapData,
      tileset,
      _map_events: {
        warps: [],
        coord_events: [],
        bg_events: [{ x: 0, y: 0, event_type: "BGEVENT_READ", script: "PokecenterSignScript" }],
      },
      player_x: 7,
      player_y: 7,
      player_direction: "down",
      TILES_PER_COLLISION: 2,
      _text_ui_color: false,
      _ascii_overlay_cache_key: null,
      _ascii_overlay_cached_viewport: null,
      _ascii_overlay_cached_info: null,
      _ascii_overlay_last_npc_positions: [],
      _ascii_overlay_last_event_identity: null,
      _ascii_overlay_last_event_counts: null,
      _last_block_feedback: null,
      npcs: [],
      game_state: null,
    });

    (overworld as any)._draw_ascii_overworld(textTarget);

    const [viewportLines, infoLines] = (textTarget.renderOverworldOverlay as jest.Mock).mock.calls[0];
    const viewportText = viewportLines.join("\n");
    const legendText = infoLines.join("\n");

    expect(viewportText).toContain("S");
    expect(legendText).toContain("S=Sign");
    expect(legendText).not.toContain("+=Healer");
  });

  it("renders Elm and the three starter poke balls as separate ascii cells in Elm's Lab", () => {
    const textTarget = { renderOverworldOverlay: jest.fn() };
    const mapData = {
      mapName: "ELMS_LAB",
      width: 10,
      height: 9,
      getMetatileAt: () => 0,
    };
    const tileset = {
      tilesetName: "TEST",
      metatiles: [{ collision: [0, 0, 0, 0] }],
    };

    class TestOverworld extends OverworldRenderingMixin {}
    const overworld = new TestOverworld() as OverworldRenderingMixin & Record<string, unknown>;
    Object.assign(overworld, {
      map: mapData,
      tileset,
      _map_events: { warps: [], coord_events: [], bg_events: [] },
      player_x: 19,
      player_y: 15,
      player_direction: "down",
      TILES_PER_COLLISION: 2,
      _text_ui_color: false,
      _ascii_overlay_cache_key: null,
      _ascii_overlay_cached_viewport: null,
      _ascii_overlay_cached_info: null,
      _ascii_overlay_last_npc_positions: [],
      _ascii_overlay_last_event_identity: null,
      _ascii_overlay_last_event_counts: null,
      _last_block_feedback: null,
      npcs: [
        { x: 11, y: 5, collisionStride: 2, direction: "down", event: { sprite: "SPRITE_ELM", object_type: "OBJECTTYPE_SCRIPT" } },
        { x: 13, y: 7, collisionStride: 2, direction: "down", event: { sprite: "SPRITE_POKE_BALL", object_type: "OBJECTTYPE_SCRIPT" } },
        { x: 15, y: 7, collisionStride: 2, direction: "down", event: { sprite: "SPRITE_POKE_BALL", object_type: "OBJECTTYPE_SCRIPT" } },
        { x: 17, y: 7, collisionStride: 2, direction: "down", event: { sprite: "SPRITE_POKE_BALL", object_type: "OBJECTTYPE_SCRIPT" } },
      ],
      game_state: null,
    });

    (overworld as any)._draw_ascii_overworld(textTarget);

    const [viewportLines, infoLines] = (textTarget.renderOverworldOverlay as jest.Mock).mock.calls[0];
    const viewportText = viewportLines.join("\n");
    const legendText = infoLines.join("\n");

    expect(viewportText).toContain("02 .  .  .  .  .  Nv");
    expect(viewportText).toContain("03 .  .  .  .  .  .  I  I  I");
    expect(viewportText).toContain("07 .  .  .  .  .  .  .  .  .  @v");
    expect(legendText).toContain("N=Person");
    expect(legendText).toContain("I=Item ball");
  });

  it("does not render walkable nonzero land permissions as blocked", () => {
    const textTarget = { renderOverworldOverlay: jest.fn() };
    const walkRight = resolveCollisionValue("WALK_RIGHT");
    const mapData = {
      mapName: "TEST_MAP",
      width: 2,
      height: 1,
      getMetatileAt: () => 0,
    };
    const tileset = {
      tilesetName: "TEST",
      metatiles: [{ collision: [walkRight, walkRight, walkRight, walkRight] }],
    };

    class TestOverworld extends OverworldRenderingMixin {}
    const overworld = new TestOverworld() as OverworldRenderingMixin & Record<string, unknown>;
    Object.assign(overworld, {
      map: mapData,
      tileset,
      _map_events: { warps: [], coord_events: [], bg_events: [] },
      player_x: 1,
      player_y: 1,
      player_direction: "right",
      TILES_PER_COLLISION: 2,
      _text_ui_color: false,
      _ascii_overlay_cache_key: null,
      _ascii_overlay_cached_viewport: null,
      _ascii_overlay_cached_info: null,
      _ascii_overlay_last_npc_positions: [],
      _ascii_overlay_last_event_identity: null,
      _ascii_overlay_last_event_counts: null,
      _last_block_feedback: null,
      npcs: [],
      game_state: null,
    });

    (overworld as any)._draw_ascii_overworld(textTarget);

    const [viewportLines, infoLines] = (textTarget.renderOverworldOverlay as jest.Mock).mock.calls[0];
    const viewportText = viewportLines.join("\n");
    const legendText = infoLines.join("\n");

    expect(viewportText).not.toContain("#");
    expect(legendText).not.toContain("#=Blocked");
    expect(viewportText).toContain(">");
  });

  it("adds dialogue context to yes/no prompt overlay lines", () => {
    const textTarget = { renderOverworldOverlay: jest.fn() };
    const mapData = {
      mapName: "TEST_MAP",
      width: 2,
      height: 1,
      getMetatileAt: () => 0,
    };
    const tileset = {
      tilesetName: "TEST",
      metatiles: [{ collision: [0, 0, 0, 0] }],
    };

    class TestOverworld extends OverworldRenderingMixin {}
    const overworld = new TestOverworld() as OverworldRenderingMixin & Record<string, unknown>;
    Object.assign(overworld, {
      map: mapData,
      tileset,
      _map_events: { warps: [], coord_events: [], bg_events: [] },
      player_x: 1,
      player_y: 1,
      player_direction: "down",
      TILES_PER_COLLISION: 2,
      _text_ui_color: false,
      _ascii_overlay_cache_key: null,
      _ascii_overlay_cached_viewport: null,
      _ascii_overlay_cached_info: null,
      _ascii_overlay_last_npc_positions: [],
      _ascii_overlay_last_event_identity: null,
      _ascii_overlay_last_event_counts: null,
      _last_block_feedback: null,
      npcs: [],
      game_state: null,
      dialogue: {
        active: true,
        window: { visible_text: "Use the switch?" },
        _yes_no_prompt: { selection: 0 },
        waiting_for_input: true,
        pending_waits: 0,
        pending_text_count: 0,
      },
    });

    (overworld as any)._draw_ascii_overworld(textTarget);

    const overlayOptions = (textTarget.renderOverworldOverlay as jest.Mock).mock.calls[0][2];

    expect(overlayOptions.promptLines).toEqual(["Use the switch?", ">YES", "  NO"]);
  });

  it("uses the full current dialogue page for text snapshots before typewriter reveal completes", () => {
    const textTarget = { renderOverworldOverlay: jest.fn() };
    const mapData = {
      mapName: "TEST_MAP",
      width: 1,
      height: 1,
      getMetatileAt: () => 0,
    };
    const tileset = {
      tilesetName: "TEST",
      metatiles: [{ collision: [0, 0, 0, 0] }],
    };

    class TestOverworld extends OverworldRenderingMixin {}
    const overworld = new TestOverworld() as OverworldRenderingMixin & Record<string, unknown>;
    Object.assign(overworld, {
      map: mapData,
      tileset,
      _map_events: { warps: [], coord_events: [], bg_events: [] },
      player_x: 1,
      player_y: 1,
      player_direction: "down",
      TILES_PER_COLLISION: 2,
      _text_ui_color: false,
      _ascii_overlay_cache_key: null,
      _ascii_overlay_cached_viewport: null,
      _ascii_overlay_cached_info: null,
      _ascii_overlay_last_npc_positions: [],
      _ascii_overlay_last_event_identity: null,
      _ascii_overlay_last_event_counts: null,
      _last_block_feedback: null,
      npcs: [],
      game_state: null,
      dialogue: {
        active: true,
        current_text: "KIMONO GIRL: You have lovely #MON.",
        window: {
          visible_text: "K",
          current_page_text: "KIMONO GIRL: You have lovely #MON.",
        },
        _yes_no_prompt: null,
        waiting_for_input: true,
        pending_waits: 1,
        pending_text_count: 0,
      },
    });

    (overworld as any)._draw_ascii_overworld(textTarget);

    const overlayOptions = (textTarget.renderOverworldOverlay as jest.Mock).mock.calls[0][2];

    expect(overlayOptions.dialogueLines).toContain("KIMONO GIRL: You have lovely #MON.");
  });

  it("renders full dialogue text in text snapshots instead of the current Game Boy page", () => {
    const textTarget = { renderOverworldOverlay: jest.fn() };
    const mapData = {
      mapName: "TEST_MAP",
      width: 1,
      height: 1,
      getMetatileAt: () => 0,
    };
    const tileset = {
      tilesetName: "TEST",
      metatiles: [{ collision: [0, 0, 0, 0] }],
    };

    class TestOverworld extends OverworldRenderingMixin {}
    const overworld = new TestOverworld() as OverworldRenderingMixin & Record<string, unknown>;
    Object.assign(overworld, {
      map: mapData,
      tileset,
      _map_events: { warps: [], coord_events: [], bg_events: [] },
      player_x: 1,
      player_y: 1,
      player_direction: "down",
      TILES_PER_COLLISION: 2,
      _text_ui_color: false,
      _ascii_overlay_cache_key: null,
      _ascii_overlay_cached_viewport: null,
      _ascii_overlay_cached_info: null,
      _ascii_overlay_last_npc_positions: [],
      _ascii_overlay_last_event_identity: null,
      _ascii_overlay_last_event_counts: null,
      _last_block_feedback: null,
      npcs: [],
      game_state: null,
      dialogue: {
        active: true,
        current_text: "Although you can't\nsee it from here,\n\nCIANWOOD is across\nthe sea.",
        window: {
          visible_text: "Although you can't\nsee",
          current_page_text: "Although you can't\nsee it from here,",
        },
        _yes_no_prompt: null,
        waiting_for_input: true,
        pending_waits: 1,
        pending_text_count: 0,
      },
    });

    (overworld as any)._draw_ascii_overworld(textTarget);

    const overlayOptions = (textTarget.renderOverworldOverlay as jest.Mock).mock.calls[0][2];

    expect(overlayOptions.dialogueLines).toEqual([
      "Although you can't",
      "see it from here,",
      "CIANWOOD is across",
      "the sea.",
    ]);
  });

  it("labels warp doors for centers, gyms, and marts", () => {
    const textTarget = { renderOverworldOverlay: jest.fn() };
    const mapData = {
      mapName: "TEST_MAP",
      width: 2,
      height: 1,
      getMetatileAt: () => 0,
    };
    const tileset = {
      tilesetName: "TEST",
      metatiles: [{ collision: [0, 0, 0, 0] }],
    };

    class TestOverworld extends OverworldRenderingMixin {}
    const overworld = new TestOverworld() as OverworldRenderingMixin & Record<string, unknown>;
    Object.assign(overworld, {
      map: mapData,
      tileset,
      _map_events: {
        warps: [
          { x: 0, y: 0, target_map: "Pokecenter1F" },
          { x: 1, y: 0, target_map: "VioletGym" },
          { x: 2, y: 0, target_map: "GoldenrodMart1F" },
          { x: 3, y: 0, target_map: "PlayerHouse1F" },
        ],
        coord_events: [],
        bg_events: [],
      },
      player_x: 7,
      player_y: 7,
      player_direction: "down",
      TILES_PER_COLLISION: 2,
      _text_ui_color: false,
      _ascii_overlay_cache_key: null,
      _ascii_overlay_cached_viewport: null,
      _ascii_overlay_cached_info: null,
      _ascii_overlay_last_npc_positions: [],
      _ascii_overlay_last_event_identity: null,
      _ascii_overlay_last_event_counts: null,
      _last_block_feedback: null,
      npcs: [],
      game_state: null,
    });

    (overworld as any)._draw_ascii_overworld(textTarget);

    const [viewportLines] = (textTarget.renderOverworldOverlay as jest.Mock).mock.calls[0];
    const viewportText = viewportLines.join("\n");

    expect(viewportText).toContain("DP");
    expect(viewportText).toContain("DG");
    expect(viewportText).toContain("DM");
  });

  it("renders ledge glyphs on the cliff face instead of the traversable hop tile", () => {
    const textTarget = { renderOverworldOverlay: jest.fn() };
    const wall = resolveCollisionValue("WALL");
    const hopDown = resolveCollisionValue("HOP_DOWN");
    const hopRight = resolveCollisionValue("HOP_RIGHT");
    const mapData = {
      mapName: "TEST_MAP",
      width: 3,
      height: 2,
      getMetatileAt: (x: number, y: number) => {
        if (y === 0 && x === 0) return 0;
        if (y === 0 && x === 1) return 1;
        return 2;
      },
    };
    const tileset = {
      tilesetName: "TEST",
      metatiles: [
        { collision: [hopDown, hopDown, wall, wall] },
        { collision: [hopRight, wall, hopRight, wall] },
        { collision: [0, 0, 0, 0] },
      ],
    };

    class TestOverworld extends OverworldRenderingMixin {}
    const overworld = new TestOverworld() as OverworldRenderingMixin & Record<string, unknown>;
    Object.assign(overworld, {
      map: mapData,
      tileset,
      _map_events: { warps: [], coord_events: [], bg_events: [] },
      player_x: 7,
      player_y: 7,
      player_direction: "down",
      TILES_PER_COLLISION: 2,
      _text_ui_color: false,
      _ascii_overlay_cache_key: null,
      _ascii_overlay_cached_viewport: null,
      _ascii_overlay_cached_info: null,
      _ascii_overlay_last_npc_positions: [],
      _ascii_overlay_last_event_identity: null,
      _ascii_overlay_last_event_counts: null,
      _last_block_feedback: null,
      npcs: [],
      game_state: null,
    });

    (overworld as any)._draw_ascii_overworld(textTarget);

    const [viewportLines, infoLines] = (textTarget.renderOverworldOverlay as jest.Mock).mock.calls[0];
    const viewportText = viewportLines.join("\n");
    const legendText = infoLines.join("\n");

    expect(viewportText).toMatch(/\.\s+\.\s+r/);
    expect(viewportText).toMatch(/d\s+d\s+\./);
    expect(viewportText).not.toMatch(/d\s+r\s+\./);
    expect(viewportText).not.toContain(" v ");
    expect(legendText).toContain("d=Ledge pass down");
    expect(legendText).toContain("r=Ledge pass right");
    expect(legendText).not.toContain("D=Ledge pass down");
  });

  it("keeps Dance Theater's stage landing row rendered as passable floor", () => {
    const textTarget = { renderOverworldOverlay: jest.fn() };
    const floor = resolveCollisionValue("FLOOR");
    const hopDown = resolveCollisionValue("HOP_DOWN");
    const danceTheaterBlocks = [
      0x2d, 0x2d, 0x2d, 0x2d, 0x2d, 0x2d,
      0x2c, 0x2c, 0x2c, 0x2c, 0x2c, 0x2c,
      0x2e, 0x30, 0x30, 0x30, 0x30, 0x2f,
      0x10, 0x10, 0x15, 0x11, 0x0e, 0x0e,
      0x10, 0x10, 0x04, 0x04, 0x0e, 0x0e,
      0x10, 0x10, 0x04, 0x04, 0x0e, 0x0e,
      0x05, 0x2a, 0x06, 0x07, 0x2b, 0x2a,
    ];
    const metatiles = Array.from({ length: 0x31 }, () => ({
      collision: [floor, floor, floor, floor],
    }));
    metatiles[0x2c] = { collision: [floor, floor, hopDown, hopDown] };
    const mapData = {
      mapName: "DanceTheater",
      width: 6,
      height: 7,
      getMetatileAt: (x: number, y: number) => danceTheaterBlocks[y * 6 + x] ?? 0,
    };
    const tileset = {
      tilesetName: "traditional_house",
      metatiles,
    };

    class TestOverworld extends OverworldRenderingMixin {}
    const overworld = new TestOverworld() as OverworldRenderingMixin & Record<string, unknown>;
    Object.assign(overworld, {
      map: mapData,
      tileset,
      _map_events: { warps: [], coord_events: [], bg_events: [] },
      player_x: 7,
      player_y: 15,
      player_direction: "down",
      TILES_PER_COLLISION: 2,
      _text_ui_color: false,
      _ascii_overlay_cache_key: null,
      _ascii_overlay_cached_viewport: null,
      _ascii_overlay_cached_info: null,
      _ascii_overlay_last_npc_positions: [],
      _ascii_overlay_last_event_identity: null,
      _ascii_overlay_last_event_counts: null,
      _last_block_feedback: null,
      npcs: [],
      game_state: null,
    });

    (overworld as any)._draw_ascii_overworld(textTarget);

    const [viewportLines, infoLines] = (textTarget.renderOverworldOverlay as jest.Mock).mock.calls[0];
    const landingRow = viewportLines.find((line) => line.startsWith("04 "));

    expect(landingRow).toBeDefined();
    expect(landingRow).toContain(".  .  .  .  .  .  .  .  .  .  .  .");
    expect(landingRow).not.toContain("d");
    expect(infoLines.join("\n")).not.toContain("d=Ledge pass down");
  });

  it("does not render hidden-item BG events in the ASCII grid or legend", () => {
    const textTarget = { renderOverworldOverlay: jest.fn() };
    const mapData = {
      mapName: "TEST_MAP",
      width: 1,
      height: 1,
      getMetatileAt: () => 0,
    };
    const tileset = {
      tilesetName: "TEST",
      metatiles: [{ collision: [0, 0, 0, 0] }],
    };

    class TestOverworld extends OverworldRenderingMixin {}
    const overworld = new TestOverworld() as OverworldRenderingMixin & Record<string, unknown>;
    Object.assign(overworld, {
      map: mapData,
      tileset,
      _map_events: {
        warps: [],
        coord_events: [],
        bg_events: [{ x: 1, y: 0, event_type: "SIGNPOST_ITEM", script: "HiddenItemScript" }],
      },
      player_x: 2,
      player_y: 2,
      player_direction: "down",
      TILES_PER_COLLISION: 2,
      _text_ui_color: false,
      _ascii_overlay_cache_key: null,
      _ascii_overlay_cached_viewport: null,
      _ascii_overlay_cached_info: null,
      _ascii_overlay_last_npc_positions: [],
      _ascii_overlay_last_event_identity: null,
      _ascii_overlay_last_event_counts: null,
      _last_block_feedback: null,
      npcs: [],
      game_state: null,
    });

    (overworld as any)._draw_ascii_overworld(textTarget);

    const [viewportLines, infoLines] = (textTarget.renderOverworldOverlay as jest.Mock).mock.calls[0];
    const viewportText = viewportLines.join("\n");
    const infoText = infoLines.join(" ");

    expect(viewportText).not.toContain("b");
    expect(infoText).not.toContain("b=BG event");
  });

  it("renders visible BG hotspots with their map-info tokens", () => {
    const textTarget = { renderOverworldOverlay: jest.fn() };
    const mapData = {
      mapName: "TEST_MAP",
      width: 3,
      height: 1,
      getMetatileAt: () => 0,
    };
    const tileset = {
      tilesetName: "TEST",
      metatiles: [{ collision: [0, 0, 0, 0] }],
    };

    class TestOverworld extends OverworldRenderingMixin {}
    const overworld = new TestOverworld() as OverworldRenderingMixin & Record<string, unknown>;
    Object.assign(overworld, {
      map: mapData,
      tileset,
      _map_events: {
        warps: [],
        coord_events: [],
        bg_events: [
          { x: 0, y: 0, event_type: "BGEVENT_UP", script: "PlayersHousePCScript" },
          { x: 1, y: 0, event_type: "BGEVENT_READ", script: "PlayersHouseBookshelfScript" },
          { x: 2, y: 0, event_type: "BGEVENT_READ", script: "PlayersHousePosterScript" },
        ],
      },
      player_x: 11,
      player_y: 3,
      player_direction: "down",
      TILES_PER_COLLISION: 2,
      _text_ui_color: false,
      _ascii_overlay_cache_key: null,
      _ascii_overlay_cached_viewport: null,
      _ascii_overlay_cached_info: null,
      _ascii_overlay_last_npc_positions: [],
      _ascii_overlay_last_event_identity: null,
      _ascii_overlay_last_event_counts: null,
      _last_block_feedback: null,
      npcs: [],
      game_state: null,
    });

    (overworld as any)._draw_ascii_overworld(textTarget);

    const [viewportLines, infoLines] = (textTarget.renderOverworldOverlay as jest.Mock).mock.calls[0];
    const viewportText = viewportLines.join("\n");
    const legendText = infoLines.join("\n");

    expect(viewportText).toContain("00 P  B  S");
    expect(viewportText).toContain("@v");
    expect(legendText).toContain("P=PC/Center");
    expect(legendText).toContain("B=Berry tree/Bookshelf");
    expect(legendText).toContain("S=Sign");
  });

  it("renders interactive blockers distinctly from plain walls", () => {
    const textTarget = { renderOverworldOverlay: jest.fn() };
    const counter = resolveCollisionValue("COUNTER");
    const wall = resolveCollisionValue("WALL");
    const mapData = {
      mapName: "TEST_MAP",
      width: 2,
      height: 1,
      getMetatileAt: (x: number) => (x === 0 ? 0 : 1),
    };
    const tileset = {
      tilesetName: "TEST",
      metatiles: [
        { collision: [counter, counter, counter, counter] },
        { collision: [wall, wall, wall, wall] },
      ],
    };

    class TestOverworld extends OverworldRenderingMixin {}
    const overworld = new TestOverworld() as OverworldRenderingMixin & Record<string, unknown>;
    Object.assign(overworld, {
      map: mapData,
      tileset,
      _map_events: { warps: [], coord_events: [], bg_events: [] },
      player_x: 1,
      player_y: 1,
      player_direction: "right",
      TILES_PER_COLLISION: 2,
      _text_ui_color: false,
      _ascii_overlay_cache_key: null,
      _ascii_overlay_cached_viewport: null,
      _ascii_overlay_cached_info: null,
      _ascii_overlay_last_npc_positions: [],
      _ascii_overlay_last_event_identity: null,
      _ascii_overlay_last_event_counts: null,
      _last_block_feedback: null,
      npcs: [],
      game_state: null,
    });

    (overworld as any)._draw_ascii_overworld(textTarget);

    const [viewportLines, infoLines] = (textTarget.renderOverworldOverlay as jest.Mock).mock.calls[0];
    const viewportText = viewportLines.join("\n");
    const legendText = infoLines.join("\n");

    expect(viewportText).toContain("T  #");
    expect(legendText).toContain("T=Counter");
    expect(legendText).toContain("#=Blocked");
  });

  it("does not render coord events in the ASCII grid or legend", () => {
    const textTarget = { renderOverworldOverlay: jest.fn() };
    const mapData = {
      mapName: "TEST_MAP",
      width: 1,
      height: 1,
      getMetatileAt: () => 0,
    };
    const tileset = {
      tilesetName: "TEST",
      metatiles: [{ collision: [0, 0, 0, 0] }],
    };

    class TestOverworld extends OverworldRenderingMixin {}
    const overworld = new TestOverworld() as OverworldRenderingMixin & Record<string, unknown>;
    Object.assign(overworld, {
      map: mapData,
      tileset,
      _map_events: {
        warps: [],
        coord_events: [{ x: 1, y: 0, scene_id: "SCENE_TEST", script_name: "TestCoordEvent" }],
        bg_events: [],
      },
      player_x: 2,
      player_y: 2,
      player_direction: "down",
      TILES_PER_COLLISION: 2,
      _text_ui_color: false,
      _ascii_overlay_cache_key: null,
      _ascii_overlay_cached_viewport: null,
      _ascii_overlay_cached_info: null,
      _ascii_overlay_last_npc_positions: [],
      _ascii_overlay_last_event_identity: null,
      _ascii_overlay_last_event_counts: null,
      _last_block_feedback: null,
      npcs: [],
      game_state: null,
    });

    (overworld as any)._draw_ascii_overworld(textTarget);

    const [viewportLines, infoLines] = (textTarget.renderOverworldOverlay as jest.Mock).mock.calls[0];
    const viewportText = viewportLines.join("\n");
    const infoText = infoLines.join(" ");

    expect(viewportText).not.toContain("E");
    expect(infoText).not.toContain("E=Coord event");
  });

  it("allocates enough crowd marker slots for 500 online entities", () => {
    const positions = __test__buildCrowdMarkerPositions(500, 160, 144, 2);
    expect(positions).toHaveLength(500);
    expect(positions[0]).toEqual([0, 0]);
    expect(positions[499][1]).toBeGreaterThanOrEqual(0);
  });

  it("renders only in-frame remote players in normal multiplayer view", () => {
    class TestOverworld extends OverworldRenderingMixin {}
    const instance = new TestOverworld() as OverworldRenderingMixin & Record<string, unknown>;

    const screen = {
      fill: jest.fn(),
      get_width: jest.fn(() => 160),
      get_height: jest.fn(() => 144),
      blit: jest.fn(),
    };
    const mapSurface = {
      get_size: () => [512, 512] as [number, number],
    };
    const playerSprite = { get_width: () => 16, get_height: () => 16 };
    const remoteSprite = { get_width: () => 16, get_height: () => 16 };

    Object.assign(instance, {
      ui: null,
      screen,
      _composite_surface: mapSurface,
      map_surface: null,
      _composite_origin: [0, 0],
      _composite_priority_surface: null,
      priority_surface: null,
      _field_move_animation_renderer: null,
      _active_emotes: new Map(),
      _grass_rustle: null,
      _phone_call_overlay: null,
      dialogue: null,
      _town_map_overlay: null,
      _egg_hatch_animation: null,
      _map_sign: null,
      pokepic_overlay: null,
      _fade_alpha: 0,
      _poison_overlay_alpha: 0,
      _debug_sightlines: false,
      _multiplayer_remote_render_enabled: true,
      _multiplayer_remote_crowd_view: false,
      _multiplayer_remote_players: [
        {
          userId: "remote-1",
          playerName: "Remote 1",
          entityType: "player",
          mapName: "TestMap",
          tileX: 10,
          tileY: 10,
          direction: "right",
          updatedAtMs: 1,
        },
        {
          userId: "remote-2",
          playerName: "Remote 2",
          entityType: "player",
          mapName: "OtherMap",
          tileX: 10,
          tileY: 10,
          direction: "right",
          updatedAtMs: 1,
        },
        {
          userId: "remote-3",
          playerName: "Remote 3",
          entityType: "player",
          mapName: "TestMap",
          tileX: 200,
          tileY: 200,
          direction: "right",
          updatedAtMs: 1,
        },
      ],
      current_map_name: "TestMap",
      player_animations: {
        down: { currentFrame: playerSprite },
        right: { currentFrame: remoteSprite },
      },
      player_direction: "down",
      player_x: 10,
      player_y: 10,
      player_px_x: 72,
      player_px_y: 72,
      target_px_x: 72,
      target_px_y: 72,
      player_object: null,
      npcs: [],
      _npc_pixel_position: () => [0, 0],
      TILES_PER_COLLISION: 2,
    });

    instance.draw();

    expect(screen.blit).toHaveBeenCalledWith(mapSurface, [0, 0], expect.anything());
    expect(screen.blit).toHaveBeenCalledWith(remoteSprite, expect.any(Array), undefined);
    expect(screen.blit).toHaveBeenCalledWith(playerSprite, expect.any(Array), undefined);
    expect(screen.blit).toHaveBeenCalledTimes(3);
  });

  it("draws the priority plane after sprites even when the player is not on grass", () => {
    class TestOverworld extends OverworldRenderingMixin {}
    const instance = new TestOverworld() as OverworldRenderingMixin & Record<string, unknown>;

    const screen = {
      fill: jest.fn(),
      get_width: jest.fn(() => 160),
      get_height: jest.fn(() => 144),
      blit: jest.fn(),
    };
    const mapSurface = {
      get_size: () => [160, 144] as [number, number],
    };
    const prioritySurface = {};
    const playerSprite = { get_width: () => 16, get_height: () => 16 };

    Object.assign(instance, {
      ui: null,
      screen,
      _composite_surface: mapSurface,
      map_surface: null,
      _composite_origin: [0, 0],
      _composite_priority_surface: prioritySurface,
      priority_surface: null,
      _field_move_animation_renderer: null,
      _active_emotes: new Map(),
      _grass_rustle: null,
      _phone_call_overlay: null,
      dialogue: null,
      _town_map_overlay: null,
      _egg_hatch_animation: null,
      _map_sign: null,
      pokepic_overlay: null,
      _fade_alpha: 0,
      _poison_overlay_alpha: 0,
      _debug_sightlines: false,
      _multiplayer_remote_render_enabled: false,
      _multiplayer_remote_players: [],
      current_map_name: "TestMap",
      player_animations: {
        down: { currentFrame: playerSprite },
      },
      player_direction: "down",
      player_x: 10,
      player_y: 10,
      player_px_x: 72,
      player_px_y: 72,
      target_px_x: 72,
      target_px_y: 72,
      player_object: null,
      npcs: [],
      _npc_pixel_position: () => [0, 0],
      TILES_PER_COLLISION: 2,
    });

    instance.draw();

    expect(screen.blit).toHaveBeenNthCalledWith(1, mapSurface, [0, 0], expect.anything());
    expect(screen.blit).toHaveBeenNthCalledWith(2, playerSprite, expect.any(Array), undefined);
    expect(screen.blit).toHaveBeenNthCalledWith(3, prioritySurface, [0, 0], expect.anything());
  });

  it("draws item balls after the priority plane so table priority tiles do not cover them", () => {
    class TestOverworld extends OverworldRenderingMixin {}
    const instance = new TestOverworld() as OverworldRenderingMixin & Record<string, unknown>;

    const screen = {
      fill: jest.fn(),
      get_width: jest.fn(() => 160),
      get_height: jest.fn(() => 144),
      blit: jest.fn(),
    };
    const mapSurface = {
      get_size: () => [160, 144] as [number, number],
    };
    const prioritySurface = {};
    const playerSprite = { get_width: () => 16, get_height: () => 16 };
    const itemBallSprite = { get_width: () => 16, get_height: () => 16 };

    Object.assign(instance, {
      ui: null,
      screen,
      _composite_surface: mapSurface,
      map_surface: null,
      _composite_origin: [0, 0],
      _composite_priority_surface: prioritySurface,
      priority_surface: null,
      _field_move_animation_renderer: null,
      _active_emotes: new Map(),
      _grass_rustle: null,
      _phone_call_overlay: null,
      dialogue: null,
      _town_map_overlay: null,
      _egg_hatch_animation: null,
      _map_sign: null,
      pokepic_overlay: null,
      _fade_alpha: 0,
      _poison_overlay_alpha: 0,
      _debug_sightlines: false,
      _multiplayer_remote_render_enabled: false,
      _multiplayer_remote_players: [],
      current_map_name: "ElmsLab",
      player_animations: {
        down: { currentFrame: playerSprite },
      },
      player_direction: "down",
      player_x: 18,
      player_y: 14,
      player_px_x: 136,
      player_px_y: 104,
      target_px_x: 136,
      target_px_y: 104,
      player_object: null,
      npcs: [
        {
          x: 13,
          y: 7,
          direction: "down",
          animations: { down: { currentFrame: itemBallSprite } },
          event: { sprite: "SPRITE_POKE_BALL", object_type: "OBJECTTYPE_SCRIPT" },
        },
      ],
      _npc_pixel_position: () => [96, 48],
      TILES_PER_COLLISION: 2,
    });

    instance.draw();

    expect(screen.blit).toHaveBeenNthCalledWith(1, mapSurface, [0, 0], expect.anything());
    expect(screen.blit).toHaveBeenNthCalledWith(2, playerSprite, expect.any(Array), undefined);
    expect(screen.blit).toHaveBeenNthCalledWith(3, prioritySurface, [0, 0], expect.anything());
    expect(screen.blit).toHaveBeenNthCalledWith(4, itemBallSprite, expect.any(Array), undefined);
  });

  it("draws item balls in grass before the priority plane so grass covers their lower pixels", () => {
    class TestOverworld extends OverworldRenderingMixin {}
    const instance = new TestOverworld() as OverworldRenderingMixin & Record<string, unknown>;

    const screen = {
      fill: jest.fn(),
      get_width: jest.fn(() => 160),
      get_height: jest.fn(() => 144),
      blit: jest.fn(),
    };
    const mapSurface = {
      get_size: () => [160, 144] as [number, number],
    };
    const prioritySurface = {};
    const playerSprite = { get_width: () => 16, get_height: () => 16 };
    const itemBallSprite = { get_width: () => 16, get_height: () => 16 };

    Object.assign(instance, {
      ui: null,
      screen,
      _composite_surface: mapSurface,
      map_surface: null,
      _composite_origin: [0, 0],
      _composite_priority_surface: prioritySurface,
      priority_surface: null,
      _field_move_animation_renderer: null,
      _active_emotes: new Map(),
      _grass_rustle: null,
      _phone_call_overlay: null,
      dialogue: null,
      _town_map_overlay: null,
      _egg_hatch_animation: null,
      _map_sign: null,
      pokepic_overlay: null,
      _fade_alpha: 0,
      _poison_overlay_alpha: 0,
      _debug_sightlines: false,
      _multiplayer_remote_render_enabled: false,
      _multiplayer_remote_players: [],
      current_map_name: "Route29",
      player_animations: {
        down: { currentFrame: playerSprite },
      },
      player_direction: "down",
      player_x: 18,
      player_y: 14,
      player_px_x: 136,
      player_px_y: 104,
      target_px_x: 136,
      target_px_y: 104,
      player_object: null,
      npcs: [
        {
          x: 13,
          y: 7,
          direction: "down",
          overhead: true,
          animations: { down: { currentFrame: itemBallSprite } },
          event: { sprite: "SPRITE_POKE_BALL", object_type: "OBJECTTYPE_ITEMBALL" },
        },
      ],
      _npc_pixel_position: () => [96, 48],
      TILES_PER_COLLISION: 2,
    });

    instance.draw();

    expect(screen.blit).toHaveBeenNthCalledWith(1, mapSurface, [0, 0], expect.anything());
    expect(screen.blit).toHaveBeenNthCalledWith(2, itemBallSprite, expect.any(Array), undefined);
    expect(screen.blit).toHaveBeenNthCalledWith(3, playerSprite, expect.any(Array), undefined);
    expect(screen.blit).toHaveBeenNthCalledWith(4, prioritySurface, [0, 0], expect.anything());
  });

  it("renders 500 crowd markers plus base world and player sprite", () => {
    class TestOverworld extends OverworldRenderingMixin {}
    const instance = new TestOverworld() as OverworldRenderingMixin & Record<string, unknown>;

    const screen = {
      fill: jest.fn(),
      get_width: jest.fn(() => 160),
      get_height: jest.fn(() => 144),
      blit: jest.fn(),
    };
    const mapSurface = {
      get_size: () => [160, 144] as [number, number],
    };
    const playerSprite = { get_width: () => 16, get_height: () => 16 };
    const remotePlayers = Array.from({ length: 500 }, (_, index) => ({
      userId: `remote-${index}`,
      playerName: `Remote ${index}`,
      entityType: index % 2 === 0 ? "player" : "ai",
      mapName: "TestMap",
      tileX: 10,
      tileY: 10,
      direction: "down" as const,
      updatedAtMs: index,
    }));

    Object.assign(instance, {
      ui: null,
      screen,
      _composite_surface: mapSurface,
      map_surface: null,
      _composite_origin: [0, 0],
      _composite_priority_surface: null,
      priority_surface: null,
      _field_move_animation_renderer: null,
      _active_emotes: new Map(),
      _grass_rustle: null,
      _phone_call_overlay: null,
      dialogue: null,
      _town_map_overlay: null,
      _egg_hatch_animation: null,
      _map_sign: null,
      pokepic_overlay: null,
      _fade_alpha: 0,
      _poison_overlay_alpha: 0,
      _debug_sightlines: false,
      _multiplayer_remote_render_enabled: true,
      _multiplayer_remote_crowd_view: true,
      _multiplayer_remote_players: remotePlayers,
      current_map_name: "TestMap",
      player_animations: {
        down: { currentFrame: playerSprite },
      },
      player_direction: "down",
      player_x: 10,
      player_y: 10,
      player_px_x: 72,
      player_px_y: 72,
      target_px_x: 72,
      target_px_y: 72,
      player_object: null,
      npcs: [],
      _npc_pixel_position: () => [0, 0],
      TILES_PER_COLLISION: 2,
    });

    instance.draw();

    // map blit + 500 marker blits + player sprite blit
    expect(screen.blit).toHaveBeenCalledTimes(502);
  });

  it("throws when the ledge shadow sprite was not synchronously preloaded", () => {
    class TestOverworld extends OverworldRenderingMixin {}
    const instance = new TestOverworld() as OverworldRenderingMixin & Record<string, unknown>;
    const originalLoadSync = gameEngine.image.loadSync;
    gameEngine.image.loadSync = jest.fn(() => null);

    try {
      expect(() =>
        (instance as unknown as { _load_ledge_shadow_surface: () => unknown })._load_ledge_shadow_surface()
      ).toThrow("Ledge shadow sprite must be preloaded before overworld rendering:");
    } finally {
      gameEngine.image.loadSync = originalLoadSync;
    }
  });

  it("renders a temporary black frame while overworld surfaces are still loading", () => {
    class TestOverworld extends OverworldRenderingMixin {}
    const instance = new TestOverworld() as OverworldRenderingMixin & Record<string, unknown>;

    const screen = {
      fill: jest.fn(),
      get_width: jest.fn(() => 160),
      get_height: jest.fn(() => 144),
      blit: jest.fn(),
    };

    Object.assign(instance, {
      ui: null,
      screen,
      _composite_surface: null,
      map_surface: null,
      tileset: {
        ready: Promise.resolve(),
        loaded: false,
      },
      _active_emotes: new Map(),
    });

    expect(() => instance.draw()).not.toThrow();
    expect(screen.fill).toHaveBeenCalledWith([0, 0, 0, 255]);
    expect(screen.blit).not.toHaveBeenCalled();
  });
});
