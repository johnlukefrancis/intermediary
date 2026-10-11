// Path: app/src/lib/agent/agent_client.ts
// Description: WebSocket client with reconnection and message correlation

import {
  ProtocolEnvelopeSchema,
  type RequestEnvelope,
  type UiCommand,
  type UiResponse,
  type AgentEvent,
} from "../../shared/protocol.js";
import {
  type ConnectionState,
  INITIAL_CONNECTION_STATE,
} from "./connection_state.js";
import { normalizeLegacyEnvelope } from "./agent_client_legacy.js";
import { getRequestTimeoutMs } from "./agent_request_timeouts.js";
import { AgentResponseError } from "./error_codes.js";

const RECONNECT_BASE_MS = 1_000;
const RECONNECT_MAX_MS = 30_000;

export interface AgentClientConfig {
  host: string;
  port: number;
  authToken: string;
  onConnectionChange: (state: ConnectionState) => void;
  onEvent: (event: AgentEvent) => void;
}

interface PendingRequest {
  resolve: (response: UiResponse) => void;
  reject: (error: Error) => void;
  timeout: ReturnType<typeof setTimeout>;
  cleanup: () => void;
}

export interface AgentClient {
  connect(): void;
  disconnect(): void;
  send<T extends UiResponse>(command: UiCommand): Promise<T>;
  send<T extends UiResponse>(command: Extract<UiCommand, { type: "inspectBundleEntry" }>, signal: AbortSignal): Promise<T>;
  getConnectionState(): ConnectionState;
}

export function createAgentClient(config: AgentClientConfig): AgentClient {
  let ws: WebSocket | null = null;
  let connectionState: ConnectionState = { ...INITIAL_CONNECTION_STATE };
  let reconnectTimeout: ReturnType<typeof setTimeout> | null = null;
  const pendingRequests = new Map<string, PendingRequest>();

  function setConnectionState(state: ConnectionState): void {
    connectionState = state;
    config.onConnectionChange(state);
  }

  function connect(): void {
    if (ws && (ws.readyState === WebSocket.OPEN || ws.readyState === WebSocket.CONNECTING)) {
      return;
    }

    setConnectionState({
      ...connectionState,
      status: "connecting",
      lastError: null,
    });

    const resolvedHost = config.host === "localhost" ? "127.0.0.1" : config.host;
    const url = new URL(`ws://${resolvedHost}:${config.port}`);
    url.searchParams.set("token", config.authToken);
    ws = new WebSocket(url.toString());

    ws.onopen = handleOpen;
    ws.onclose = handleClose;
    ws.onerror = handleError;
    ws.onmessage = handleMessage;
  }

  function disconnect(): void {
    if (reconnectTimeout) {
      clearTimeout(reconnectTimeout);
      reconnectTimeout = null;
    }
    if (ws) {
      ws.onclose = null;
      ws.close();
      ws = null;
    }
    rejectAllPending("Client disconnected");
    setConnectionState({
      status: "disconnected",
      connectedAt: null,
      reconnectAttempts: 0,
      lastError: null,
    });
  }

  function handleOpen(): void {
    setConnectionState({
      status: "connected",
      connectedAt: Date.now(),
      reconnectAttempts: 0,
      lastError: null,
    });
  }

  function handleClose(): void {
    ws = null;
    rejectAllPending("Connection closed");
    scheduleReconnect();
  }

  function handleError(event: Event): void {
    const message = event instanceof ErrorEvent ? event.message : "Connection error";
    console.error("[AgentClient] WebSocket error", {
      host: config.host,
      port: config.port,
      message,
    });
    setConnectionState({
      ...connectionState,
      lastError: message,
    });
  }

  function handleMessage(event: MessageEvent<string>): void {
    let parsed: unknown;
    try {
      parsed = JSON.parse(event.data);
    } catch {
      console.error("[AgentClient] Invalid JSON received");
      return;
    }

    const normalized = normalizeLegacyEnvelope(parsed);
    const result = ProtocolEnvelopeSchema.safeParse(normalized);
    if (!result.success) {
      console.error("[AgentClient] Invalid envelope:", result.error.message);
      return;
    }

    const envelope = result.data;

    if (envelope.kind === "response") {
      const pending = pendingRequests.get(envelope.requestId);
      if (pending) {
        pendingRequests.delete(envelope.requestId);
        clearTimeout(pending.timeout);
        pending.cleanup();

        if (envelope.status === "ok") {
          pending.resolve(envelope.payload);
        } else {
          pending.reject(
            new AgentResponseError(
              envelope.error.code,
              envelope.error.message,
              envelope.error.details
            )
          );
        }
      }
    } else if (envelope.kind === "event") {
      config.onEvent(envelope.payload);
    }
  }

  function scheduleReconnect(): void {
    const attempts = connectionState.reconnectAttempts;
    const delay = Math.min(RECONNECT_BASE_MS * Math.pow(2, attempts), RECONNECT_MAX_MS);

    setConnectionState({
      ...connectionState,
      status: "reconnecting",
      reconnectAttempts: attempts + 1,
    });

    reconnectTimeout = setTimeout(() => {
      reconnectTimeout = null;
      connect();
    }, delay);
  }

  function rejectAllPending(reason: string): void {
    for (const [, pending] of pendingRequests) {
      clearTimeout(pending.timeout);
      pending.cleanup();
      pending.reject(new Error(reason));
    }
    pendingRequests.clear();
  }

  function send<T extends UiResponse>(command: UiCommand, signal?: AbortSignal): Promise<T> {
    return new Promise((resolve, reject) => {
      if (!ws || ws.readyState !== WebSocket.OPEN) {
        reject(new Error("Not connected"));
        return;
      }

      if (signal?.aborted) { reject(new DOMException("Inspection cancelled", "AbortError")); return; }
      const socket = ws;
      const requestId = crypto.randomUUID();
      const cancelRemote = (): void => {
        if (signal && socket.readyState === WebSocket.OPEN) {
          socket.send(JSON.stringify({ kind: "cancel", requestId }));
        }
      };
      const abort = (): void => {
        const pending = pendingRequests.get(requestId);
        if (!pending) return;
        pendingRequests.delete(requestId);
        clearTimeout(pending.timeout);
        pending.cleanup();
        cancelRemote();
        reject(new DOMException("Inspection cancelled", "AbortError"));
      };
      const cleanup = (): void => { signal?.removeEventListener("abort", abort); };

      const timeout = setTimeout(() => {
        pendingRequests.delete(requestId);
        cleanup();
        cancelRemote();
        reject(new Error(`Request timeout: ${command.type}`));
      }, getRequestTimeoutMs(command));

      pendingRequests.set(requestId, {
        resolve: resolve as (r: UiResponse) => void,
        reject,
        timeout,
        cleanup,
      });
      signal?.addEventListener("abort", abort, { once: true });

      const envelope: RequestEnvelope = {
        kind: "request",
        requestId,
        payload: command,
      };

      ws.send(JSON.stringify(envelope));
    });
  }

  return {
    connect,
    disconnect,
    send,
    getConnectionState: () => connectionState,
  };
}
