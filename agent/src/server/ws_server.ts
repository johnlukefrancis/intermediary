// Path: agent/src/server/ws_server.ts
// Description: WebSocket server lifecycle on localhost:3141

import { WebSocketServer, type WebSocket } from "ws";
import { logger } from "../util/logger.js";
import type { Router } from "./router.js";

const DEFAULT_PORT = 3141;

export interface WsServerConfig {
  port?: number;
  router: Router;
}

export interface WsServer {
  start(): Promise<void>;
  stop(): Promise<void>;
  broadcast(message: string): void;
  readonly port: number;
  readonly clientCount: number;
}

export function createWsServer(config: WsServerConfig): WsServer {
  const port = config.port ?? DEFAULT_PORT;
  let wss: WebSocketServer | null = null;
  const clients = new Set<WebSocket>();

  async function start(): Promise<void> {
    return new Promise((resolve, reject) => {
      wss = new WebSocketServer({ host: "127.0.0.1", port });

      wss.on("listening", () => {
        logger.info("WebSocket server started", { port });
        resolve();
      });

      wss.on("error", (err) => {
        logger.error("WebSocket server error", { error: err.message });
        reject(err);
      });

      wss.on("connection", (ws, req) => {
        const clientIp = req.socket.remoteAddress ?? "unknown";
        logger.info("Client connected", { clientIp });
        clients.add(ws);

        ws.on("message", (data: Buffer) => {
          const raw = data.toString("utf-8");
          config.router.handleMessage(raw, ws);
        });

        ws.on("close", () => {
          logger.info("Client disconnected", { clientIp });
          clients.delete(ws);
        });

        ws.on("error", (err) => {
          logger.warn("Client error", { clientIp, error: err.message });
          clients.delete(ws);
        });
      });
    });
  }

  async function stop(): Promise<void> {
    return new Promise((resolve) => {
      if (!wss) {
        resolve();
        return;
      }

      for (const client of clients) {
        client.close();
      }
      clients.clear();

      wss.close(() => {
        logger.info("WebSocket server stopped");
        wss = null;
        resolve();
      });
    });
  }

  function broadcast(message: string): void {
    for (const client of clients) {
      if (client.readyState === client.OPEN) {
        client.send(message);
      }
    }
  }

  return {
    start,
    stop,
    broadcast,
    get port() {
      return port;
    },
    get clientCount() {
      return clients.size;
    },
  };
}
