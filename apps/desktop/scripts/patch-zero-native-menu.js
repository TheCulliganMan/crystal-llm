"use strict";

const fs = require("node:fs");
const path = require("node:path");

const ROOT_DIR = path.resolve(__dirname, "../../..");
const ZERO_NATIVE_DIR = path.join(ROOT_DIR, "node_modules", "zero-native");

const replaceFirstAvailable = (source, replacements, relativePath) => {
  for (const { before, after } of replacements) {
    if (source.includes(after)) {
      return { source, changed: false };
    }
    if (source.includes(before)) {
      return { source: source.replace(before, after), changed: true };
    }
  }
  throw new Error(`Could not patch zero-native ${relativePath}; expected menu block was not found.`);
};

const patchFile = (relativePath, patchGroups) => {
  const filePath = path.join(ZERO_NATIVE_DIR, relativePath);
  let source = fs.readFileSync(filePath, "utf8");
  let changed = false;
  for (const replacements of patchGroups) {
    const result = replaceFirstAvailable(source, replacements, relativePath);
    source = result.source;
    changed = changed || result.changed;
  }
  if (changed) {
    fs.writeFileSync(filePath, source);
  }
};

const appkitOriginalFileMenu = `    [fileMenu addItem:[self menuItem:@"Close Window" action:@selector(performClose:) key:@"w" modifiers:NSEventModifierFlagCommand]];`;
const appkitPatchedFileMenu = `    [fileMenu addItem:[self menuItem:@"MCP Streamable HTTP" action:@selector(openKrabbyclawMcp:) key:@"m" modifiers:(NSEventModifierFlagCommand | NSEventModifierFlagOption)]];
    [fileMenu addItem:[self menuItem:@"Local Saves" action:@selector(openKrabbyclawSaves:) key:@"s" modifiers:(NSEventModifierFlagCommand | NSEventModifierFlagOption)]];
    [fileMenu addItem:[NSMenuItem separatorItem]];
    [fileMenu addItem:[self menuItem:@"Close Window" action:@selector(performClose:) key:@"w" modifiers:NSEventModifierFlagCommand]];`;

