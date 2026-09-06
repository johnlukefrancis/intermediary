// Path: app/src/lib/stream/stream_types.ts
// Description: Card, ring, snapshot, transport, and store contracts for the per-repo Stream store

import type { DiffLine } from "../diff/diff_lines.js";
import type { VisibleFileKind } from "../files/file_feed.js";
import type {
  AgentEvent,
  DeltaBaseline,
  DeltaOp,
  DeltaStats,
  OpaqueReason,
} from "../../shared/protocol.js";
import type { StreamSupport } from "./stream_agent_support.js";
import type { BurstDetectState } from "./stream_burst_detect.js";
import type { StreamImageStripCard } from "./stream_strip_types.js";

export type { StreamImageStripCard, StreamStripTile, StreamStripTileBody } from "./stream_strip_types.js";

export interface StreamTextBody {
  status: "text";
  /** One segment per merged delta, oldest first; the total never exceeds EXPAND_CAP lines */
  segments: readonly (readonly DiffLine[])[];
  /** Lines outside the collapsed LINE_CAP view on the standard chassis, `beyondCap` included */
  hiddenLines: number;
  /** Lines the EXPAND_CAP cut from the retained segments; they are never printable again */
  beyondCap: number;
  /** Summed over every merged delta */
  stats: DeltaStats;
  /** Baseline of the newest segment */
  baseline: DeltaBaseline;
  truncated: boolean;
}

export type StreamCardBody =
  | StreamTextBody
  | { status: "image"; bytes: number; mimeType: string | null }
  | { status: "opaque"; bytes: number; reason: OpaqueReason }
  | { status: "gone" };

export interface StreamFileCard {
  kind: "file";
  id: number;
  path: string;
  fromPath: string | null;
  fileKind: VisibleFileKind;
  op: DeltaOp;
  /** Best-effort from the wire; null when the agent did not say */
  tracked: boolean | null;
  outsideSelection: boolean;
  /** Wall clock of arrival, formatted once */
  clock: string;
  arrivedAtMs: number;
  /** Merge and memo key: the newest delta folded into this card */
  updatedAtMs: number;
  /** Stamped by the store when the card enters the ring (0 while pending); the static clock starts here */
  admittedAtMs: number;
  /** Deltas merged into this card; 1 for a single edit */
  edits: number;
  expanded: boolean;
  exiting: boolean;
  /** Older than STATIC_AFTER_MS since admission: the arrival never replays, even when extended */
  static: boolean;
  body: StreamCardBody;
}

export interface StreamBurstCard {
  kind: "burst";
  id: number;
  arrivedAtMs: number;
  updatedAtMs: number;
  admittedAtMs: number;
  /** Distinct paths absorbed */
  files: number;
  byOp: Record<DeltaOp, number>;
  byKind: Record<VisibleFileKind, number>;
  topDirs: ReadonlyArray<{ dir: string; count: number }>;
  elapsedMs: number;
  /** Deltas that arrived for absorbed paths */
  resolved: number;
  exiting: boolean;
  static: boolean;
}

export interface StreamHistoryRow {
  kind: "history";
  id: number;
  path: string;
  fileKind: VisibleFileKind;
  lastSeenAtIso: string;
  exiting: boolean;
}

export type StreamRingCard = StreamFileCard | StreamImageStripCard | StreamBurstCard | StreamHistoryRow;

/** A card the conductor has not admitted yet; never a history row */
export type StreamPendingCard = StreamFileCard | StreamImageStripCard | StreamBurstCard;

/** The two cards a reader can expand in place */
export type StreamExpandableCard = StreamFileCard | StreamImageStripCard;

export type StreamNoticeTone = "accent" | "success" | "warning" | "error";

export interface StreamNoticeRow {
  kind: "notice";
  id: number;
  /** Notices with the same key coalesce while fresh */
  key: string;
  arrivedAtMs: number;
  tone: StreamNoticeTone;
  count: number;
  text: string;
}

export interface StreamBurstOpen {
  id: number;
  untilMs: number;
  /** Distinct member paths, at most BURST_MEMBER_CAP; paths past the cap are counted on the card only */
  paths: ReadonlySet<string>;
  /** Top-level directory counts behind the card's top-3 strip, at most BURST_TOP_DIRS_TRACKED plus `other` */
  dirCounts: ReadonlyMap<string, number>;
}

/** A closed burst still owns its members' late deltas until `untilMs`; after that they print as cards */
export interface StreamBurstGrace {
  id: number;
  paths: ReadonlySet<string>;
  untilMs: number;
}

export interface StreamRing {
  cards: readonly StreamRingCard[];
  notices: readonly StreamNoticeRow[];
  /** Last seq seen on a fileDelta or fileDeltaCounters; null before the first and after a rehydrate */
  lastSeq: number | null;
  burstOpen: StreamBurstOpen | null;
  burstGrace: StreamBurstGrace | null;
}

export type StreamPressure = "calm" | "busy" | "flood";

export interface StreamSnapshot {
  ring: StreamRing;
  /** Resolved cards awaiting admission */
  pending: number;
  pressure: StreamPressure;
  visible: boolean;
  /** Hidden or minimized: admission is paused and the panel fetches no tiles */
  documentHidden: boolean;
  offline: boolean;
  held: boolean;
  support: StreamSupport;
  admittedWhileAway: number;
  /** Text paths whose fileChanged arrived and whose delta has not, bounded for display */
  settling: readonly string[];
  /** Bumps once per committed change; the snapshot object is otherwise identical */
  seq: number;
}

export interface StreamSettlingEntry {
  path: string;
  atMs: number;
}

/** Everything the pure reducers read and return; the store owns timers and view flags around it */
export interface StreamReduceState {
  ring: StreamRing;
  pending: readonly StreamPendingCard[];
  settling: readonly StreamSettlingEntry[];
  burstDetect: BurstDetectState;
  nextId: number;
}

export type StreamRepoRootKind = "host" | "wsl";

export interface StreamTransport {
  connected: boolean;
  helloOk: boolean;
  agentVersion: string | null;
  repoRootKind: StreamRepoRootKind;
  wslOnline: boolean;
}

export interface StreamHistorySeed {
  path: string;
  fileKind: VisibleFileKind;
  lastSeenAtIso: string;
}

export type StreamSelectionFilter = ((path: string) => boolean) | null;

/** Property-typed members: the store never reads `this`, so React may hold them unbound */
export interface StreamStore {
  readonly repoId: string;
  /** Buffered; the reducers run once per FLUSH_MS */
  intake: (event: AgentEvent) => void;
  setVisible: (visible: boolean) => void;
  setDocumentHidden: (hidden: boolean) => void;
  setTransport: (transport: StreamTransport) => void;
  setReducedMotion: (reduced: boolean) => void;
  setSelectionFilter: (filter: StreamSelectionFilter) => void;
  /** Reconnect: keep the ring, announce, and treat the next seq as a new stream */
  markRehydrated: () => void;
  /** Toggle a file card's or strip's expansion in place */
  expand: (id: number) => void;
  subscribe: (listener: () => void) => () => void;
  getSnapshot: () => StreamSnapshot;
  dispose: () => void;
}

export type StreamTimerHandle = number;

/** Injected clock and timers so the conductor is testable without React or real time */
export interface StreamStoreDeps {
  now(): number;
  setTimer(callback: () => void, ms: number): StreamTimerHandle;
  clearTimer(handle: StreamTimerHandle): void;
}
