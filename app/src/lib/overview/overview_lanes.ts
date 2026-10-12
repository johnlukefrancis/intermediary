// Path: app/src/lib/overview/overview_lanes.ts
// Description: Bounded exact-root agent session projection for Overview lane activity

import type { RepoConfig } from "../../shared/config.js";
import { containingRepository, crosstalkPathKey } from "../crosstalk/repository.js";

export const LANE_FINISH_MS = 24_000;
export interface Turn { id: string; from: string; cwd: string; text: "working" | "idle"; sent: number }
export interface Lane { working: number; finishedAt: number | null }
interface Session { id: string; repoId: string; state: Turn["text"]; at: number }

export function recentlyFinished(lane: Lane | undefined, now: number): boolean {
  return lane?.finishedAt !== null && lane?.finishedAt !== undefined && now - lane.finishedAt < LANE_FINISH_MS;
}

export function createLaneStore(): {
  subscribe: (listener: () => void) => () => void;
  getSnapshot: () => ReadonlyMap<string, Lane>;
  setRepositories: (repos: readonly Pick<RepoConfig, "repoId" | "root">[]) => void;
  receive: (turn: Turn, now: number) => string | undefined;
} {
  let roots: { repoId: string; path: string }[] = [];
  const sessions = new Map<string, Session>();
  const listeners = new Set<() => void>();
  let snapshot: ReadonlyMap<string, Lane> = new Map();
  const publish = (): void => {
    const next = new Map<string, Lane>();
    for (const session of sessions.values()) {
      const lane = next.get(session.repoId) ?? { working: 0, finishedAt: null };
      if (session.state === "working") lane.working++;
      else lane.finishedAt = Math.max(lane.finishedAt ?? 0, session.at);
      next.set(session.repoId, lane);
    }
    for (const lane of next.values()) if (lane.working > 0) lane.finishedAt = null;
    snapshot = next;
    for (const listener of listeners) listener();
  };
  return {
    subscribe(listener) { listeners.add(listener); return () => { listeners.delete(listener); }; },
    getSnapshot: () => snapshot,
    setRepositories(repos) {
      const next = repos.map(repo => ({ repoId: repo.repoId, path: repo.root.path }));
      const retained = new Set(next.filter(repo => roots.some(old => old.repoId === repo.repoId
        && crosstalkPathKey(old.path) === crosstalkPathKey(repo.path))).map(repo => repo.repoId));
      roots = next;
      let changed = false;
      for (const [address, session] of sessions) if (!retained.has(session.repoId)) {
        sessions.delete(address); changed = true;
      }
      if (changed) publish();
    },
    receive(turn, now) {
      const previous = sessions.get(turn.from);
      if (previous && turn.id <= previous.id) return;
      const repo = containingRepository(turn.cwd, roots);
      if (!repo) return;
      if (previous?.repoId === repo.repoId && previous.state === turn.text) {
        sessions.set(turn.from, { ...previous, id: turn.id });
        return;
      }
      sessions.delete(turn.from);
      sessions.set(turn.from, { id: turn.id, repoId: repo.repoId, state: turn.text, at: Math.min(now, turn.sent * 1000) });
      if (sessions.size > 128) {
        const oldest = [...sessions].find(([, session]) => session.state === "idle")?.[0] ?? sessions.keys().next().value;
        if (oldest !== undefined) sessions.delete(oldest);
      }
      publish();
      return repo.repoId;
    },
  };
}

export const overviewLanes = createLaneStore();
