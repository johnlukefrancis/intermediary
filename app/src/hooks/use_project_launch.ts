// Path: app/src/hooks/use_project_launch.ts
// Description: Project-launch actions over the repo candidates: effective target, launch, choose, and transient feedback

import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { RepoConfig } from "../shared/config.js";
import {
  LaunchOutcomeSchema,
  type LaunchCandidate,
  type LaunchOutcome,
} from "../shared/protocol_launch.js";
import { effectiveLaunchTarget } from "../lib/launch/launch_target.js";
import { useLaunchCandidates } from "./use_launch_candidates.js";

const OUTCOME_FLASH_MS = 1600;
const ERROR_FLASH_MS = 6000;

export type LaunchPhase = "pending" | LaunchOutcome | "error";

/** Transient state of one repo's launch; sourceId names the control that started it. */
export interface LaunchFeedback {
  phase: LaunchPhase;
  sourceId: string;
  message: string | null;
}

interface UseProjectLaunchArgs {
  repos: readonly RepoConfig[];
  distroOverride: string | null;
  launchTargets: Record<string, string>;
  setLaunchTarget: (repoId: string, relativePath: string) => void;
  onError: (repoId: string, sourceId: string) => void;
}

export interface ProjectLaunch {
  candidatesFor: (repoId: string) => readonly LaunchCandidate[];
  targetFor: (repoId: string) => LaunchCandidate | null;
  feedbackFor: (repoId: string) => LaunchFeedback | null;
  launch: (repoId: string, sourceId: string, relativePath?: string) => void;
  choose: (repoId: string, sourceId: string, relativePath: string) => void;
  refresh: () => void;
}

function errorMessage(err: unknown): string {
  return typeof err === "string" ? err : err instanceof Error ? err.message : String(err);
}

export function useProjectLaunch({
  repos,
  distroOverride,
  launchTargets,
  setLaunchTarget,
  onError,
}: UseProjectLaunchArgs): ProjectLaunch {
  const { candidatesByRepo, refresh } = useLaunchCandidates(repos, distroOverride);
  const [feedback, setFeedback] = useState<Record<string, LaunchFeedback>>({});
  const timers = useRef(new Map<string, number>());

  useEffect(() => {
    const pending = timers.current;
    return () => {
      for (const id of pending.values()) window.clearTimeout(id);
    };
  }, []);

  const showFeedback = useCallback((repoId: string, next: LaunchFeedback, clearMs: number | null) => {
    const existing = timers.current.get(repoId);
    if (existing !== undefined) window.clearTimeout(existing);
    timers.current.delete(repoId);
    setFeedback((prev) => ({ ...prev, [repoId]: next }));
    if (clearMs === null) return;
    timers.current.set(repoId, window.setTimeout(() => {
      timers.current.delete(repoId);
      setFeedback((prev) => {
        const { [repoId]: _cleared, ...rest } = prev;
        return rest;
      });
    }, clearMs));
  }, []);

  const candidatesFor = useCallback(
    (repoId: string) => candidatesByRepo[repoId] ?? [],
    [candidatesByRepo]
  );

  const targetFor = useCallback(
    (repoId: string) => effectiveLaunchTarget(candidatesFor(repoId), launchTargets[repoId]),
    [candidatesFor, launchTargets]
  );

  const feedbackFor = useCallback((repoId: string) => feedback[repoId] ?? null, [feedback]);

  const launch = useCallback((repoId: string, sourceId: string, relativePath?: string) => {
    const root = repos.find((r) => r.repoId === repoId)?.root;
    const target = relativePath ?? targetFor(repoId)?.relativePath;
    if (!root || !target) return;
    showFeedback(repoId, { phase: "pending", sourceId, message: null }, null);
    invoke<unknown>("launch_target", { root, relativePath: target, distroOverride })
      .then((raw) => {
        const outcome = LaunchOutcomeSchema.parse(raw);
        showFeedback(repoId, { phase: outcome, sourceId, message: null }, OUTCOME_FLASH_MS);
      })
      .catch((err: unknown) => {
        const message = errorMessage(err);
        console.error("[launch] Launch failed:", message);
        showFeedback(repoId, { phase: "error", sourceId, message }, ERROR_FLASH_MS);
        onError(repoId, sourceId);
      });
  }, [repos, targetFor, distroOverride, showFeedback, onError]);

  const choose = useCallback((repoId: string, sourceId: string, relativePath: string) => {
    setLaunchTarget(repoId, relativePath);
    launch(repoId, sourceId, relativePath);
  }, [setLaunchTarget, launch]);

  return { candidatesFor, targetFor, feedbackFor, launch, choose, refresh };
}