const appkitOriginalViewMenu = `    [viewMenu addItem:[self menuItem:@"Reload" action:@selector(reload:) key:@"r" modifiers:NSEventModifierFlagCommand]];
    [viewMenu addItem:[self menuItem:@"Toggle Web Inspector" action:@selector(toggleWebInspector:) key:@"i" modifiers:(NSEventModifierFlagCommand | NSEventModifierFlagOption)]];
}`;
const appkitPatchedViewMenu = `    [viewMenu addItem:[self menuItem:@"Show Sidebar" action:@selector(openKrabbyclawSidebar:) key:@"b" modifiers:(NSEventModifierFlagCommand | NSEventModifierFlagOption)]];
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

const appkitOriginalPreferences = `- (void)showPreferences:(id)sender {
    (void)sender;
}`;
const appkitOldEventPreferences = `- (void)emitKrabbyclawDesktopMenuAction:(NSString *)action {
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
const appkitPatchedPreferences = `- (void)openKrabbyclawPath:(NSString *)path {
    WKWebView *webView = [self mainWebViewForWindow:NSApp.keyWindow];
    if (!webView || path.length == 0) return;
    NSString *script = [NSString stringWithFormat:@"window.location.assign('%@');", path];
    [webView evaluateJavaScript:script completionHandler:nil];
}

- (void)emitKrabbyclawMenuCommand:(NSString *)command {
    WKWebView *webView = [self mainWebViewForWindow:NSApp.keyWindow];
    if (!webView || command.length == 0) return;
    NSString *script = [NSString stringWithFormat:@"window.dispatchEvent(new CustomEvent('zero-native:menu-command',{detail:{command:'%@'}}));", command];
    [webView evaluateJavaScript:script completionHandler:nil];
}

- (void)showPreferences:(id)sender {
    (void)sender;
    [self openKrabbyclawPath:@"/desktop?panel=settings"];
}

- (void)openKrabbyclawMcp:(id)sender {
    (void)sender;
    [self openKrabbyclawPath:@"/desktop?panel=mcp"];
}

- (void)openKrabbyclawSaves:(id)sender {
    (void)sender;
    [self openKrabbyclawPath:@"/desktop?panel=saves"];
}

- (void)openKrabbyclawSidebar:(id)sender {
    (void)sender;
    [self emitKrabbyclawMenuCommand:@"sidebar"];
}`;
const appkitOldPathPreferences = appkitPatchedPreferences.replace(
  '[self openKrabbyclawPath:@"/desktop?panel=mcp"];',
  '[self openKrabbyclawPath:@"/mcp"];'
);

const cefOriginalViewMenu = `    [viewMenu addItem:[self menuItem:@"Reload" action:@selector(reload:) key:@"r" modifiers:NSEventModifierFlagCommand]];
}`;
const cefPatchedViewMenu = `    [viewMenu addItem:[self menuItem:@"Show Sidebar" action:@selector(openKrabbyclawSidebar:) key:@"b" modifiers:(NSEventModifierFlagCommand | NSEventModifierFlagOption)]];
    [viewMenu addItem:[NSMenuItem separatorItem]];
    [viewMenu addItem:[self menuItem:@"Reload" action:@selector(reload:) key:@"r" modifiers:NSEventModifierFlagCommand]];

    NSMenuItem *serverMenuItem = [[NSMenuItem alloc] initWithTitle:@"Server" action:nil keyEquivalent:@""];
    [mainMenu addItem:serverMenuItem];
    NSMenu *serverMenu = [[NSMenu alloc] initWithTitle:@"Server"];
    [serverMenuItem setSubmenu:serverMenu];
    [serverMenu addItem:[self menuItem:@"MCP Streamable HTTP" action:@selector(openKrabbyclawMcp:) key:@"m" modifiers:(NSEventModifierFlagCommand | NSEventModifierFlagShift)]];
    [serverMenu addItem:[self menuItem:@"Preferences" action:@selector(showPreferences:) key:@"," modifiers:NSEventModifierFlagCommand]];
}`;

const cefOldEventPreferences = `- (void)emitKrabbyclawDesktopMenuAction:(NSString *)action {
    if (action.length == 0 || !self.browsers) return;
    auto it = self.browsers->find([self activeKrabbyclawWindowId]);
    if (it == self.browsers->end() || !it->second) return;
    std::string script = "window.dispatchEvent(new CustomEvent('krabbyclaw:desktop-menu',{detail:{action:'" + std::string(action.UTF8String) + "'}}));";
    it->second->GetMainFrame()->ExecuteJavaScript(script, it->second->GetMainFrame()->GetURL(), 0);
}`;
const cefOriginalPreferences = appkitOriginalPreferences;
const cefPatchedPreferences = `- (uint64_t)activeKrabbyclawWindowId {
    NSWindow *keyWindow = NSApp.keyWindow;
    for (NSNumber *key in self.windows) {
        if ([self.windows[key] isEqual:keyWindow]) {
            return key.unsignedLongLongValue;
        }
    }
    return 1;
}

- (void)openKrabbyclawPath:(NSString *)path {
    if (path.length == 0 || !self.browsers) return;
    auto it = self.browsers->find([self activeKrabbyclawWindowId]);
    if (it == self.browsers->end() || !it->second) return;
    std::string script = "window.location.assign('" + std::string(path.UTF8String) + "');";
    it->second->GetMainFrame()->ExecuteJavaScript(script, it->second->GetMainFrame()->GetURL(), 0);
}

- (void)emitKrabbyclawMenuCommand:(NSString *)command {
    if (command.length == 0 || !self.browsers) return;
    auto it = self.browsers->find([self activeKrabbyclawWindowId]);
    if (it == self.browsers->end() || !it->second) return;
    std::string script = "window.dispatchEvent(new CustomEvent('zero-native:menu-command',{detail:{command:'" + std::string(command.UTF8String) + "'}}));";
    it->second->GetMainFrame()->ExecuteJavaScript(script, it->second->GetMainFrame()->GetURL(), 0);
}

- (void)showPreferences:(id)sender {
    (void)sender;
    [self openKrabbyclawPath:@"/desktop?panel=settings"];
}

- (void)openKrabbyclawMcp:(id)sender {
    (void)sender;
    [self openKrabbyclawPath:@"/desktop?panel=mcp"];
}

- (void)openKrabbyclawSaves:(id)sender {
    (void)sender;
    [self openKrabbyclawPath:@"/desktop?panel=saves"];
}

- (void)openKrabbyclawSidebar:(id)sender {
    (void)sender;
    [self emitKrabbyclawMenuCommand:@"sidebar"];
}`;
const cefOldPathPreferences = cefPatchedPreferences.replace(
  '[self openKrabbyclawPath:@"/desktop?panel=mcp"];',
  '[self openKrabbyclawPath:@"/mcp"];'
);

const patchZeroNativeMenu = () => {
  patchFile("src/platform/macos/appkit_host.m", [
    [{ before: appkitOriginalFileMenu, after: appkitPatchedFileMenu }],
    [{ before: appkitOriginalViewMenu, after: appkitPatchedViewMenu }],
    [
      { before: appkitOldPathPreferences, after: appkitPatchedPreferences },
      { before: appkitOldEventPreferences, after: appkitPatchedPreferences },
      { before: appkitOriginalPreferences, after: appkitPatchedPreferences },
    ],
  ]);
  patchFile("src/platform/macos/cef_host.mm", [
    [{ before: appkitOriginalFileMenu, after: appkitPatchedFileMenu }],
    [{ before: cefOriginalViewMenu, after: cefPatchedViewMenu }],
    [
      { before: cefOldPathPreferences, after: cefPatchedPreferences },
      { before: cefOldEventPreferences, after: cefPatchedPreferences },
      { before: cefOriginalPreferences, after: cefPatchedPreferences },
    ],
  ]);
};

if (require.main === module) {
  patchZeroNativeMenu();
}

module.exports = { patchZeroNativeMenu };
