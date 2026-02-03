// Path: agent/src/repos/watcher_error.ts
// Description: Repo watcher error classification and user-facing event shaping

import type {
  AgentErrorCode,
  AgentErrorEvent,
} from "../../../app/src/shared/protocol.js";

const DOC_PATH = "docs/commands/fix_inotify_limits.md";

type WatcherErrorInfo = {
  code?: AgentErrorCode;
  message: string;
  rawCode?: string;
  rawMessage: string;
  docPath?: string;
};

function extractErrorDetails(err: unknown): { rawMessage: string; rawCode?: string } {
  if (err instanceof Error) {
    const code =
      typeof (err as NodeJS.ErrnoException).code === "string"
        ? (err as NodeJS.ErrnoException).code
        : undefined;
    if (code) {
      return { rawMessage: err.message, rawCode: code };
    }
    return { rawMessage: err.message };
  }
  return { rawMessage: String(err) };
}

function classifyWatcherError(err: unknown): WatcherErrorInfo {
  const { rawMessage, rawCode } = extractErrorDetails(err);

  if (rawCode === "ENOSPC") {
    return {
      code: "watcher_inotify_limit",
      message: "Repo watcher hit the inotify watch limit (ENOSPC).",
      rawCode,
      rawMessage,
      docPath: DOC_PATH,
    };
  }

  if (rawCode === "EMFILE") {
    return {
      code: "watcher_fd_limit",
      message: "Repo watcher hit the open file descriptor limit (EMFILE).",
      rawCode,
      rawMessage,
      docPath: DOC_PATH,
    };
  }

  return {
    message: "Repo watcher encountered an error.",
    rawMessage,
    ...(rawCode ? { rawCode } : {}),
  };
}

export function buildWatcherErrorEvent(repoId: string, err: unknown): AgentErrorEvent {
  const info = classifyWatcherError(err);
  return {
    type: "error",
    scope: "watcher",
    message: `${info.message} Repo: ${repoId}`,
    details: {
      repoId,
      rawMessage: info.rawMessage,
      ...(info.code ? { code: info.code } : {}),
      ...(info.docPath ? { docPath: info.docPath } : {}),
      ...(info.rawCode ? { rawCode: info.rawCode } : {}),
    },
  };
}
