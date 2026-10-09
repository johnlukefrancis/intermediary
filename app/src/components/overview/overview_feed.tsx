// Path: app/src/components/overview/overview_feed.tsx
// Description: One chronological repository-labelled Overview feed with tail-follow

import type React from "react";
import { useMemo, useRef, useState } from "react";
import type { RepoConfig, TabTheme } from "../../shared/config.js";
import type { ImageTiles } from "../../hooks/images/use_image_tiles.js";
import { DEFAULT_ACCENT_HEX, hexToAccentCssVars } from "../../lib/theme/accent_utils.js";
import type { OverviewFeedEntry as FeedEntry } from "./overview_feed_entries.js";
import { hasImagePreview } from "./overview_settled_edit.js";
import { OverviewFeedEntry } from "./overview_feed_entry.js";
import { useOverviewFollow } from "./use_overview_follow.js";
import { StreamFollowPill } from "../stream/stream_follow_pill.js";
import "../../styles/overview/overview_image.css";

interface OverviewFeedProps {
  entries: readonly FeedEntry[];
  images: ImageTiles;
  tabThemes: Readonly<Record<string, TabTheme>>;
  now: number;
  documentHidden: boolean;
  viewKey: string;
  onOpenRepo: (repoId: string) => void;
  onOpenFile: (repo: RepoConfig, path: string) => void;
  onDragFile: (repo: RepoConfig, path: string) => void;
  onContextMenu: (event: React.MouseEvent, repo: RepoConfig, path: string) => void;
}

function consecutiveGroups(entries: readonly FeedEntry[]): { repo: RepoConfig; entries: FeedEntry[] }[] {
  const groups: { repo: RepoConfig; entries: FeedEntry[] }[] = [];
  for (const entry of entries) {
    const tail = groups.at(-1);
    if (tail?.repo.repoId === entry.repo.repoId) tail.entries.push(entry);
    else groups.push({ repo: entry.repo, entries: [entry] });
  }
  return groups;
}

function imageRuns(entries: readonly FeedEntry[]): { key: string; images: boolean; entries: FeedEntry[] }[] {
  const runs: { key: string; images: boolean; entries: FeedEntry[] }[] = [];
  for (const entry of entries) {
    const images = entry.kind === "edit" && hasImagePreview(entry.edit);
    const tail = runs.at(-1);
    if (images && tail?.images) tail.entries.push(entry);
    else runs.push({ key: entry.key, images, entries: [entry] });
  }
  return runs;
}

export function OverviewFeed({ entries, images, tabThemes, now, documentHidden, viewKey, onOpenRepo, onOpenFile, onDragFile, onContextMenu }: OverviewFeedProps): React.JSX.Element {
  const scroller = useRef<HTMLDivElement | null>(null);
  const [focused, setFocused] = useState(false);
  const follow = useOverviewFollow(scroller, entries, viewKey, focused || documentHidden);
  const groups = useMemo(() => consecutiveGroups(entries), [entries]);
  const onBlur = (event: React.FocusEvent): void => {
    if (!(event.relatedTarget instanceof Node) || !event.currentTarget.contains(event.relatedTarget)) setFocused(false);
  };
  return (
    <div className="overview-feed-shell">
      <div ref={scroller} className="overview-feed" role="log" aria-live="off" aria-label="Repository change feed"
        tabIndex={0} onScroll={follow.onScroll} onFocusCapture={(event) => { if (event.target !== event.currentTarget) setFocused(true); }} onBlurCapture={onBlur}>
        {groups.map(({ repo, entries: groupEntries }) => {
          const theme = tabThemes[repo.groupId ?? repo.repoId];
          const style = hexToAccentCssVars(theme?.accentHex ?? DEFAULT_ACCENT_HEX) as React.CSSProperties;
          const group = repo.groupLabel ?? repo.groupId;
          return <section key={groupEntries[0]?.key} className="overview-feed-group" style={style}
            data-repo-id={repo.repoId} aria-label={`${repo.label} changes`}>
            <header className="overview-repo-label">
              <button type="button" className="overview-repo-link" onClick={() => { onOpenRepo(repo.repoId); }}
                title={`Open ${repo.label} Stream`} aria-label={`Open ${repo.label} Stream`}>
                <span className="overview-repo-label__dot" aria-hidden="true" />
                <strong title={`${repo.root.kind.toUpperCase()} · ${repo.root.path}`}>{repo.label}</strong>
                {group?.trim() && !repo.label.toLowerCase().includes(group.trim().toLowerCase())
                  && <span className="overview-repo-label__group">{group}</span>}
                <span className="overview-repo-link__rule" aria-hidden="true" />
                <span className="overview-repo-link__go" aria-hidden="true"><span>Stream</span>
                  <svg className="overview-repo-link__arrow" viewBox="0 0 24 24"><path d="M5 12h14M13 6l6 6-6 6" /></svg>
                </span>
              </button>
            </header>
            {imageRuns(groupEntries).map((run) => <div key={run.key} className="overview-entry-run" data-images={run.images}>
              {run.entries.map((entry) => <OverviewFeedEntry key={entry.key} entry={entry} now={now}
                imageTile={images.byKey.get(entry.key)} onOpenFile={onOpenFile} onDragFile={onDragFile} onContextMenu={onContextMenu} />)}
            </div>)}
          </section>;
        })}
      </div>
      {follow.unread > 0 && <StreamFollowPill unread={follow.unread} onResume={follow.resume} />}
    </div>
  );
}
