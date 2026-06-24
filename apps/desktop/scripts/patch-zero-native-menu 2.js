"use strict";

const fs = require("fs");
const path = require("path");

const ROOT_DIR = path.resolve(__dirname, "../../..");
const ZERO_NATIVE_DIR = path.join(ROOT_DIR, "node_modules", "zero-native");

const patchFile = (relativePath, patches) => {
  const filePath = path.join(ZERO_NATIVE_DIR, relativePath);
  let source = fs.readFileSync(filePath, "utf8");
  let changed = false;
  for (const { before, after } of patches) {
    if (source.includes(after)) {
      continue;
    }
    if (!source.includes(before)) {
      throw new Error(`Could not patch zero-native ${relativePath}; expected block was not found.`);
    }
    source = source.replace(before, after);
    changed = true;
  }
  if (changed) {
    fs.writeFileSync(filePath, source);
  }
};

const appkitFile = "src/platform/macos/appkit_host.m";
const cefFile = "src/platform/macos/cef_host.mm";

const appkitFileMenuBefore = `    [fileMenu addItem:[self menuItem:@"Close Window" action:@selector(performClose:) key:@"w" modifiers:NSEventModifierFlagCommand]];`;
const appkitFileMenuAfter = `    [fileMenu addItem:[self menuItem:@"MCP Streamable HTTP" action:@selector(openKrabbyclawMcp:) key:@"m" modifiers:(NSEventModifierFlagCommand | NSEventModifierFlagOption)]];
    [fileMenu addItem:[self menuItem:@"Local Saves" action:@selector(openKrabbyclawSaves:) key:@"s" modifiers:(NSEventModifierFlagCommand | NSEventModifierFlagOption)]];
    [fileMenu addItem:[NSMenuItem separatorItem]];
    [fileMenu addItem:[self menuItem:@"Close Window" action:@selector(performClose:) key:@"w" modifiers:NSEventModifierFlagCommand]];`;

const appkitViewMenuBefore = `    [viewMenu addItem:[self menuItem:@"Reload" action:@selector(reload:) key:@"r" modifiers:NSEventModifierFlagCommand]];
    [viewMenu addItem:[self menuItem:@"Toggle Web Inspector" action:@selector(toggleWebInspector:) key:@"i" modifiers:(NSEventModifierFlagCommand | NSEventModifierFlagOption)]];
}`;
const appkitViewMenuAfter = `    [viewMenu addItem:[self menuItem:@"Show Sidebar" action:@selector(openKrabbyclawSidebar:) key:@"b" modifiers:(NSEventModifierFlagCommand | NSEventModifierFlagOption)]];
    [viewMenu addItem:[NSMenuItem separatorItem]];
    [viewMenu addItem:[self menuItem:@"Reload" action:@selector(reload:) key:@"r" modifiers:NSEventModifierFlagCommand]];
    [viewMenu addItem:[self menuItem:@"Toggle Web Inspector" action:@selector(toggleWebInspector:) key:@"i" modifiers:(NSEventModifierFlagCommand | NSEventModifierFlagOption)]];

    NSMenuItem *serverMenuItem = [[NSMenuItem alloc] initWithTitle:@"Server" action:nil keyEquivalent:@""];
    [mainMenu addItem:serverMenuItem];
    NSMenu *serverMenu = [[NSMenu alloc] initWithTitle:@"Server"];
    [serverMenuItem setSubmenu:serverMenu];
    [serverMenu addItem:[self menuItem:@"MCP Streamable HTTP" action:@selector(openKrabbyclawMcp:) key:@"m" modifiers:(NSEventModifierFlagCommand | NSEventModifierFlagShift)]];
    [serverMenu addItem:[self menuItem:@"Preferences" action:@selector(showPreferences:) key:@"," modifiers:NSEventModifierFlagCommand]];
}`;

const appkitPreferencesBefore = `- (void)showPreferences:(id)sender {
    (void)sender;
}`;
const appkitPreferencesAfter = `- (void)emitKrabbyclawDesktopMenuAction:(NSString *)action {
    WKWebView *webView = [self mainWebViewForWindow:NSApp.keyWindow];
    if (!webView || action.length == 0) return;
    NSString *script = [NSString stringWithFormat:@"window.dispatchEvent(new CustomEvent('krabbyclaw:desktop-menu',{detail:{action:'%@'}}));", action];
    [webView evaluateJavaScript:script completionHandler:nil];
}

- (void)showPreferences:(id)sender {
    (void)sender;
    [self emitKrabbyclawDesktopMenuAction:@"preferences"];
}

- (void)openKrabbyclawMcp:(id)sender {
    (void)sender;
    [self emitKrabbyclawDesktopMenuAction:@"mcp"];
}

- (void)openKrabbyclawSaves:(id)sender {
    (void)sender;
    [self emitKrabbyclawDesktopMenuAction:@"saves"];
}

- (void)openKrabbyclawSidebar:(id)sender {
    (void)sender;
    [self emitKrabbyclawDesktopMenuAction:@"sidebar"];
}`;

