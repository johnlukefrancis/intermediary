// Path: app/src/crosstalk/bubble.tsx
// Description: Agent speech card with hover-held lifetime, expansion and exit animation

import { useEffect, useRef, useState } from "react";
import { hexToAccentCssVars } from "../lib/theme/accent_utils.js";
import { agent, repoStyle, type RepoStyle } from "./identity.js";
import type { SpeechMessage } from "./conversation.js";

const codex = { icon: new URL("./assets/codex.svg?no-inline", import.meta.url).href, name: "Codex" };
const clients: Record<string, { icon: string; name: string }> = {
  claude: { icon: new URL("./assets/claude_code.svg?no-inline", import.meta.url).href, name: "Claude Code" },
  codex,
  muse: { icon: new URL("./assets/muse.png?no-inline", import.meta.url).href, name: "Muse Code" },
};
const clientOf = (client: string | undefined) => clients[client ?? ""] ?? codex;

interface Props {
  message: SpeechMessage;
  repos: readonly RepoStyle[];
  onRemove: (id: string) => void;
  onHold: (id: string, held: boolean) => void;
}

export function Bubble({ message, repos, onRemove, onHold }: Props): React.JSX.Element {
  const [hovered, setHovered] = useState(false);
  const [expanded, setExpanded] = useState(false);
  const [leaving, setLeaving] = useState(false);
  const remaining = useRef(14_000);
  const held = hovered || expanded;
  const sender = agent(message.from);
  const recipient = agent(message.to);
  const repo = repoStyle(message, repos);
  const echo = message.echo;
  const echoRepo = echo ? repoStyle(echo, repos) : null;
  const echoSender = echo ? agent(echo.from) : null;

  useEffect(() => {
    onHold(message.id, held);
    return () => { onHold(message.id, false); };
  }, [held, message.id, onHold]);

  useEffect(() => {
    if (held || leaving) return;
    const started = performance.now();
    const timer = window.setTimeout(() => { setLeaving(true); }, remaining.current);
    return () => {
      window.clearTimeout(timer);
      remaining.current = Math.max(1200, remaining.current - (performance.now() - started));
    };
  }, [held, leaving]);

  useEffect(() => {
    if (!leaving) return;
    const timer = window.setTimeout(() => { onRemove(message.id); }, 300);
    return () => { window.clearTimeout(timer); };
  }, [leaving, message.id, onRemove]);

  return <article className="speech" data-leaving={leaving || undefined} data-expanded={expanded || undefined} data-echo={!!echo || undefined}
    style={hexToAccentCssVars(repo.accent) as React.CSSProperties}
    onMouseEnter={() => { setHovered(true); if (leaving) { setLeaving(false); remaining.current = 2500; } }}
    onMouseLeave={() => { setHovered(false); }}>
    {echo && echoRepo && <button className="speech__echo" aria-label={expanded ? "Collapse previous message" : "Expand previous message"}
      aria-expanded={expanded} style={hexToAccentCssVars(echoRepo.accent) as React.CSSProperties}
      onClick={() => { setExpanded(value => !value); }}>
      <img src={clientOf(echoSender?.client).icon} alt={clientOf(echoSender?.client).name} />
      <span className="speech__echo-body">
        <span className="speech__echo-heading" title={echo.from}>
          <span className="speech__echo-repo">{echoRepo.label}</span>
          {echoSender?.tag && <span className="speech__tag">#{echoSender.tag}</span>}
        </span>
        <span className="speech__echo-text">{echo.text}</span>
      </span>
    </button>}
    <div className="speech__current">
      <div className="speech__icon" title={clientOf(sender.client).name}>
        <img src={clientOf(sender.client).icon} alt={clientOf(sender.client).name} />
      </div>
      <div className="speech__body">
        <header className="speech__header">
          <span className="speech__repo" title={message.cwd ?? message.from}>{repo.label}</span>
          {sender.tag && <span className="speech__tag" title={message.from}>#{sender.tag}</span>}
          <button className="speech__close" aria-label="Dismiss message" onClick={() => { setLeaving(true); }}>×</button>
        </header>
        <div className="speech__route" title={`${message.from} → ${message.to}`}>
          <span>→ {recipient.folder}</span>
        </div>
        <button className="speech__message" aria-expanded={expanded} aria-label={expanded ? "Collapse message" : "Expand message"}
          onClick={() => { setExpanded(value => !value); }}>
          <span>{message.text}</span>
        </button>
        <footer className="speech__footer">
          <time dateTime={new Date(message.sent * 1000).toISOString()}>{new Date(message.sent * 1000).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}</time>
        </footer>
      </div>
    </div>
  </article>;
}
