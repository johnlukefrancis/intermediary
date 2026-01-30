// Path: app/src/lib/agent/connection_state.ts
// Description: Agent connection status types

export type ConnectionStatus =
  | "disconnected"
  | "connecting"
  | "connected"
  | "reconnecting";

export interface ConnectionState {
  status: ConnectionStatus;
  /** Timestamp of last successful connection */
  connectedAt: number | null;
  /** Number of reconnect attempts since last success */
  reconnectAttempts: number;
  /** Last error message if any */
  lastError: string | null;
}

export const INITIAL_CONNECTION_STATE: ConnectionState = {
  status: "disconnected",
  connectedAt: null,
  reconnectAttempts: 0,
  lastError: null,
};