const cefFileMenuBefore = `    [fileMenu addItem:[self menuItem:@"Close Window" action:@selector(performClose:) key:@"w" modifiers:NSEventModifierFlagCommand]];`;
const cefFileMenuAfter = `    [fileMenu addItem:[self menuItem:@"MCP Streamable HTTP" action:@selector(openKrabbyclawMcp:) key:@"m" modifiers:(NSEventModifierFlagCommand | NSEventModifierFlagOption)]];
    [fileMenu addItem:[self menuItem:@"Local Saves" action:@selector(openKrabbyclawSaves:) key:@"s" modifiers:(NSEventModifierFlagCommand | NSEventModifierFlagOption)]];
    [fileMenu addItem:[NSMenuItem separatorItem]];
    [fileMenu addItem:[self menuItem:@"Close Window" action:@selector(performClose:) key:@"w" modifiers:NSEventModifierFlagCommand]];`;

const cefViewMenuBefore = `    [viewMenu addItem:[self menuItem:@"Reload" action:@selector(reload:) key:@"r" modifiers:NSEventModifierFlagCommand]];
}`;
const cefViewMenuAfter = `    [viewMenu addItem:[self menuItem:@"Show Sidebar" action:@selector(openKrabbyclawSidebar:) key:@"b" modifiers:(NSEventModifierFlagCommand | NSEventModifierFlagOption)]];
    [viewMenu addItem:[NSMenuItem separatorItem]];
    [viewMenu addItem:[self menuItem:@"Reload" action:@selector(reload:) key:@"r" modifiers:NSEventModifierFlagCommand]];

    NSMenuItem *serverMenuItem = [[NSMenuItem alloc] initWithTitle:@"Server" action:nil keyEquivalent:@""];
    [mainMenu addItem:serverMenuItem];
    NSMenu *serverMenu = [[NSMenu alloc] initWithTitle:@"Server"];
    [serverMenuItem setSubmenu:serverMenu];
    [serverMenu addItem:[self menuItem:@"MCP Streamable HTTP" action:@selector(openKrabbyclawMcp:) key:@"m" modifiers:(NSEventModifierFlagCommand | NSEventModifierFlagShift)]];
    [serverMenu addItem:[self menuItem:@"Preferences" action:@selector(showPreferences:) key:@"," modifiers:NSEventModifierFlagCommand]];
}`;

const cefPreferencesBefore = `- (void)showPreferences:(id)sender {
    (void)sender;
}`;
const cefPreferencesAfter = `- (uint64_t)activeKrabbyclawWindowId {
    NSWindow *keyWindow = NSApp.keyWindow;
    uint64_t windowId = 1;
    for (NSNumber *key in self.windows) {
        if ([self.windows[key] isEqual:keyWindow]) {
            return key.unsignedLongLongValue;
        }
    }
    return windowId;
}

- (void)emitKrabbyclawDesktopMenuAction:(NSString *)action {
    if (action.length == 0 || !self.browsers) return;
    auto it = self.browsers->find([self activeKrabbyclawWindowId]);
    if (it == self.browsers->end() || !it->second) return;
    std::string script = "window.dispatchEvent(new CustomEvent('krabbyclaw:desktop-menu',{detail:{action:'" + std::string(action.UTF8String) + "'}}));";
    it->second->GetMainFrame()->ExecuteJavaScript(script, it->second->GetMainFrame()->GetURL(), 0);
}

- (void)showPreferences:(id)sender {
    (void)sender;
    [self emitKrabbyclawDesktopMenuAction:@"preferences"];
}

- (void)openKrabbyclawMcp:(id)sender {
    (void)sender;
    [self emitKrabbyclawDesktopMenuAction:@"mcp"];
}

- (void)openKrabbyclawSaves:(id)sender {
    (void)sender;
    [self emitKrabbyclawDesktopMenuAction:@"saves"];
}

- (void)openKrabbyclawSidebar:(id)sender {
    (void)sender;
    [self emitKrabbyclawDesktopMenuAction:@"sidebar"];
}`;

const patchZeroNativeMenu = () => {
  patchFile(appkitFile, [
    { before: appkitFileMenuBefore, after: appkitFileMenuAfter },
    { before: appkitViewMenuBefore, after: appkitViewMenuAfter },
    { before: appkitPreferencesBefore, after: appkitPreferencesAfter },
  ]);
  patchFile(cefFile, [
    { before: cefFileMenuBefore, after: cefFileMenuAfter },
    { before: cefViewMenuBefore, after: cefViewMenuAfter },
    { before: cefPreferencesBefore, after: cefPreferencesAfter },
  ]);
};

if (require.main === module) {
  patchZeroNativeMenu();
}

module.exports = { patchZeroNativeMenu };
