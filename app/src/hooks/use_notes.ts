// Path: app/src/hooks/use_notes.ts
// Description: Repo or global note content hook with debounced save via Tauri commands

import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";

const SAVE_DEBOUNCE_MS = 800;
const MAX_NOTE_LENGTH = 100_000;
const NOTE_LOAD_ERROR = "Failed to load notes.";
const NOTE_SAVE_ERROR = "Failed to save notes.";

export type NoteTarget = { kind: "repo"; repoId: string } | { kind: "global" };

export interface NoteState {
  content: string;
  isLoading: boolean;
  error: string | null;
  onChange: (value: string) => void;
}

function noteKeyOf(target: NoteTarget): string {
  return target.kind === "global" ? "global" : `repo:${target.repoId}`;
}

export function useNotes(target: NoteTarget): NoteState {
  const noteKey = noteKeyOf(target);
  const [content, setContent] = useState("");
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const saveTimeoutRef = useRef<number | null>(null);
  const isDirtyRef = useRef(false);
  const dirtyTokenRef = useRef(0);
  const contentRef = useRef(content);
  const targetRef = useRef(target);
  const noteKeyRef = useRef(noteKey);

  // Keep refs in sync
  useEffect(() => {
    contentRef.current = content;
  }, [content]);
  useEffect(() => {
    targetRef.current = target;
    noteKeyRef.current = noteKey;
  });

  const persistNote = useCallback(async (
    saveTarget: NoteTarget,
    text: string,
    dirtyToken: number,
  ): Promise<void> => {
    try {
      await invoke("save_note", { target: saveTarget, content: text });
      if (
        dirtyTokenRef.current === dirtyToken
        && noteKeyRef.current === noteKeyOf(saveTarget)
        && contentRef.current === text
      ) {
        isDirtyRef.current = false;
      }
      setError(null);
    } catch (err: unknown) {
      console.error("[useNotes] save failed:", err);
      setError(NOTE_SAVE_ERROR);
    }
  }, []);

  // Flush any pending save immediately for a given target.
  const flushSave = useCallback((saveTarget: NoteTarget, text: string) => {
    if (saveTimeoutRef.current !== null) {
      clearTimeout(saveTimeoutRef.current);
      saveTimeoutRef.current = null;
    }
    if (!isDirtyRef.current) return;
    const currentToken = dirtyTokenRef.current;
    void persistNote(saveTarget, text, currentToken);
  }, [persistNote]);

  // Load note when the target changes
  useEffect(() => {
    const ac = new AbortController();
    setIsLoading(true);
    setError(null);
    setContent("");
    isDirtyRef.current = false;
    dirtyTokenRef.current = 0;

    void (async () => {
      try {
        const text = await invoke<string>("load_note", { target: targetRef.current });
        if (ac.signal.aborted) return;
        setContent(text);
        contentRef.current = text;
        setIsLoading(false);
        setError(null);
      } catch (err: unknown) {
        if (ac.signal.aborted) return;
        console.error("[useNotes] load failed:", err);
        setError(NOTE_LOAD_ERROR);
        setIsLoading(false);
      }
    })();

    return () => {
      ac.abort();
    };
  }, [noteKey]);

  // Debounced save on text change
  const onChange = useCallback(
    (value: string) => {
      const clamped = value.slice(0, MAX_NOTE_LENGTH);
      const dirtyToken = dirtyTokenRef.current + 1;
      setContent(clamped);
      contentRef.current = clamped;
      dirtyTokenRef.current = dirtyToken;
      isDirtyRef.current = true;

      if (saveTimeoutRef.current !== null) {
        clearTimeout(saveTimeoutRef.current);
      }

      const saveTarget = targetRef.current;
      saveTimeoutRef.current = window.setTimeout(() => {
        saveTimeoutRef.current = null;
        void persistNote(saveTarget, clamped, dirtyToken);
      }, SAVE_DEBOUNCE_MS);
    },
    [persistNote],
  );

  // Flush on beforeunload and on cleanup (target change / unmount)
  useEffect(() => {
    const handleUnload = (): void => {
      flushSave(targetRef.current, contentRef.current);
    };
    window.addEventListener("beforeunload", handleUnload);
    return () => {
      window.removeEventListener("beforeunload", handleUnload);
      flushSave(targetRef.current, contentRef.current);
    };
  }, [flushSave, noteKey]);

  // Cleanup timeout on unmount
  useEffect(() => {
    return () => {
      if (saveTimeoutRef.current !== null) {
        clearTimeout(saveTimeoutRef.current);
      }
    };
  }, []);

  return { content, isLoading, error, onChange };
}
