// Path: app/src/lib/agent/transient_wsl_error.ts
// Description: Detect transient WSL transport/bootstrap failures and compute retry delays

const RETRY_BASE_MS = 500;
const RETRY_CAP_MS = 5000;

function toLowerMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message.toLowerCase();
  }
  if (typeof error === "string") {
    return error.toLowerCase();
  }
  return "";
}

export function computeTransientRetryDelayMs(attempt: number): number {
  const safeAttempt = Math.max(0, attempt);
  const delay = RETRY_BASE_MS * Math.pow(2, safeAttempt);
  return Math.min(RETRY_CAP_MS, delay);
}

export function isTransientWslTransportError(error: unknown): boolean {
  const message = toLowerMessage(error);
  if (message.length === 0) {
    return false;
  }

  if (message.includes("wsl_backend_unavailable")) {
    return true;
  }
  if (message.includes("wsl_backend_timeout")) {
    return true;
  }
  if (message.includes("wsl backend is not available")) {
    return true;
  }
  if (message.includes("failed to connect to wsl backend")) {
    return true;
  }
  if (message.includes("connection reset")) {
    return true;
  }
  if (message.includes("actively refused")) {
    return true;
  }
  if (message.includes("handshake not finished")) {
    return true;
  }

  return false;
}
