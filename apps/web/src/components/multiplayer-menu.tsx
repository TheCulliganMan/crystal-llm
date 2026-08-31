"use client";

import {
  useConnectionState,
  useIsInQueue,
  useMultiplayerError,
} from "@pokecrystal/core/multiplayer/multiplayer-store";
import type { MatchmakingMode } from "@pokecrystal/core/multiplayer/matchmaking-service";
import type { RemoteOverworldPlayer } from "@pokecrystal/core/types/overworld";

export type MultiplayerLeaderboardEntry = {
  id: string | null;
  display_name: string | null;
  handle: string | null;
  link_battle_rating: number | null;
  link_battle_wins: number | null;
  link_battle_losses: number | null;
  total_trades?: number | null;
  rank: number | null;
};

export type MultiplayerMenuProps = {
  onConnect?: () => void;
  onDisconnect?: () => void;
  onToggleRemoteSprites?: () => void;
  onToggleCrowdView?: () => void;
  onRequestBattle?: () => void;
  onRequestTrade?: () => void;
  onSelectRemotePlayer?: (userId: string) => void;
  onAcceptRequest?: () => void;
  onDeclineRequest?: () => void;
  onJoinMatchmaking?: (mode: MatchmakingMode) => void;
  onLeaveMatchmaking?: () => void;
  isAuthenticated?: boolean;
  authLabel?: string | null;
  remotePlayers?: RemoteOverworldPlayer[];
  selectedRemoteUserId?: string | null;
  leaderboard?: MultiplayerLeaderboardEntry[];
  remoteSpritesVisible?: boolean;
  crowdViewEnabled?: boolean;
  onlinePlayerCount?: number;
  onlineAiCount?: number;
  canRequestInteraction?: boolean;
  pendingOutgoingLabel?: string | null;
  incomingRequestLabel?: string | null;
  interactionStatusLabel?: string | null;
  queueMode?: MatchmakingMode | null;
};

const interactionStatusChip = (state: string | null | undefined) => {
  if (state === "connected") {
    return "badge-success";
  }
  if (state === "error") {
    return "badge-error";
  }
  return "badge-outline";
};

const sectionLabel = (text: string) => (
  <p className="text-xs font-semibold uppercase tracking-[0.14em] text-base-content/65">{text}</p>
);

const getPlayerDistanceLabel = (player: RemoteOverworldPlayer) =>
  `${player.mapName} (${player.tileX}, ${player.tileY})`;

