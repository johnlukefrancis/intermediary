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

const REQUEST_TIMEOUT_MS = 30_000;
const RECONNECT_BASE_MS = 1_000;
const RECONNECT_MAX_MS = 30_000;

export interface AgentClientConfig {
  host: string;
  port: number;
  onConnectionChange: (state: ConnectionState) => void;
  onEvent: (event: AgentEvent) => void;
}

interface PendingRequest {
  resolve: (response: UiResponse) => void;
  reject: (error: Error) => void;
  timeout: ReturnType<typeof setTimeout>;
}

export interface AgentClient {
  connect(): void;
  disconnect(): void;
  send<T extends UiResponse>(command: UiCommand): Promise<T>;
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
    const url = `ws://${resolvedHost}:${config.port}`;
    ws = new WebSocket(url);

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

    const result = ProtocolEnvelopeSchema.safeParse(parsed);
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

        if (envelope.status === "ok") {
          pending.resolve(envelope.payload);
        } else {
          pending.reject(new Error(`${envelope.error.code}: ${envelope.error.message}`));
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
      pending.reject(new Error(reason));
    }
    pendingRequests.clear();
  }

  function send<T extends UiResponse>(command: UiCommand): Promise<T> {
    return new Promise((resolve, reject) => {
      if (!ws || ws.readyState !== WebSocket.OPEN) {
        reject(new Error("Not connected"));
        return;
      }

      const requestId = crypto.randomUUID();

      const timeout = setTimeout(() => {
        pendingRequests.delete(requestId);
        reject(new Error(`Request timeout: ${command.type}`));
      }, REQUEST_TIMEOUT_MS);

      pendingRequests.set(requestId, {
        resolve: resolve as (r: UiResponse) => void,
        reject,
        timeout,
      });

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
