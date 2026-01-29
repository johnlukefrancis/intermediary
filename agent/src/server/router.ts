// Path: agent/src/server/router.ts
// Description: Request dispatch and response building for WebSocket protocol

import type { WebSocket } from "ws";
import {
  RequestEnvelopeSchema,
  type UiCommand,
  type UiResponse,
  type ResponseError,
  type EventEnvelope,
  type AgentEvent,
} from "../../../app/src/shared/protocol.js";
import { logger } from "../util/logger.js";
import { toErrorResponse } from "../util/errors.js";

export type CommandHandler = (
  command: UiCommand,
  ws: WebSocket
) => Promise<UiResponse>;

export interface Router {
  handleMessage(raw: string, ws: WebSocket): void;
  setHandler(handler: CommandHandler): void;
  sendEvent(ws: WebSocket, event: AgentEvent): void;
  broadcastEvent(event: AgentEvent): void;
  setBroadcaster(fn: (msg: string) => void): void;
}

let eventCounter = 0;

function nextEventId(): string {
  return `evt_${++eventCounter}`;
}

export function createRouter(): Router {
  let commandHandler: CommandHandler | null = null;
  let broadcaster: ((msg: string) => void) | null = null;

  function sendResponse(
    ws: WebSocket,
    requestId: string,
    payload: UiResponse
  ): void {
    const envelope = {
      kind: "response" as const,
      requestId,
      status: "ok" as const,
      payload,
    };
    ws.send(JSON.stringify(envelope));
  }

  function sendError(
    ws: WebSocket,
    requestId: string,
    error: ResponseError
  ): void {
    const envelope = {
      kind: "response" as const,
      requestId,
      status: "error" as const,
      error,
    };
    ws.send(JSON.stringify(envelope));
  }

  function sendEvent(ws: WebSocket, event: AgentEvent): void {
    const envelope: EventEnvelope = {
      kind: "event",
      eventId: nextEventId(),
      payload: event,
    };
    ws.send(JSON.stringify(envelope));
  }

  function broadcastEvent(event: AgentEvent): void {
    if (!broadcaster) {
      logger.warn("No broadcaster set, cannot broadcast event");
      return;
    }
    const envelope: EventEnvelope = {
      kind: "event",
      eventId: nextEventId(),
      payload: event,
    };
    broadcaster(JSON.stringify(envelope));
  }

  async function handleMessage(raw: string, ws: WebSocket): Promise<void> {
    let parsed: unknown;
    try {
      parsed = JSON.parse(raw);
    } catch {
      logger.warn("Invalid JSON received", { raw: raw.slice(0, 100) });
      return;
    }

    const result = RequestEnvelopeSchema.safeParse(parsed);
    if (!result.success) {
      logger.warn("Invalid request envelope", {
        error: result.error.message,
      });
      return;
    }

    const { requestId, payload } = result.data;
    logger.debug("Received command", { type: payload.type, requestId });

    if (!commandHandler) {
      sendError(ws, requestId, {
        code: "NO_HANDLER",
        message: "No command handler registered",
      });
      return;
    }

    try {
      const response = await commandHandler(payload, ws);
      sendResponse(ws, requestId, response);
    } catch (err) {
      const errorResponse = toErrorResponse(err);
      logger.error("Command handler error", {
        type: payload.type,
        error: errorResponse.message,
      });
      sendError(ws, requestId, errorResponse);
    }
  }

  return {
    handleMessage: (raw, ws) => {
      void handleMessage(raw, ws);
    },
    setHandler(handler) {
      commandHandler = handler;
    },
    sendEvent,
    broadcastEvent,
    setBroadcaster(fn) {
      broadcaster = fn;
    },
  };
}
