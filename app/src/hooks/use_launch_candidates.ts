// Path: app/src/hooks/use_launch_candidates.ts
// Description: Fetches launch candidates for every configured repo root in one call; refreshes on root change, focus, or demand

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { repoRootKey, type RepoConfig, type RepoRoot } from "../shared/config.js";
import {
  LaunchCandidatesByRepoSchema,
  type LaunchCandidatesByRepo,
} from "../shared/protocol_launch.js";

interface LaunchRootRequest {
  repoId: string;
  root: RepoRoot;
  configuredTarget: string | null;
}

export interface LaunchCandidatesState {
  candidatesByRepo: LaunchCandidatesByRepo;
  refresh: () => void;
}

export function useLaunchCandidates(
  repos: readonly RepoConfig[],
  distroOverride: string | null,
  launchTargets: Readonly<Record<string, string>>
): LaunchCandidatesState {
  const [candidatesByRepo, setCandidatesByRepo] = useState<LaunchCandidatesByRepo>({});
  const requestSeq = useRef(0);
  const rootsKey = JSON.stringify(repos.map((r) => [r.repoId, repoRootKey(r.root), launchTargets[r.repoId]]));
  const roots = useMemo<LaunchRootRequest[]>(
    () => repos.map((r) => ({ repoId: r.repoId, root: r.root, configuredTarget: launchTargets[r.repoId] ?? null })),
    // Unrelated repo-config edits do not refetch.
    [rootsKey]
  );

  const refresh = useCallback(() => {
    const seq = ++requestSeq.current;
    if (roots.length === 0) {
      setCandidatesByRepo({});
      return;
    }
    invoke<unknown>("launch_candidates", { roots, distroOverride })
      .then((raw) => {
        if (seq !== requestSeq.current) return;
        setCandidatesByRepo(LaunchCandidatesByRepoSchema.parse(raw));
      })
      .catch((err: unknown) => {
        console.error("[launch] Failed to list launch candidates:", err);
      });
  }, [roots, distroOverride]);

  useEffect(() => {
    refresh();
    window.addEventListener("focus", refresh);
    return () => {
      window.removeEventListener("focus", refresh);
    };
  }, [refresh]);

  return { candidatesByRepo, refresh };
}
