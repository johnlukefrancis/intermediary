// Path: agent/src/util/errors.ts
// Description: Error types and helpers for the agent

export class AgentError extends Error {
  constructor(
    public readonly code: string,
    message: string,
    public readonly details?: unknown
  ) {
    super(message);
    this.name = "AgentError";
  }
}

export function isNodeError(err: unknown): err is NodeJS.ErrnoException {
  return err instanceof Error && "code" in err;
}

export function toErrorResponse(err: unknown): { code: string; message: string; details?: unknown } {
  if (err instanceof AgentError) {
    return { code: err.code, message: err.message, details: err.details };
  }
  if (err instanceof Error) {
    return { code: "INTERNAL_ERROR", message: err.message };
  }
  return { code: "UNKNOWN_ERROR", message: String(err) };
}
