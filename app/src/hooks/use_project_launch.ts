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
import { demoTargetFor, effectiveLaunchTarget } from "../lib/launch/launch_target.js";
import { useLaunchCandidates } from "./use_launch_candidates.js";

const OUTCOME_FLASH_MS = 1600;
const ERROR_FLASH_MS = 6000;

export type LaunchPhase = "pending" | LaunchOutcome | "error";

/** Transient feedback owned by the exact launch control. */
export interface LaunchFeedback {
  phase: LaunchPhase;
  message: string | null;
}

interface UseProjectLaunchArgs {
  repos: readonly RepoConfig[];
  distroOverride: string | null;
  launchTargets: Record<string, string>;
  demoTargets: Record<string, string>;
  setLaunchTarget: (repoId: string, target: string) => void;
  onError: (repoId: string, sourceId: string) => void;
}

export interface ProjectLaunch {
  candidatesFor: (repoId: string) => readonly LaunchCandidate[];
  targetFor: (repoId: string) => LaunchCandidate | null;
  demoFor: (repoId: string) => string | null;
  feedbackFor: (sourceId: string) => LaunchFeedback | null;
  launch: (repoId: string, sourceId: string, target?: string) => void;
  choose: (repoId: string, sourceId: string, target: string) => void;
  refresh: () => void;
}

function errorMessage(err: unknown): string {
  return typeof err === "string" ? err : err instanceof Error ? err.message : String(err);
}

export function useProjectLaunch({
  repos,
  distroOverride,
  launchTargets,
  demoTargets,
  setLaunchTarget,
  onError,
}: UseProjectLaunchArgs): ProjectLaunch {
  const { candidatesByRepo, refresh } = useLaunchCandidates(repos, distroOverride, launchTargets);
  const [feedback, setFeedback] = useState<Record<string, LaunchFeedback>>({});
  const timers = useRef(new Map<string, number>());

  useEffect(() => {
    const pending = timers.current;
    return () => {
      for (const id of pending.values()) window.clearTimeout(id);
    };
  }, []);

  const showFeedback = useCallback((sourceId: string, next: LaunchFeedback, clearMs: number | null) => {
    const existing = timers.current.get(sourceId);
    if (existing !== undefined) window.clearTimeout(existing);
    timers.current.delete(sourceId);
    setFeedback((prev) => ({ ...prev, [sourceId]: next }));
    if (clearMs === null) return;
    timers.current.set(sourceId, window.setTimeout(() => {
      timers.current.delete(sourceId);
      setFeedback((prev) => {
        const { [sourceId]: _cleared, ...rest } = prev;
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

  const demoFor = useCallback((repoId: string) => demoTargetFor(repos, demoTargets, repoId), [repos, demoTargets]);
  const feedbackFor = useCallback((sourceId: string) => feedback[sourceId] ?? null, [feedback]);

  const launch = useCallback((repoId: string, sourceId: string, targetOverride?: string) => {
    const root = repos.find((r) => r.repoId === repoId)?.root;
    const target = targetOverride ?? targetFor(repoId)?.target;
    if (!root || !target) return;
    showFeedback(sourceId, { phase: "pending", message: null }, null);
    invoke<unknown>("launch_target", { root, target, distroOverride })
      .then((raw) => {
        const outcome = LaunchOutcomeSchema.parse(raw);
        showFeedback(sourceId, { phase: outcome, message: null }, OUTCOME_FLASH_MS);
      })
      .catch((err: unknown) => {
        const message = errorMessage(err);
        console.error("[launch] Launch failed:", message);
        showFeedback(sourceId, { phase: "error", message }, ERROR_FLASH_MS);
        onError(repoId, sourceId);
      });
  }, [repos, targetFor, distroOverride, showFeedback, onError]);

  const choose = useCallback((repoId: string, sourceId: string, target: string) => {
    setLaunchTarget(repoId, target);
    launch(repoId, sourceId, target);
  }, [setLaunchTarget, launch]);

  return { candidatesFor, targetFor, demoFor, feedbackFor, launch, choose, refresh };
}
