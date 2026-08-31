"use client";

import { useMemo, useState, type FormEvent } from "react";
import { useConnectionState } from "@pokecrystal/core/multiplayer/multiplayer-store";
import type { MultiplayerChatChannel } from "@pokecrystal/core/multiplayer/overworld-presence";
import type { RemoteOverworldPlayer } from "@pokecrystal/core/types/overworld";

export type MultiplayerChatTab = MultiplayerChatChannel | "system";

export type MultiplayerChatLine = {
  messageId: string;
  userId: string;
  playerName: string;
  text: string;
  outgoing: boolean;
  channel: MultiplayerChatChannel;
  timestampMs: number;
};

export type MultiplayerChatProps = {
  messages?: MultiplayerChatLine[];
  systemMessages?: string[];
  remotePlayers?: RemoteOverworldPlayer[];
  selectedRemoteUserId?: string | null;
  blockedUserIds?: ReadonlySet<string>;
  onSelectRemotePlayer?: (userId: string) => void;
  onSend?: (channel: MultiplayerChatChannel, text: string) => void;
  onReport?: (message: MultiplayerChatLine) => void;
  onToggleBlock?: (userId: string) => void;
  onOpenLobby?: () => void;
};

const CHAT_TABS: Array<{ id: MultiplayerChatTab; label: string; color: string }> = [
  { id: "local", label: "Local", color: "text-amber-300" },
  { id: "trade", label: "Trade", color: "text-pink-300" },
  { id: "whisper", label: "Whisper", color: "text-violet-300" },
  { id: "system", label: "System", color: "text-cyan-300" },
];

export function MultiplayerChat(props: MultiplayerChatProps) {
  const connectionState = useConnectionState();
  const [activeTab, setActiveTab] = useState<MultiplayerChatTab>("local");
  const [draft, setDraft] = useState("");
  const messages = props.messages ?? [];
  const remotePlayers = props.remotePlayers ?? [];
  const selectedPlayer = remotePlayers.find((player) => player.userId === props.selectedRemoteUserId) ?? null;
  const visibleMessages = useMemo(
    () => messages.filter((message) => message.channel === activeTab),
    [activeTab, messages],
  );
  const canSend = connectionState === "connected"
    && activeTab !== "system"
    && (activeTab !== "whisper" || Boolean(selectedPlayer));

  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const text = draft.trim();
    if (!text || activeTab === "system" || !canSend) {
      return;
    }
    props.onSend?.(activeTab, text);
    setDraft("");
  };

  return (
    <section
      className="flex h-full min-h-[18rem] flex-col overflow-hidden rounded-lg border border-white/10 bg-[#0b0d12]/95 text-slate-100 shadow-xl"
      data-testid="multiplayer-chat"
      aria-label="Multiplayer chat"
    >
      <header className="flex items-center justify-between gap-2 border-b border-white/10 px-3 py-2">
        <div className="min-w-0">
          <h2 className="text-xs font-bold uppercase tracking-[0.16em] text-white/90">World Chat</h2>
          <p className="truncate text-[0.65rem] text-white/45">
            {connectionState === "connected" ? `${remotePlayers.length + 1} trainers online` : "Offline"}
          </p>
        </div>
        <button type="button" className="btn btn-xs btn-ghost text-white/70" onClick={props.onOpenLobby}>
          Players
        </button>
      </header>

      <div className="flex shrink-0 overflow-x-auto border-b border-white/10" role="tablist" aria-label="Chat channels">
        {CHAT_TABS.map((tab) => (
          <button
            key={tab.id}
            type="button"
            role="tab"
            aria-selected={activeTab === tab.id}
            className={`min-w-16 flex-1 border-b-2 px-2 py-2 text-[0.68rem] font-semibold ${
              activeTab === tab.id ? `border-current bg-white/5 ${tab.color}` : "border-transparent text-white/45"
            }`}
            onClick={() => setActiveTab(tab.id)}
            data-testid={`chat-tab-${tab.id}`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {activeTab === "whisper" ? (
        <div className="flex shrink-0 gap-1 overflow-x-auto border-b border-white/10 p-2" data-testid="whisper-targets">
          {remotePlayers.length ? remotePlayers.map((player) => (
            <button
              key={player.userId}
              type="button"
              className={`btn btn-xs rounded-full normal-case ${player.userId === props.selectedRemoteUserId ? "btn-secondary" : "btn-ghost"}`}
              onClick={() => props.onSelectRemotePlayer?.(player.userId)}
            >
              {player.playerName}
            </button>
          )) : <span className="px-1 text-[0.65rem] text-white/45">No trainer selected</span>}
        </div>
      ) : null}

      <div className="min-h-0 flex-1 space-y-1 overflow-y-auto px-3 py-2 font-mono text-xs leading-relaxed" data-testid="chat-log">
        {activeTab === "system" ? (
          (props.systemMessages?.length ? props.systemMessages : ["System messages and multiplayer notices appear here."]).map((text, index) => (
            <p key={`${index}-${text}`} className="text-cyan-300/80">[System] {text}</p>
          ))
        ) : visibleMessages.length ? visibleMessages.map((message) => (
          <div key={message.messageId} className="group flex items-start gap-1">
            <p className="min-w-0 flex-1 break-words">
              <span className={message.outgoing ? "text-emerald-300" : "text-amber-200"}>
                [{message.outgoing ? "You" : message.playerName}]
              </span>{" "}
              <span className="text-slate-200">{message.text}</span>
            </p>
            {!message.outgoing ? (
              <span className="flex shrink-0 opacity-40 transition group-hover:opacity-100 group-focus-within:opacity-100">
                <button
                  type="button"
                  className="px-1 text-[0.6rem] text-slate-400 hover:text-amber-200"
                  onClick={() => props.onToggleBlock?.(message.userId)}
                  data-testid={`block-${message.messageId}`}
                >
                  {props.blockedUserIds?.has(message.userId) ? "Unignore" : "Ignore"}
                </button>
                <button
                  type="button"
                  className="px-1 text-[0.6rem] text-slate-400 hover:text-red-300"
                  onClick={() => props.onReport?.(message)}
                  data-testid={`report-${message.messageId}`}
                >
                  Report
                </button>
              </span>
            ) : null}
          </div>
        )) : (
          <p className="text-white/35">No messages in this channel yet.</p>
        )}
      </div>

      <form className="flex shrink-0 gap-2 border-t border-white/10 p-2" onSubmit={submit} data-testid="chat-form">
        <span className="self-center text-[0.65rem] font-bold uppercase text-white/35">/{activeTab}</span>
        <input
          type="text"
          value={draft}
          maxLength={240}
          onChange={(event) => setDraft(event.target.value)}
          className="input input-xs input-bordered min-w-0 flex-1 bg-black/30 text-white placeholder:text-white/25"
          placeholder={activeTab === "system" ? "Read only" : activeTab === "whisper" ? `Whisper ${selectedPlayer?.playerName ?? "a trainer"}` : `Message ${activeTab}`}
          disabled={!canSend}
          aria-label="Chat message"
          data-testid="chat-input"
        />
        <button type="submit" className="btn btn-xs btn-primary" disabled={!canSend || !draft.trim()} data-testid="chat-send">
          Send
        </button>
      </form>
    </section>
  );
}
