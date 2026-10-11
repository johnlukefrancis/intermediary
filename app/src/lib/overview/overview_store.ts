// Path: app/src/lib/overview/overview_store.ts
// Description: App-lifetime all-repository observations with event-triggered immutable publication

import { isVisibleFileKind } from "../files/file_feed.js";
import { activitySupport } from "../agent/activity_support.js";
import {
  acceptDetails, createActivity, isUnchangedEdit, noteLoss, observeChange, retractLiveSignal, seedHistory, type OverviewActivity,
} from "./overview_activity.js";
import { projectEdit } from "./overview_edit.js";
import {
  OVERVIEW_EDITS_MAX, type OverviewAvailability, type OverviewRepoSnapshot,
  type OverviewSnapshot, type OverviewStore, type OverviewTransport,
} from "./overview_types.js";

const FLUSH_MS = 80;
const OFFLINE: OverviewTransport = {
  connected: false, helloOk: false, agentVersion: null, watchedRepoIds: [], wslOnline: true, error: null,
};

function alive(activity: OverviewActivity, transport: OverviewTransport): boolean {
  return transport.connected && (activity.root.kind === "host" || transport.wslOnline);
}

function usable(activity: OverviewActivity, transport: OverviewTransport): boolean {
  return alive(activity, transport) && transport.helloOk;
}

function availability(activity: OverviewActivity, transport: OverviewTransport, watched: ReadonlySet<string>, wslVersionIssue: string | null): OverviewAvailability {
  if (!transport.connected) return "offline";
  if (activity.root.kind === "wsl" && wslVersionIssue !== null) return "update-required";
  if (activity.root.kind === "wsl" && !transport.wslOnline) return "held";
  if (!transport.helloOk) return "connecting";
  if (activitySupport(transport.agentVersion) !== "supported") return "update-required";
  if (activity.watcherIssue !== null) return "error";
  if (activity.refreshUnavailable) return "unwatched";
  return watched.has(activity.repoId) || activity.confirmed ? "watching" : "unwatched";
}

