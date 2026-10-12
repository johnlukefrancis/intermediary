# Repository Overview Architecture
Updated on: 2026-10-12
Owners: JL · Agents
Depends on: ADR-000, ADR-005, ADR-007, ADR-008, ADR-009

Overview is one repository-labelled feed of recent work beside a read-only Git
column, using the full main surface. Its product contract is [the design](../design/overview_design.md).
It monitors filesystem activity and does not infer agent identity or completion.

## Ownership

| Concern | Owner | Contract |
| --- | --- | --- |
| Navigation | `use_app_navigation.ts`, `use_deck_section.ts`, TS/Rust `uiState.mainView` | Overview is a separate persisted destination. Open Stream selects the exact root and Stream mode; handset opens Files. `SOURCE ›` persists the SOURCE rail and carries a `rail` deck landing, read once when the repo tab mounts, so handset lands on SOURCE too. |
| Dispatch | `use_activity_host.ts`, Stream registry | One app-lifetime subscription supplies Stream and Overview. Exact configured root path gates activity. Removed/replaced roots dispose old state; Stream bindings include root identity. |
| Compatibility | `lib/agent/activity_support.ts`, host WSL hello routing | Root-identified activity requires 0.1.24. Host refuses older external WSL hello results before accepting cached handshake success. |
| History | `use_overview_history.ts` | Refresh connected roots on hello/resume/backend transition, with host roots first and one shared request slot across effect generations. Disconnected WSL roots wait for reconnection; a timed-out but connected backend retains its recovery requests. Explicit recovery retries prior failures. Captured root identity rejects stale failures. |
| Capture | `overview_store.ts`, `overview_activity.ts` | Six recent paths and twelve settled edits per root; immutable publication after event-triggered 80 ms batching. Stable edit IDs span roots and replacements. No retained intake queue. |
| Excerpts | `overview_edit.ts` | Shared patch parser; six changed lines of 180 characters, plus metadata. No file/image reads or Blob URLs. |
| Images | `hooks/images`, `lib/images`, Overview target projection | Shared revision-bound thumbnail loader with Stream. Visible targets retain at most twenty-four slots, including available previous revisions, with two reads in flight; repository/root/path scopes before/after. Source byte/pixel gates and URL cleanup remain shared. |
| Membership | `overview_focus.ts`, `lastWorkedAt` | Live changes/deltas and persisted activity timestamps qualify roots for fifteen minutes. Watch health and tab selection never imply work. History timestamps are consumed before presentation truncation. |
| Reading | `use_overview.ts`, `use_overview_follow.ts` | Capture is continuous. Scroll away from the tail holds reading position; keyboard focus and the file workspace suspend follow. One visible five-second age clock. |
| Selection | `use_overview_selection.ts`, `overview_view.tsx` | One filter owns `working`, `selection` and `shown` for the toolbar, the feed and the Git column. `working` changes identity only when membership does, never on the age clock. |
| Feed | `components/overview`, `styles/overview` | Chronological entries, at most eighty visible, repository accents/labels, bounded excerpts, history/observation rows and concise availability notices. The toolbar strip above both columns holds the focus chips. |
| Git column | `components/overview/git`, `overview_git.css`, source-control status registry | Desktop uses the shared `ThreeColumn` grid and `railWidthPercent`; handset uses a FEED/GIT deck. The column retains every working repository, so chip switches start no read, and renders only `shown`. It never writes. |
| Workspace | `overview_view.tsx`, `overview_workspace.tsx`, `use_repo_workspace.ts`, `repo_workspace_panel.tsx` | The target is `{kind:"file",path}` or `{kind:"diff",entry}`. Files open the actual current file; diffs open through `openDiff` (text, image, staged, conflict). Keys are repository, root, kind, area, path and original path; close/removal/replacement invalidates reads. The feed and column stay mounted and hidden behind the workspace. |
| Empty states | `empty_state_panel.tsx` | One centered glass presentation supplies Overview and the existing no-repositories screen. |

Detailed Stream stores retain their four-store LRU. Overview creates no extra
WebSocket, filesystem watcher, Git poll or background repository workspace. Git
status comes from the one source-control status registry the SOURCE rail reads.
Terminal groups retain their existing app-level ownership across navigation.

## Evidence and recovery

