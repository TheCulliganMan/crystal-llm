import { ChangeBlockCommand, RefreshMapCommand } from "./overworld";

describe("ChangeBlockCommand", () => {
  it("writes the metatile derived from block coordinates", () => {
    const writeMetatile = jest.fn();
    const command = new ChangeBlockCommand(4, 2, 0x2e);

    command.execute({} as any, {} as any, { _write_metatile: writeMetatile } as any);

    expect(writeMetatile).toHaveBeenCalledWith(2, 1, 0x2e);
  });

  it("masks the block id to 8 bits", () => {
    const writeMetatile = jest.fn();
    const command = new ChangeBlockCommand(0, 0, 0x1ff);

    command.execute({} as any, {} as any, { _write_metatile: writeMetatile } as any);

    expect(writeMetatile).toHaveBeenCalledWith(0, 0, 0xff);
  });

  it("throws when no metatile writer is available", () => {
    const command = new ChangeBlockCommand(0, 0, 0x00);

    expect(() => command.execute({} as any, {} as any, {} as any)).toThrow(
      "ChangeBlockCommand requires an overworld with _write_metatile."
    );
  });
});

describe("RefreshMapCommand", () => {
  it("refreshes warp permissions and sprite caches when supported", () => {
    const refreshWarpPermissions = jest.fn();
    const refreshMapSprites = jest.fn();
    const command = new RefreshMapCommand();

    command.execute(
      {} as any,
      {} as any,
      {
        _refresh_warp_permissions: refreshWarpPermissions,
        refresh_map_sprites: refreshMapSprites,
      } as any,
    );

    expect(refreshWarpPermissions).toHaveBeenCalledTimes(1);
    expect(refreshMapSprites).toHaveBeenCalledWith({ reload_standing: false, reload_walking: false });
  });

  it("throws when no overworld is active", () => {
    const command = new RefreshMapCommand();
    expect(() => command.execute({} as any, {} as any, null as any)).toThrow(
      "RefreshMapCommand requires an active overworld.",
    );
  });
});