export function createOverviewStore(now: () => number = Date.now): OverviewStore {
  let activities = new Map<string, OverviewActivity>();
  let transport = OFFLINE;
  let watched = new Set<string>();
  let wslVersionIssue: string | null = null;
  let wslRecoveryPending = false;
  let nextEditId = 1;
  let snapshot: OverviewSnapshot = { repos: new Map(), revision: 0 };
  let timer: ReturnType<typeof setTimeout> | null = null;
  let dirty = false;
  let disposed = false;
  const listeners = new Set<() => void>();

  function flush(): void {
    if (timer !== null) clearTimeout(timer);
    timer = null;
    if (disposed || !dirty) return;
    dirty = false;
    const repos = new Map<string, OverviewRepoSnapshot>();
    for (const activity of activities.values()) {
      repos.set(activity.repoId, {
        repoId: activity.repoId, root: { ...activity.root },
        availability: availability(activity, transport, watched, wslVersionIssue),
        issue: (activity.root.kind === "wsl" ? wslVersionIssue : null) ?? activity.watcherIssue ?? activity.refreshIssue ?? transport.error,
        recent: activity.recent.map((signal) => ({ ...signal })),
        edits: activity.edits.map((edit) => ({
          ...edit, lines: edit.lines.map((line) => ({ ...line })), image: edit.image === null ? null : { ...edit.image },
        })),
        lastChangedAt: activity.lastChangedAt,
        lastWorkedAt: activity.lastWorkedAt,
        losses: activity.losses.map((loss) => ({ ...loss })),
      });
    }
    snapshot = { repos, revision: snapshot.revision + 1 };
    for (const listener of listeners) listener();
  }

  function publish(): void {
    dirty = true;
    timer ??= setTimeout(flush, FLUSH_MS);
  }

  return {
    subscribe(listener) {
      if (disposed) return () => undefined;
      listeners.add(listener);
      return () => { listeners.delete(listener); };
    },
    getSnapshot: () => snapshot,
    setRepositories(repos) {
      if (disposed) return;
      const next = new Map<string, OverviewActivity>();
      for (const repo of repos) {
        const current = activities.get(repo.repoId);
        const retained = current !== undefined && current.root.kind === repo.root.kind && current.root.path === repo.root.path;
        if (current !== undefined && !retained) watched.delete(repo.repoId);
        next.set(repo.repoId, retained ? current : createActivity(repo.repoId, repo.root));
      }
      activities = next;
      publish();
    },
    setTransport(next) {
      if (disposed) return;
      const wasWslAlive = transport.connected && transport.wslOnline;
      const isWslAlive = next.connected && next.wslOnline;
      if ((wasWslAlive && !isWslAlive) || (!wasWslAlive && isWslAlive && wslVersionIssue !== null)) wslRecoveryPending = true;
      const nextWatched = new Set(next.watchedRepoIds);
      for (const activity of activities.values()) {
        if (alive(activity, transport) && !alive(activity, next)) {
          activity.confirmed = false;
          activity.rehydrating = false;
          activity.recoveryPending = true;
        }
      }
      transport = { ...next, watchedRepoIds: [...next.watchedRepoIds] };
      watched = nextWatched;
      publish();
    },
    intake(event, at = now()) {
      if (disposed) return;
      if (event.type === "error") {
        if (event.scope === "wslBackend" && event.details?.rawCode === "WSL_BACKEND_VERSION_MISMATCH") {
          wslVersionIssue = event.message;
          publish();
          return;
        }
        const repoId = event.details?.repoId;
        const activity = repoId === undefined ? undefined : activities.get(repoId);
        if (activity === undefined || event.scope !== "watcher" || event.details?.rootPath !== activity.root.path) return;
        activity.watcherIssue = event.message;
        activity.rehydrating = false;
        publish();
        return;
      }
      if (event.type !== "fileChanged" && event.type !== "snapshot" && event.type !== "fileDelta" && event.type !== "fileDeltaCounters") return;
      const activity = activities.get(event.repoId);
      if (activity === undefined || event.rootPath !== activity.root.path) return;
      if (usable(activity, transport)) activity.confirmed = true;
      if (event.type === "snapshot") {
        seedHistory(activity, event, at);
        activity.refreshIssue = null;
        activity.refreshUnavailable = false;
        if (activity.rehydrating && usable(activity, transport)) activity.watcherIssue = null;
        activity.rehydrating = false;
      } else if (event.type === "fileChanged") {
        observeChange(activity, event, at);
      } else {
        const lost = acceptDetails(activity, event);
        if (lost !== null && noteLoss(activity, lost, at, nextEditId)) nextEditId += 1;
        if (lost !== null && event.type === "fileDelta" && isVisibleFileKind(event.kind)) {
          if (isUnchangedEdit(event)) retractLiveSignal(activity, event.path);
          else {
            activity.edits.push(projectEdit(event, at, nextEditId));
            nextEditId += 1;
            if (activity.edits.length > OVERVIEW_EDITS_MAX) activity.edits.shift();
            activity.lastWorkedAt = at;
          }
        }
      }
      publish();
    },
    setRepoIssue(repoId, root, issue, watchUnavailable = false) {
      if (disposed) return;
      const activity = activities.get(repoId);
      if (activity === undefined || activity.root.kind !== root.kind || activity.root.path !== root.path) return;
      activity.refreshIssue = issue;
      activity.refreshUnavailable ||= watchUnavailable;
      publish();
    },
    markRehydrated() {
      if (disposed) return;
      let rehydrated = false;
      if (wslRecoveryPending && transport.connected && transport.helloOk && transport.wslOnline) {
        wslVersionIssue = null;
        wslRecoveryPending = false;
        rehydrated = true;
      }
      for (const activity of activities.values()) {
        if (!activity.recoveryPending || !usable(activity, transport)) continue;
        activity.lastSeq = null;
        activity.rehydrating = true;
        if (watched.has(activity.repoId) || activity.confirmed) activity.watcherIssue = null;
        activity.recoveryPending = false;
        rehydrated = true;
      }
      if (rehydrated) publish();
    },
    flush,
    dispose() {
      disposed = true;
      if (timer !== null) clearTimeout(timer);
      timer = null;
      listeners.clear();
      activities.clear();
    },
  };
}

export const overviewStore = createOverviewStore();
