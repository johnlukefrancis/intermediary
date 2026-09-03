// Path: app/src/hooks/repo_workspace_diff_loaders.ts
// Description: Diff loaders for the repo workspace hook: text patches and two-sided image snapshots

import type { AgentClient } from "../lib/agent/agent_client.js";
import {
  sendSourceControlDiff,
  sendSourceControlImageDiff,
} from "../lib/agent/messages_source_control.js";
import {
  computeTransientRetryDelayMs,
  isTransientWslTransportError,
} from "../lib/agent/transient_wsl_error.js";
import { agentErrorMessage } from "./source_control/source_control_failures.js";
import type {
  DiffWorkspaceBase,
  ImageDiffWorkspaceBase,
  RepoWorkspace,
} from "./repo_workspace_types.js";

export interface DiffLoaderContext {
  client: AgentClient;
  repoId: string;
  /** True once a newer open superseded this request: the result is dropped, not rendered */
  isStale: () => boolean;
  setWorkspace: (workspace: RepoWorkspace) => void;
}

interface RetryableLoad {
  isStale: () => boolean;
  /** Sends the request and applies the result; rejections drive the retry decision */
  attempt: () => Promise<void>;
  onFailure: (error: unknown) => void;
}

/** A dropped WSL transport hop is retried with backoff; every other failure is the user's answer. */
function loadWithTransientRetry({ isStale, attempt, onFailure }: RetryableLoad): void {
  const run = (attemptIndex: number): void => {
    void attempt().catch((err: unknown) => {
      if (isStale()) return;
      if (isTransientWslTransportError(err)) {
        setTimeout(() => {
          if (!isStale()) run(attemptIndex + 1);
        }, computeTransientRetryDelayMs(attemptIndex));
        return;
      }
      onFailure(err);
    });
  };
  run(0);
}

export function loadTextDiff(context: DiffLoaderContext, base: DiffWorkspaceBase): void {
  const { client, repoId, isStale, setWorkspace } = context;
  setWorkspace({ ...base, status: "loading" });

  loadWithTransientRetry({
    isStale,
    attempt: async () => {
      const result = await sendSourceControlDiff(
        client,
        repoId,
        base.path,
        base.area,
        base.originalPath ?? undefined
      );
      if (isStale()) return;
      setWorkspace({
        ...base,
        status: "ready",
        patch: result.patch,
        truncated: result.truncated,
        binary: result.binary,
      });
    },
    onFailure: (err) => {
      setWorkspace({ ...base, status: "error", error: agentErrorMessage(err) });
    },
  });
}

export function loadImageDiff(context: DiffLoaderContext, base: ImageDiffWorkspaceBase): void {
  const { client, repoId, isStale, setWorkspace } = context;
  setWorkspace({ ...base, status: "loading" });

  loadWithTransientRetry({
    isStale,
    attempt: async () => {
      const result = await sendSourceControlImageDiff(
        client,
        repoId,
        base.path,
        base.area,
        base.originalPath ?? undefined
      );
      if (isStale()) return;
      setWorkspace({
        ...base,
        status: "ready",
        before: result.before,
        after: result.after,
      });
    },
    onFailure: (err) => {
      setWorkspace({ ...base, status: "error", error: agentErrorMessage(err) });
    },
  });
}
