// Path: app/src/hooks/bundles/use_directory_listings.ts
// Description: Lazy directory listing state for the bundle explorer, re-listed in place when Git reports a change

import { useCallback, useEffect, useRef, useState } from "react";
import { sendListRepoDirectory } from "../../lib/agent/messages.js";
import { useAgent } from "../use_agent.js";

export interface DirectoryListingState {
  status: "idle" | "loading" | "ready" | "error";
  dirs: string[];
  files: string[];
  error?: string;
}

/** Same trailing window as the source-control status refresh, so one burst re-lists once. */
const RELIST_DEBOUNCE_MS = 300;

/** A first load may show "Loading"; a refresh keeps the visible listing until the fresh one lands. */
type ListingMode = "initial" | "refresh";

interface ListingScope {
  repoId: string;
  generation: number;
}

interface UseDirectoryListingsOptions {
  repoId: string;
  topLevelDirs: readonly string[];
  topLevelFiles: readonly string[];
}

export interface DirectoryListings {
  expandedDirs: ReadonlySet<string>;
  listings: ReadonlyMap<string, DirectoryListingState>;
  toggleExpanded: (path: string) => void;
}

export function useDirectoryListings({
  repoId,
  topLevelDirs,
  topLevelFiles,
}: UseDirectoryListingsOptions): DirectoryListings {
  const { client, helloState, subscribe } = useAgent();
  const [expandedDirs, setExpandedDirs] = useState<Set<string>>(() => new Set());
  const [listings, setListings] = useState<Map<string, DirectoryListingState>>(() => new Map());
  const scopeRef = useRef<ListingScope>({ repoId, generation: 0 });
  // Expansion has one writer, so its ref leads the state; listings have many, so the ref follows.
  const expandedDirsRef = useRef<ReadonlySet<string>>(expandedDirs);
  const listingsRef = useRef<ReadonlyMap<string, DirectoryListingState>>(listings);
  const relistTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  /** Only the newest request per path may write; the agent answers listings concurrently. */
  const requestSeqRef = useRef<Map<string, number>>(new Map());

  useEffect(() => {
    listingsRef.current = listings;
  }, [listings]);

  const clearRelistTimer = useCallback((): void => {
    if (relistTimerRef.current === null) return;
    clearTimeout(relistTimerRef.current);
    relistTimerRef.current = null;
  }, []);

  const applyExpanded = useCallback((next: Set<string>): void => {
    expandedDirsRef.current = next;
    setExpandedDirs(next);
  }, []);

  useEffect(() => {
    scopeRef.current = { repoId, generation: scopeRef.current.generation + 1 };
    clearRelistTimer();
    applyExpanded(new Set());
    requestSeqRef.current = new Map();
    setListings(new Map());
  }, [applyExpanded, clearRelistTimer, repoId, topLevelDirs, topLevelFiles]);

  const fetchListing = useCallback(
    (path: string, mode: ListingMode): void => {
      if (!client || helloState.status !== "ok") {
        if (mode === "initial") {
          setListings((prev) => new Map(prev).set(path, {
            status: "error",
            dirs: [],
            files: [],
            error: "Agent session initializing",
          }));
        }
        return;
      }

      const current = listingsRef.current.get(path);
      if (mode === "initial" && (current?.status === "loading" || current?.status === "ready")) {
        return;
      }
      if (mode === "initial") {
        setListings((prev) => new Map(prev).set(path, { status: "loading", dirs: [], files: [] }));
      }

      const requestScope = scopeRef.current;
      const requestRepoId = repoId;
      const requestSeq = (requestSeqRef.current.get(path) ?? 0) + 1;
      requestSeqRef.current.set(path, requestSeq);
      const isStale = (): boolean =>
        scopeRef.current.repoId !== requestRepoId ||
        scopeRef.current.generation !== requestScope.generation ||
        requestSeqRef.current.get(path) !== requestSeq;

      void sendListRepoDirectory(client, requestRepoId, path)
        .then((result) => {
          if (isStale() || result.repoId !== requestRepoId || result.path !== path) return;
          setListings((prev) => new Map(prev).set(path, {
            status: "ready",
            dirs: result.dirs,
            files: result.files,
          }));
        })
        .catch((error: unknown) => {
          // A failed re-list keeps a visible listing (the next change event retries); a listing
          // still loading has nothing to keep, so it shows the failure and re-expand retries.
          if (isStale()) return;
          if (mode === "refresh" && listingsRef.current.get(path)?.status === "ready") return;
          const message = error instanceof Error ? error.message : "Unable to load directory";
          setListings((prev) => new Map(prev).set(path, {
            status: "error",
            dirs: [],
            files: [],
            error: message,
          }));
        });
    },
    [client, helloState.status, repoId]
  );

  const toggleExpanded = useCallback(
    (path: string): void => {
      const expand = !expandedDirsRef.current.has(path);
      const next = new Set(expandedDirsRef.current);
      if (expand) {
        next.add(path);
      } else {
        next.delete(path);
      }
      applyExpanded(next);
      if (expand) fetchListing(path, "initial");
    },
    [applyExpanded, fetchListing]
  );

  useEffect(
    () =>
      subscribe((event) => {
        if (event.type !== "sourceControlChanged" || event.repoId !== repoId) return;
        clearRelistTimer();
        relistTimerRef.current = setTimeout(() => {
          relistTimerRef.current = null;
          for (const path of expandedDirsRef.current) {
            fetchListing(path, "refresh");
          }
        }, RELIST_DEBOUNCE_MS);
      }),
    [clearRelistTimer, fetchListing, repoId, subscribe]
  );

  useEffect(() => clearRelistTimer, [clearRelistTimer]);

  return { expandedDirs, listings, toggleExpanded };
}
