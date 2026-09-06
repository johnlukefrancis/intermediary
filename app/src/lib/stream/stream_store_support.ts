// Path: app/src/lib/stream/stream_store_support.ts
// Description: Store-side pure helpers: browser timer deps, transport facts, the selection remap, and the snapshot projection

import { streamSupport } from "./stream_agent_support.js";
import { pressureBand } from "./stream_cadence.js";
import type {
  StreamPendingCard,
  StreamReduceState,
  StreamSnapshot,
  StreamStoreDeps,
  StreamTransport,
} from "./stream_types.js";

export const OFFLINE_TRANSPORT: StreamTransport = {
  connected: false,
  helloOk: false,
  agentVersion: null,
  repoRootKind: "host",
  wslOnline: true,
};

export function browserStoreDeps(): StreamStoreDeps {
  return {
    now: () => Date.now(),
    setTimer: (callback, ms) => window.setTimeout(callback, ms),
    clearTimer: (handle) => { window.clearTimeout(handle); },
  };
}

/** A WSL-rooted repo has no watcher while the WSL backend is offline; host repos are unaffected */
export function isHeld(transport: StreamTransport): boolean {
  return transport.repoRootKind === "wsl" && !transport.wslOnline;
}

/** The ZIP selection moved: every file card and every strip tile re-reads whether it sits outside it */
export function remapSelection(state: StreamReduceState, outside: (path: string) => boolean): StreamReduceState {
  const remap = (card: StreamPendingCard): StreamPendingCard => {
    if (card.kind === "file") return { ...card, outsideSelection: outside(card.path) };
    if (card.kind === "images") {
      return { ...card, tiles: card.tiles.map((tile) => ({ ...tile, outsideSelection: outside(tile.path) })) };
    }
    return card;
  };
  return {
    ...state,
    pending: state.pending.map(remap),
    ring: { ...state.ring, cards: state.ring.cards.map((card) => (card.kind === "history" ? card : remap(card))) },
  };
}

/** The panel shows the scroller when anything at all is on it: a card, a notice row, or the settling line */
export function snapshotHasEntries(snapshot: StreamSnapshot): boolean {
  return snapshot.ring.cards.length > 0 || snapshot.ring.notices.length > 0 || snapshot.settling.length > 0;
}

export interface SnapshotInput {
  state: StreamReduceState;
  visible: boolean;
  documentHidden: boolean;
  transport: StreamTransport;
  admittedWhileAway: number;
  seq: number;
}

export function buildSnapshot(input: SnapshotInput): StreamSnapshot {
  const { state, visible, documentHidden, transport, admittedWhileAway, seq } = input;
  return {
    ring: state.ring,
    pending: state.pending.length,
    pressure: pressureBand(state.pending.length),
    visible,
    documentHidden,
    offline: !transport.connected || !transport.helloOk,
    held: isHeld(transport),
    support: streamSupport(transport.agentVersion),
    admittedWhileAway,
    // Already capped at SETTLING_MAX by the reducer; the projection only drops the timestamps
    settling: state.settling.map((entry) => entry.path),
    seq,
  };
}