Settled edit content belongs to its delta. A raw path notification has no inferred
causal join to a preview. History never creates an edit excerpt or live signal.
New MRU updates stamp observation time independently of file mtime; legacy saved
activity can retain older mtime-derived timestamps. Root-mismatched saved history
is refused before normalization or resaving.

Connection, successful hello, WSL backend state and root-scoped watcher errors
determine availability. Snapshot refresh refuses a finished watcher; a successful
root-matched snapshot can confirm a watch omitted by a cached host hello.
Ordinary traffic does not erase a watcher error. Genuine backend recovery can
re-arm matching snapshot proof. WSL recovery never resets host sequence evidence.

Initial watch startup cannot lose coverage it never established. A later loss of
an established watch, sequence gaps and withheld/dropped detail are disclosed.
Label-only rehello does not invent a coverage gap. Monitoring failures remain
visible even when no repositories qualify for the feed.

## Crosstalk companion

`src-tauri/src/lib/crosstalk` owns the read-only Crosstalk event observer and
the transparent `crosstalk` window, owned by the main native window. The main
frontend's `use_crosstalk.ts` supplies Overview visibility, theme and configured
repository styles. The dedicated `app/src/crosstalk` entrypoint owns the three
visible cards, held lifetime, expansion and motion. It does not mount app providers,
open agent sockets or consume repository activity.

`conversation.ts` retains the latest displayed message for up to sixteen exact
participant pairs. A reverse-direction arrival within five minutes recalls that
message only when its card has gone; the recalled text is a compact part of the
new card. Pair direction supplies recent context, not a provider thread/reply ID.
No recursive transcript is retained. Navigation and native minimization clear
both visible cards and pair context.

One `notify` directory watcher coalesces file-change notifications through a
one-slot channel. Its single worker starts at EOF, retains partial JSONL records,
reads at most 256 KiB per wake and selects at most three new message events.
Oversized bursts/records are skipped with an application log warning. Status
records remain silent. Cancellation wakes and joins the worker at app exit.
The source is `CROSSTALK_ROOT/events.jsonl`, or local application data's
`crosstalk/events.jsonl`; absence of the provider is reported in the application
log/console without affecting repository observation. Crosstalk owns the schema
and the canonical delivery record; the native shell never writes to its store.

Sender registration is a bounded read of the exact safe address filename. Its cwd
selects the longest matching configured root; an unambiguous folder identity also
matches a Windows/WSL twin. Group members use the group's configured accent.
Unknown or ambiguous identities use the default app accent. Main config changes
propagate into already visible cards.
The address suffix supplies a muted `#tag` beside the repository name in both
the message and recalled-context headers, distinguishing sessions in one root.

Placement uses physical window and monitor work-area bounds, scaled from the
main window's DPI. The surface chooses left, right or inside, never takes focus,
has no taskbar entry and follows the owner's native lifetime. Its only core
capabilities are event subscription/unsubscription; its custom commands validate
the caller window. Message text is rendered as text. CSP and file access scopes
are unchanged.

On Windows the shell applies visibility through `ShowWindow` on the UI thread,
using `SW_SHOWNOACTIVATE` for arrival. Windows can restore an owned popup while
Tao's cached visibility still says hidden; native visibility remains the sole
authority so an empty surface is hidden again after owner restoration.

The product icons are bundled original-colour SVGs from
[LobeHub's icon library](https://github.com/lobehub/lobe-icons), with the MIT
licence embedded in each asset. They are the Codex purple cloud and Claude Code
orange mascot, not the corporate marks. External URLs are never loaded at runtime.
The macOS-only Tauri private-API feature supplies transparent child-window support;
this does not broaden the production CSP or frontend capabilities.

## Limits

Memory scales with configured roots and fixed per-root bounds, not total edit
count. The file workspace retains one bounded feed snapshot while open. The feed is not a durable journal.
Image bytes are requested only for previewable settled revisions in the visible
feed. History remains metadata. Small originals stay pixel-exact; larger sources
become thumbnails with a maximum 1024-pixel edge. A changed file is labelled as
changed rather than showing its newer pixels under an old edit.

Windows is the native verification surface for this candidate. Portable TS/Rust
configuration and wire changes include macOS, but no native Mac witness is
inferred from Windows checks.
