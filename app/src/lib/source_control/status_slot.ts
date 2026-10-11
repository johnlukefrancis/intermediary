// Path: app/src/lib/source_control/status_slot.ts
// Description: One repository's status slot and its read lifecycle: generation guard, hidden/held deferral, transient retry

import type { SourceControlStatus } from "../../shared/protocol.js";
import { parseAgentErrorCode } from "../agent/error_codes.js";
import { computeTransientRetryDelayMs, isTransientWslTransportError } from "../agent/transient_wsl_error.js";
import { agentErrorMessage } from "../../hooks/source_control/source_control_failures.js";
import { createRefreshScheduler, type RefreshScheduler } from "../../hooks/source_control/source_control_refresh.js";
import type { StatusReadQueue } from "./status_read_queue.js";
import type { SourceControlReadPhase, StatusHostState, StatusSlotView, StatusWriter } from "./status_registry_types.js";

const REFRESH_DEBOUNCE_MS = 300;

export const EMPTY_VIEW: StatusSlotView = { status: null, phase: { kind: "waiting" } };

export interface StatusSlot {
  repoId: string;
  rootIdentity: string;
  view: StatusSlotView;
  retryAttempt: number;
  /** Bumped by every read start and every applied result; an older read's answer is dropped */
  generation: number;
  demand: number;
  actionPending: boolean;
  /** A read was requested while the document was hidden; it runs once the window is visible */
  dirty: boolean;
  writer: StatusWriter | null;
  scheduler: RefreshScheduler;
}

export interface SlotContext<C> {
  host(): StatusHostState<C>;
  /** False once the slot was evicted or replaced */
  isCurrent(slot: StatusSlot): boolean;
  read(client: C, repoId: string): Promise<SourceControlStatus>;
  setView(slot: StatusSlot, view: StatusSlotView): void;
}

export function setSlotPhase<C>(context: SlotContext<C>, slot: StatusSlot, phase: SourceControlReadPhase): void {
  if (slot.view.phase.kind === phase.kind && phase.kind !== "error") return;
  context.setView(slot, { status: slot.view.status, phase });
}

async function readSlot<C>(context: SlotContext<C>, slot: StatusSlot): Promise<void> {
  const host = context.host();
  const { client } = host;
  if (client === null || !host.ready) return;
  if (host.documentHidden) {
    slot.dirty = true;
    return;
  }
  slot.dirty = false;
  if (host.rootKinds.get(slot.repoId) === "wsl" && !host.wslOnline) {
    setSlotPhase(context, slot, { kind: "waiting" });
    return;
  }
  slot.generation += 1;
  const generation = slot.generation;
  const isStale = (): boolean => !context.isCurrent(slot) || slot.generation !== generation;
  setSlotPhase(context, slot, { kind: "loading" });
  try {
    const status = await context.read(client, slot.repoId);
    if (isStale()) return;
    slot.retryAttempt = 0;
    context.setView(slot, { status, phase: { kind: "ready" } });
    slot.writer?.acceptStatus(status);
  } catch (err: unknown) {
    if (isStale()) return;
    if (slot.writer?.acceptFailure() === true) return;
    if (isTransientWslTransportError(err)) {
      slot.scheduler.requestRefreshIn(computeTransientRetryDelayMs(slot.retryAttempt));
      slot.retryAttempt += 1;
      return;
    }
    slot.retryAttempt = 0;
    const phase: SourceControlReadPhase = { kind: "error", code: parseAgentErrorCode(err), message: agentErrorMessage(err) };
    context.setView(slot, { status: null, phase });
  }
}

export function createStatusSlot<C>(
  context: SlotContext<C>,
  queue: StatusReadQueue,
  repoId: string,
  rootIdentity: string
): StatusSlot {
  const slot: StatusSlot = {
    repoId,
    rootIdentity,
    view: EMPTY_VIEW,
    retryAttempt: 0,
    generation: 0,
    demand: 0,
    actionPending: false,
    dirty: false,
    writer: null,
    scheduler: createRefreshScheduler({
      debounceMs: REFRESH_DEBOUNCE_MS,
      run: () => queue.run(slot, () => readSlot(context, slot)),
    }),
  };
  return slot;
}

/** The slot's root changed under live demand: forget everything read from the old root */
export function resetSlotRoot<C>(context: SlotContext<C>, queue: StatusReadQueue, slot: StatusSlot, identity: string): void {
  slot.rootIdentity = identity;
  slot.generation += 1;
  slot.retryAttempt = 0;
  slot.dirty = false;
  slot.actionPending = false;
  queue.cancel(slot);
  slot.scheduler.reset();
  context.setView(slot, EMPTY_VIEW);
  if (context.host().ready) slot.scheduler.requestRefresh();
}
