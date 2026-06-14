const std = @import("std");
const runner = @import("runner");
const zero_native = @import("zero-native");

pub const panic = std.debug.FullPanic(zero_native.debug.capturePanic);

const App = struct {
    env_map: *std.process.Environ.Map,

    fn app(self: *@This()) zero_native.App {
        return .{
            .context = self,
            .name = "krabbyclaw-desktop",
            .source = zero_native.WebViewSource.assets(.{
                .root_path = "dist/resources",
                .entry = "index.html",
                .origin = "zero://app",
                .spa_fallback = true,
            }),
            .source_fn = source,
        };
    }

    fn source(context: *anyopaque) anyerror!zero_native.WebViewSource {
        const self: *@This() = @ptrCast(@alignCast(context));
        if (self.env_map.get("KRABBY_DESKTOP_URL")) |url| {
            return zero_native.WebViewSource.url(url);
        }
        if (self.env_map.get("ZERO_NATIVE_FRONTEND_URL")) |url| {
            return zero_native.WebViewSource.url(url);
        }
        return zero_native.WebViewSource.assets(.{
            .root_path = "dist/resources",
            .entry = "index.html",
            .origin = "zero://app",
            .spa_fallback = true,
        });
    }
};

const dev_origins = [_][]const u8{ "zero://app", "zero://inline", "http://127.0.0.1:3000" };

pub fn main(init: std.process.Init) !void {
    var app = App{ .env_map = init.environ_map };
    try runner.runWithOptions(app.app(), .{
        .app_name = "KrabbyClaw Desktop",
        .window_title = "KrabbyClaw Desktop",
        .bundle_id = "com.pokecrystal.desktop",
        .icon_path = "assets/icon.icns",
        .security = .{
            .navigation = .{ .allowed_origins = &dev_origins },
        },
    }, init);
}

test "app name is configured" {
    try std.testing.expectEqualStrings("krabbyclaw-desktop", "krabbyclaw-desktop");
}
