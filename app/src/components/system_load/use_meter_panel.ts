// Path: app/src/components/system_load/use_meter_panel.ts
// Description: Hover-intent, grace-close, focus and click-to-pin state for the system-load detail panel

import { useCallback, useEffect, useRef, useState, type RefObject } from "react";
import type { MeterKey } from "../../lib/system_load/meters.js";

const OPEN_DELAY_MS = 120;
const CLOSE_GRACE_MS = 200;

export interface MeterPanelState {
  openKey: MeterKey | null;
  pinned: boolean;
  enterMeter: (key: MeterKey) => void;
  focusMeter: (key: MeterKey) => void;
  clickMeter: (key: MeterKey) => void;
  enterPanel: () => void;
  leave: () => void;
  blurMeter: () => void;
}

/** `insideRefs` bound the cluster and panel: a pointerdown outside them releases a pin. */
export function useMeterPanel(insideRefs: readonly RefObject<HTMLElement>[]): MeterPanelState {
  const [openKey, setOpenKey] = useState<MeterKey | null>(null);
  const [pinned, setPinned] = useState(false);
  const openRef = useRef<MeterKey | null>(null);
  const pinnedRef = useRef(false);
  const timer = useRef<number | undefined>(undefined);
  openRef.current = openKey;
  pinnedRef.current = pinned;

  const clearTimer = (): void => {
    window.clearTimeout(timer.current);
    timer.current = undefined;
  };
  const close = useCallback((): void => {
    clearTimer();
    setOpenKey(null);
    setPinned(false);
  }, []);

  const enterMeter = useCallback((key: MeterKey): void => {
    clearTimer();
    if (pinnedRef.current) return;
    if (openRef.current) {
      setOpenKey(key);
      return;
    }
    timer.current = window.setTimeout(() => { setOpenKey(key); }, OPEN_DELAY_MS);
  }, []);

  const focusMeter = useCallback((key: MeterKey): void => {
    clearTimer();
    if (!pinnedRef.current) setOpenKey(key);
  }, []);

  const clickMeter = useCallback((key: MeterKey): void => {
    clearTimer();
    if (pinnedRef.current && openRef.current === key) {
      setPinned(false);
      return;
    }
    setOpenKey(key);
    setPinned(true);
  }, []);

  const enterPanel = useCallback((): void => { clearTimer(); }, []);

  const leave = useCallback((): void => {
    clearTimer();
    if (pinnedRef.current) return;
    timer.current = window.setTimeout(() => { setOpenKey(null); }, CLOSE_GRACE_MS);
  }, []);

  const blurMeter = useCallback((): void => {
    if (!insideRefs.some((ref) => ref.current?.matches(":hover"))) leave();
  }, [insideRefs, leave]);

  useEffect(() => {
    if (!openKey) return;
    const onKey = (event: KeyboardEvent): void => {
      if (event.key === "Escape") close();
    };
    const onPointer = (event: PointerEvent): void => {
      const target = event.target;
      if (!(target instanceof Node)) return;
      if (insideRefs.some((ref) => ref.current?.contains(target))) return;
      close();
    };
    document.addEventListener("keydown", onKey);
    document.addEventListener("pointerdown", onPointer);
    return () => {
      document.removeEventListener("keydown", onKey);
      document.removeEventListener("pointerdown", onPointer);
    };
  }, [openKey, close, insideRefs]);

  useEffect(() => clearTimer, []);

  return { openKey, pinned, enterMeter, focusMeter, clickMeter, enterPanel, leave, blurMeter };
}
