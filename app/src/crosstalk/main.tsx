// Path: app/src/crosstalk/main.tsx
// Description: Sole bounded speech-bubble renderer in the transparent companion window

import { useCallback, useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Bubble } from "./bubble.js";
import { receive, type Conversation } from "./conversation.js";
import type { Message, Presentation } from "./identity.js";
import "../styles/tokens.css";
import "../styles/theme_dark.css";
import "../styles/theme_warm.css";
import "../styles/theme_light.css";
import "./speech.css";

function SpeechSurface(): React.JSX.Element {
  const [conversation, setConversation] = useState<Conversation>({ cards: [], recent: [] });
  const messages = conversation.cards;
  const [presentation, setPresentation] = useState<Presentation>({ active: false, theme: "dark", repos: [] });
  const [side, setSide] = useState("left");
  const root = useRef<HTMLDivElement>(null);
  const held = useRef(new Set<string>());
  const onRemove = useCallback((id: string) => {
    held.current.delete(id);
    setConversation(state => ({ ...state, cards: state.cards.filter(item => item.id !== id) }));
  }, []);
  const onHold = useCallback((id: string, value: boolean) => {
    if (value) held.current.add(id); else held.current.delete(id);
  }, []);

  useEffect(() => {
    let disposed = false;
    let active = false;
    const cleanup: (() => void)[] = [];
    const update = (next: Presentation): void => {
      if (disposed) return;
      active = next.active;
      setPresentation(next);
      document.documentElement.dataset.themeMode = next.theme;
      if (!next.active) { setConversation({ cards: [], recent: [] }); held.current.clear(); }
    };
    const start = async (): Promise<void> => {
      cleanup.push(await listen<Presentation>("crosstalk-presentation", event => { update(event.payload); }));
      cleanup.push(await listen<string>("crosstalk-side", event => { if (!disposed) setSide(event.payload); }));
      cleanup.push(await listen<Message>("crosstalk-message", ({ payload }) => {
        if (disposed || !active) return;
        const now = performance.now();
        setConversation(state => receive(state, payload, held.current, now));
      }));
      update(await invoke<Presentation>("crosstalk_ready"));
      if (disposed) cleanup.forEach(unlisten => { unlisten(); });
    };
    // TODO(ts-precision): Tauri event registration and invoke failures are external errors.
    void start().catch((error: unknown) => { console.error("[crosstalk]", error); });
    return () => { disposed = true; cleanup.forEach(unlisten => { unlisten(); }); };
  }, []);

  useEffect(() => {
    const element = root.current;
    if (!element) return;
    let pending = Promise.resolve();
    const observer = new ResizeObserver(() => {
      const height = messages.length === 0 ? 0 : element.scrollHeight;
      pending = pending.then(async () => { await invoke("crosstalk_resize", { height }); })
        // TODO(ts-precision): Tauri command rejections are an external error seam.
        .catch((error: unknown) => { console.error("[crosstalk]", error); });
    });
    observer.observe(element);
    return () => { observer.disconnect(); };
  }, [messages.length]);

  return <div ref={root} className="speech-stack" data-side={side} aria-label="Agent conversations" aria-live="polite">
    {messages.map(message => <Bubble key={message.id} message={message} repos={presentation.repos} onRemove={onRemove} onHold={onHold} />)}
  </div>;
}

const container = document.getElementById("root");
if (!container) throw new Error("Crosstalk root not found");
createRoot(container).render(<SpeechSurface />);