export function MultiplayerMenu(props: MultiplayerMenuProps) {
  const connectionState = useConnectionState();
  const error = useMultiplayerError();
  const inQueue = useIsInQueue();

  const isConnected = connectionState === "connected";
  const isConnecting = connectionState === "connecting";
  const remoteSpritesVisible = props.remoteSpritesVisible ?? true;
  const crowdViewEnabled = props.crowdViewEnabled ?? false;
  const onlinePlayerCount = Math.max(0, Math.trunc(props.onlinePlayerCount ?? 0));
  const onlineAiCount = Math.max(0, Math.trunc(props.onlineAiCount ?? 0));
  const canRequestInteraction = Boolean(props.canRequestInteraction);
  const isAuthenticated = props.isAuthenticated ?? true;
  const remotePlayers = props.remotePlayers ?? [];
  const selectedRemoteUserId = props.selectedRemoteUserId ?? remotePlayers[0]?.userId ?? null;
  const selectedRemotePlayer = remotePlayers.find((player) => player.userId === selectedRemoteUserId) ?? null;
  const leaderboard = props.leaderboard ?? [];


  const interactionDisabled = !isConnected || !canRequestInteraction || !selectedRemotePlayer;
  const interactionTooltip = !isConnected
    ? "Connect to the world first."
    : !selectedRemotePlayer
      ? "Select a player to request."
      : !canRequestInteraction
        ? "No online players available."
        : "";

  const handleToggleConnection = () => {
    if (!isAuthenticated) {
      return;
    }
    if (isConnected) {
      props.onDisconnect?.();
    } else {
      props.onConnect?.();
    }
  };

  const lobbyLabel = isConnected
    ? "Live lobby"
    : isConnecting
      ? "Joining lobby"
      : "Offline lobby";

  return (
    <div
      className="card border border-base-300/80 bg-gradient-to-b from-base-100/95 to-base-200/85 text-sm shadow-md"
      data-testid="multiplayer-menu"
    >
      <div className="card-body gap-3 p-4 sm:p-5">
        <div className="flex flex-wrap items-start justify-between gap-2">
          <div>
            <h2 className="text-sm font-semibold uppercase tracking-[0.14em] text-base-content/90">Competition Hub</h2>
            <p className="text-xs text-base-content/70">{lobbyLabel}</p>
          </div>
          <div className="flex flex-wrap gap-1">
            <span className={`badge badge-sm ${interactionStatusChip(connectionState)} badge-outline`}>{connectionState}</span>
            <span className={`badge badge-sm ${isConnected ? "badge-primary" : "badge-ghost"}`}>{isConnected ? "Ready" : "Idle"}</span>
          </div>
        </div>

        {error ? (
          <div className="alert alert-error alert-soft py-2 text-xs" data-testid="mp-error">
            {error}
          </div>
        ) : null}

        {!isAuthenticated ? (
          <div className="alert alert-warning alert-soft py-2 text-xs" data-testid="mp-auth-required">
            {props.authLabel ?? "Sign in to use multiplayer."}
          </div>
        ) : null}

        <button
          type="button"
          className={`btn btn-sm ${isConnected ? "btn-outline" : "btn-primary"} w-full`}
          onClick={handleToggleConnection}
          disabled={isConnecting || !isAuthenticated}
          data-testid="toggle-connection"
        >
          {isConnecting ? <span className="loading loading-spinner loading-xs mr-1" /> : null}
          {isConnecting ? "Connecting..." : isConnected ? "Disconnect from World" : "Join Multiplayer World"}
        </button>

        <div className="flex flex-wrap gap-2">
          <span className="badge badge-outline" data-testid="online-players-count">Frontend: {onlinePlayerCount}</span>
          <span className="badge badge-outline" data-testid="online-ai-count">API/MCP: {onlineAiCount}</span>
        </div>

        <div className="divider my-0" />

        <div className="flex flex-col gap-2">
          {sectionLabel("Global Matchmaking")}
          <p className="text-xs text-base-content/70">
            Ranked queues only match trainers running this exact modpack.
          </p>
          {inQueue ? (
            <button
              type="button"
              className="btn btn-sm btn-warning btn-outline w-full"
              onClick={props.onLeaveMatchmaking}
              data-testid="leave-matchmaking"
            >
              Leave {props.queueMode ?? "multiplayer"} Queue
            </button>
          ) : (
            <div className="grid grid-cols-2 gap-2">
              <button
                type="button"
                className="btn btn-sm btn-primary"
                onClick={() => props.onJoinMatchmaking?.("battle")}
                disabled={!isAuthenticated}
                data-testid="join-battle-queue"
              >
                Find Battle
              </button>
              <button
                type="button"
                className="btn btn-sm btn-outline"
                onClick={() => props.onJoinMatchmaking?.("trade")}
                disabled={!isAuthenticated}
                data-testid="join-trade-queue"
              >
                Find Trade
              </button>
            </div>
          )}
        </div>

        <div className="divider my-0" />

        <div className="flex flex-col gap-2">
          {sectionLabel("Online Trainers")}
          {remotePlayers.length ? (
            <div className="grid gap-2" data-testid="remote-player-list">
              {remotePlayers.map((player) => {
                const selected = player.userId === selectedRemoteUserId;
                return (
                  <button
                    key={player.userId}
                    type="button"
                    className={`btn btn-sm justify-between rounded-lg normal-case ${selected ? "btn-primary" : "btn-outline"}`}
                    onClick={() => props.onSelectRemotePlayer?.(player.userId)}
                    disabled={!isConnected}
                    data-testid={`remote-player-${player.userId}`}
                  >
                    <span className="truncate">{player.playerName}</span>
                    <span className="text-[0.7rem] opacity-80">{getPlayerDistanceLabel(player)}</span>
                  </button>
                );
              })}
            </div>
          ) : (
            <p className="rounded-box border border-base-300 bg-base-200/40 p-3 text-xs text-base-content/70" data-testid="remote-player-empty">
              {isConnected ? "No other frontend trainers are online yet." : "Join the live world to discover nearby trainers."}
            </p>
          )}
        </div>

        <div className="divider my-0" />

        <div className="flex flex-col gap-2">
          {sectionLabel("Visibility")}
          <button
            type="button"
            className={`btn btn-sm ${remoteSpritesVisible ? "btn-primary" : "btn-outline"} w-full`}
            onClick={props.onToggleRemoteSprites}
            disabled={!isConnected}
            data-testid="toggle-remote-sprites"
          >
            {remoteSpritesVisible ? "Remote NPCs: On" : "Remote NPCs: Off"}
          </button>
          <button
            type="button"
            className={`btn btn-sm ${crowdViewEnabled ? "btn-primary" : "btn-outline"} w-full`}
            onClick={props.onToggleCrowdView}
            disabled={!isConnected || !remoteSpritesVisible}
            data-testid="toggle-crowd-view"
          >
            {crowdViewEnabled ? "Crowd View: On" : "Crowd View: Off"}
          </button>
        </div>

        <div className="divider my-0" />

        <div className="flex flex-col gap-2">
          {sectionLabel("Trade or Battle")}
          {selectedRemotePlayer ? (
            <p className="text-xs text-base-content/70" data-testid="selected-remote-player">
              Targeting {selectedRemotePlayer.playerName} on {selectedRemotePlayer.mapName}.
            </p>
          ) : null}
          <div className="flex flex-wrap gap-2">
            <div
              className={`tooltip flex-1 ${interactionDisabled ? "" : "tooltip-hidden"}`}
              data-tip={interactionTooltip}
              tabIndex={interactionDisabled ? 0 : undefined}
            >
              <button
                type="button"
                className="btn btn-sm btn-outline w-full min-w-32"
                onClick={props.onRequestBattle}
                disabled={interactionDisabled}
                data-testid="request-battle"
                style={interactionDisabled ? { pointerEvents: "none" } : undefined}
              >
                Request Battle
              </button>
            </div>
            <div
              className={`tooltip flex-1 ${interactionDisabled ? "" : "tooltip-hidden"}`}
              data-tip={interactionTooltip}
              tabIndex={interactionDisabled ? 0 : undefined}
            >
              <button
                type="button"
                className="btn btn-sm btn-outline w-full min-w-32"
                onClick={props.onRequestTrade}
                disabled={interactionDisabled}
                data-testid="request-trade"
                style={interactionDisabled ? { pointerEvents: "none" } : undefined}
              >
                Request Trade
              </button>
            </div>
          </div>

          {props.pendingOutgoingLabel ? (
            <p className="text-xs text-base-content/70" data-testid="outgoing-request">
              {props.pendingOutgoingLabel}
            </p>
          ) : null}

          {props.incomingRequestLabel ? (
            <div
              className="rounded-box border border-base-300 bg-base-100 p-3"
              data-testid="incoming-request"
            >
              <p className="mb-2 text-xs text-base-content/80">{props.incomingRequestLabel}</p>
              <div className="flex flex-wrap gap-2">
                <button
                  type="button"
                  className="btn btn-sm btn-primary"
                  onClick={props.onAcceptRequest}
                  data-testid="accept-request"
                >
                  Accept
                </button>
                <button
                  type="button"
                  className="btn btn-sm btn-outline"
                  onClick={props.onDeclineRequest}
                  data-testid="decline-request"
                >
                  Decline
                </button>
              </div>
            </div>
          ) : null}

          {props.interactionStatusLabel ? (
            <p className="text-xs text-base-content/70" data-testid="interaction-status">
              {props.interactionStatusLabel}
            </p>
          ) : null}
        </div>

        {leaderboard.length ? (
          <>
            <div className="divider my-0" />
            <div className="flex flex-col gap-2">
              {sectionLabel("Ranked Trainers")}
              <div className="overflow-x-auto" data-testid="multiplayer-leaderboard">
                <table className="table table-xs">
                  <tbody>
                    {leaderboard.slice(0, 5).map((entry) => (
                      <tr key={entry.id ?? `${entry.rank}-${entry.handle}`}>
                        <td className="w-8">#{entry.rank ?? "-"}</td>
                        <td>{entry.display_name ?? entry.handle ?? "Trainer"}</td>
                        <td className="text-right">{entry.link_battle_rating ?? 1000}</td>
                        <td className="text-right">
                          {(entry.link_battle_wins ?? 0)}-{(entry.link_battle_losses ?? 0)}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          </>
        ) : null}

        <div className="alert alert-info alert-soft border border-base-300/70">
          <p className="text-xs text-base-content/80">
            {isConnected ? "Connected: trainers are live on your map." : "Connect to place your trainer into the shared world."}
          </p>
          <p className="mt-1 text-[0.7rem] text-base-content/80">
            Select a trainer on this map to talk, trade, or battle.
          </p>
        </div>
      </div>
    </div>
  );
}
