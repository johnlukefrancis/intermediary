// Path: agent/src/util/logger.ts
// Description: Structured logging to console with ISO timestamps

import * as fs from "node:fs/promises";
import * as path from "node:path";

export type LogLevel = "debug" | "info" | "warn" | "error";

interface LogEntry {
  level: LogLevel;
  ts: string;
  msg: string;
  data?: Record<string, unknown>;
}

const LEVEL_ORDER: Record<LogLevel, number> = {
  debug: 0,
  info: 1,
  warn: 2,
  error: 3,
};

let minLevel: LogLevel = "info";
const logDir = path.join(process.cwd(), "logs");
const logFile = path.join(logDir, "agent_latest.log");
let logInitPromise: Promise<void> | null = null;

function ensureLogFile(): Promise<void> {
  if (!logInitPromise) {
    logInitPromise = fs
      .mkdir(logDir, { recursive: true })
      .then(() => fs.appendFile(logFile, ""))
      .catch((err: unknown) => {
        console.error(
          `{"level":"error","msg":"Failed to init log file","error":"${String(err)}"}`
        );
      });
  }
  return logInitPromise;
}

function writeLogFile(line: string): void {
  void ensureLogFile().then(() => fs.appendFile(logFile, `${line}\n`));
}

export function setLogLevel(level: LogLevel): void {
  minLevel = level;
}

function log(level: LogLevel, msg: string, data?: Record<string, unknown>): void {
  if (LEVEL_ORDER[level] < LEVEL_ORDER[minLevel]) {
    return;
  }

  const entry: LogEntry = {
    level,
    ts: new Date().toISOString(),
    msg,
    ...(data !== undefined ? { data } : {}),
  };

  const output = JSON.stringify(entry);
  writeLogFile(output);
  if (level === "error") {
    console.error(output);
  } else {
    console.log(output);
  }
}

export const logger = {
  debug: (msg: string, data?: Record<string, unknown>) => { log("debug", msg, data); },
  info: (msg: string, data?: Record<string, unknown>) => { log("info", msg, data); },
  warn: (msg: string, data?: Record<string, unknown>) => { log("warn", msg, data); },
  error: (msg: string, data?: Record<string, unknown>) => { log("error", msg, data); },
};
