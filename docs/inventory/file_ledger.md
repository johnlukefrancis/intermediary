# File Ledger

Scope: src-tauri, crates, app, scripts (extensions: .cjs, .css, .d.cts, .d.mts, .d.ts, .html, .js, .mjs, .mts, .py, .rs, .scss, .ts, .tsx)

```text
app/crosstalk.html - Transparent native Crosstalk speech-bubble surface
app/index.html - index module
app/splashscreen.html - Deck-themed boot screen shown before main window is ready
app/src/app.tsx - Root component with config-driven tab state management
app/src/components/add_repo_button.tsx - "+" button for adding new repositories via directory picker
app/src/components/agent_offline_banner.tsx - Banner with diagnostics when the host agent endpoint is offline
app/src/components/auto_files_activity_stack.tsx - Consolidated activity waveform and pulse strip for Auto files rows
app/src/components/auto_files_header.tsx - Header controls for the unified Auto files panel
app/src/components/auto_files_panel.tsx - Unified Auto files panel with ranked telemetry table
app/src/components/auto_files_row.tsx - Single Auto files table row with activity telemetry
app/src/components/bundles/build_progress_button.tsx - Bundle build/cancel button with inline progress details
app/src/components/bundles/bundle_column.tsx - Main bundles column component
app/src/components/bundles/bundle_drag_ghost.tsx - Floating label that follows the pointer during an in-tree row drag
app/src/components/bundles/bundle_entries_feedback.tsx - Replace-conflict confirmation and inline error notice, shared by import and entry-action transactions
app/src/components/bundles/bundle_entry_hover.tsx - Glass inspection tooltip with source sizes and largest selected files
app/src/components/bundles/bundle_entry_rename_input.tsx - In-place rename input mounted in a ZIPS tree row's name slot
app/src/components/bundles/bundle_explorer_directory.tsx - Recursive directory node for the lazy bundle file explorer, with selection, drag, and rename
app/src/components/bundles/bundle_explorer_file_row.tsx - File row for the bundle explorer with icon-driven include/exclude toggle, selection, and rename
app/src/components/bundles/bundle_explorer_row_menu.tsx - Right-click menu for a ZIPS tree row - file/folder actions plus cut/copy/paste/rename/delete
app/src/components/bundles/bundle_explorer_tree.tsx - Top-level directory/file list for the bundle explorer, carrying drop-target attributes and tree keyboard focus
app/src/components/bundles/bundle_file_explorer.tsx - Lazy file explorer for bundle directory/file inclusion, selection, clipboard, drag-move, and rename
app/src/components/bundles/bundle_hover_provider.tsx - One delayed, cancellable metadata inspection per bundle tree
app/src/components/bundles/bundle_list.tsx - Single LATEST bundle row (inline, no header)
app/src/components/bundles/bundle_row.tsx - Individual bundle row with drag support
app/src/components/bundles/bundle_selection_panel.tsx - Bundle build controls and file explorer selection panel
app/src/components/bundles/indeterminate_checkbox.tsx - Checkbox component that supports the DOM indeterminate state
app/src/components/bundles/preset_selector.tsx - Preset tabs/buttons for bundle building
app/src/components/bundles/tree_interaction_context.tsx - Owns the ZIPS-tree click model (select/toggle/range/expand) and hands each row its interaction props
app/src/components/confirm_modal.tsx - Generic confirmation dialog with portal rendering
app/src/components/context_menu.tsx - Generic reusable right-click context menu with glass aesthetic
app/src/components/diff_workspace.tsx - Read-only unified/combined diff viewer inside the shared workspace shell; flags merge conflicts
app/src/components/diff/diff_line_rows.tsx - The one renderer of parsed diff lines as `.diff-line` rows, shared by the workspace and the stream
app/src/components/drag_error_notice.tsx - Small inline error notice for drag failures
app/src/components/empty_repo_state.tsx - Empty state UI when no repos are configured
app/src/components/empty_state_panel.tsx - Shared centered glass empty-state surface with optional details and an action
app/src/components/file_context_menu_items.ts - Shared context-menu item builders for repo-relative file actions
app/src/components/global_notes/global_notes_overlay.tsx - Full-page global markdown scratchpad layered over the current view
app/src/components/group_remove_button.tsx - Remove button for grouped repos with confirmation
app/src/components/image_diff_pane.tsx - One side of an image diff: Git-labelled header, checkerboard image slot, size footer
app/src/components/image_diff_workspace.tsx - Side-by-side before/after viewer for a changed image opened from source control
app/src/components/image_workspace.tsx - Fit-to-panel image preview surface for shared repo workspaces
app/src/components/layout/deck_section_icons.tsx - Inline 24x24 stroke glyphs for the deck section switcher (stroke supplied by CSS)
app/src/components/layout/deck_section_switcher.tsx - Segmented icon-rocker tablist switching deck sections; the host renders the matching tabpanel
app/src/components/layout/deck_splitter.tsx - Drag divider between the deck's left panel and the rail; previews the rail width while dragging and commits it on rel...
app/src/components/layout/handset_deck.tsx - Handset deck layout switching between Auto files, zip bundles, source control, and the terminal
app/src/components/layout/repo_rail.tsx - Right-rail instrument panel: slim icon-rocker header over the active rail body (zips | source | terminal)
app/src/components/layout/three_column.tsx - The one desktop shell: Auto Files or the shared workspace on the left, a drag divider, the rail on the right
app/src/components/layout/workspace_layout.tsx - The shared workspace panel (title bar + content); handset wraps it as the whole deck
app/src/components/options_overlay.tsx - Full-screen transparent overlay with options panel for app settings
app/src/components/options/agent_section.tsx - Options panel controls for host + WSL agent lifecycle
app/src/components/options/controls/tri_state_rocker.tsx - Reusable hardware-style rocker control for options
app/src/components/options/excludes_section.tsx - Excludes configuration section for the options panel
app/src/components/options/excludes/advanced_group.tsx - Collapsible checkbox group for advanced excludes options
app/src/components/options/excludes/excludes_recommendations.ts - Helpers for recommended global excludes toggles
app/src/components/options/excludes/excludes_updates.ts - Pure update helpers for global excludes toggles
app/src/components/options/excludes/use_excludes_state.ts - State and handlers for the excludes section UI
app/src/components/options/general_section.tsx - Options panel section for general app settings
app/src/components/options/layout/options_field_row.tsx - Shared label/control row primitive for responsive options fields
app/src/components/options/output_folder_section.tsx - Options panel controls for staging output folder
app/src/components/options/reset_section.tsx - Options panel reset settings section with confirmation modal
app/src/components/options/texture_picker.tsx - Small texture picker popover for tab theme selection
app/src/components/options/theme_section.tsx - Options panel theme controls (warm mode toggle + texture/accent per tab)
app/src/components/overview_view.tsx - Connects Overview's feed and Git column to configured roots, the shared filter and the full-pane workspace
app/src/components/overview/git/overview_git_column.tsx - Overview's read-only Git column: changed working trees, worktrees nested under their project group
app/src/components/overview/git/overview_git_group.tsx - One Git column group: shared section head, capped read-only rows that open the diff, and the SOURCE overflow link
app/src/components/overview/git/overview_git_project.tsx - One grouped project in the Git column: the group's accent rule over a block per worktree with changes
app/src/components/overview/git/overview_git_repo_head.tsx - Git column repository head: accent label to Stream, reread pulse, SOURCE link, branch line and state badges
app/src/components/overview/git/overview_git_repo.tsx - One Git column working-tree block (a repository, or one worktree of a project): status, state lines, groups
app/src/components/overview/overview_feed_entries_test.ts - Loss notices sit where the loss happened and age out with the working window
app/src/components/overview/overview_feed_entries.ts - Bounded chronological Overview edit, observation and coverage entries
app/src/components/overview/overview_feed_entry.tsx - One repository-bound feed entry with file opening, context menu and drag-out interactions
app/src/components/overview/overview_feed.tsx - One chronological repository-labelled Overview feed with tail-follow
app/src/components/overview/overview_file_head.tsx - Shared Overview filename-over-directory typography, icon and operation badge
app/src/components/overview/overview_handset_deck.tsx - Handset Overview deck: filter toolbar, then FEED and GIT sections in the handset chassis, both kept mounted behind th...
app/src/components/overview/overview_image_preview.tsx - Fixed-height real image preview that opens the full file workspace
app/src/components/overview/overview_panel.tsx - Overview chronological work feed for the shown repositories, with its quiet-state empty panel
app/src/components/overview/overview_presentation.ts - Overview watch-health summaries and relative edit ages
app/src/components/overview/overview_settled_edit.tsx - Observed edit with Stream-family file identity and a six-line excerpt
app/src/components/overview/overview_toolbar.tsx - Overview glass strip above both columns: active-repository filter chips and the monitoring badge
app/src/components/overview/overview_workspace.tsx - Overview's full-pane file or diff workspace using the shared repository viewers and actions
app/src/components/overview/use_overview_follow.ts - Overview tail-follow with a bounded unread set and no private timer
app/src/components/repo_workspace_panel.tsx - Repo workspace renderer for notes, text buffers, image previews, and text/image diffs
app/src/components/source_control/source_control_body.tsx - Phase-dependent body of the Source Control column: empty states or the three sections
app/src/components/source_control/source_control_column.tsx - Source Control column frame: status line, warnings, commit box, notices, body, menus, confirms
app/src/components/source_control/source_control_commit_box.tsx - Commit message textarea (Ctrl+Enter) and compact COMMIT button for the Source Control column
app/src/components/source_control/source_control_context_menu.ts - Right-click menu items for a source-control row, composed over the shared file items
app/src/components/source_control/source_control_copy.ts - Action labels, branch labels, and empty-state/error/confirm copy for the Source Control column
app/src/components/source_control/source_control_entry_face.tsx - Shared changed-path face for SOURCE rows and the Overview Git column: icon, name over dir, change badge
app/src/components/source_control/source_control_icons.tsx - Inline 24x24 stroke glyphs for source-control controls (stroke supplied by CSS)
app/src/components/source_control/source_control_row.tsx - One SOURCE changed-path row: the shared entry face plus a hover stage/unstage action
app/src/components/source_control/source_control_section_head.tsx - Shared collapsible group header: chevron, alert mark, LABEL [n], and an optional bulk-action slot
app/src/components/source_control/source_control_section.tsx - Collapsible MERGE CONFLICTS / STAGED CHANGES / CHANGES section with capped rows and a bulk action
app/src/components/source_control/source_control_status_line.tsx - Branch, ahead/behind, HEAD sha, and refresh/pull/push controls for the Source Control column
app/src/components/source_control/source_control_warnings.tsx - Conflict alert and warning rows (omitted paths, truncated status) in the Source Control column
app/src/components/status_bar.tsx - Status strip: connection LED and error ticker, system-load meters, and the options button
app/src/components/status_connection.tsx - Status-strip connection LED and label plus the rotating, marquee-on-overflow error ticker
app/src/components/stream/stream_burst_card.tsx - Fixed-height card standing in for a flood of changes: count, span, top dirs, per-op and per-kind strips, resolved
app/src/components/stream/stream_card_head.tsx - The one-line card head: badge, file icon, name over dir, +N M, baseline chip, edit count, clock
app/src/components/stream/stream_card.tsx - Memoized file card chassis: spine, head, body dispatch, click / double-click / drag / right-click / keys
app/src/components/stream/stream_follow_pill.tsx - Sticky "N NEW" button shown while the reader has scrolled away from the live tail
app/src/components/stream/stream_header_status.tsx - Stream header status when observation is interrupted or unavailable
app/src/components/stream/stream_history_row.tsx - Compact scrollback row seeding the Stream panel from the existing recent list
app/src/components/stream/stream_icons.tsx - Stroke glyphs owned by the Stream panel, drawn in the Auto files header idiom
app/src/components/stream/stream_image_strip.tsx - The image strip card: head with count and op tally, the growing-then-wrapping tile grid, clock-span footer, tile sele...
app/src/components/stream/stream_image_tile.tsx - One tile of an image strip: a 16:10 checkerboard slot sized by its column, badge, edit counter, name, the BEFORE/AFTE...
app/src/components/stream/stream_notice_row.tsx - Console-prompt notice row: honesty counters and transport state printed inline in the stream
app/src/components/stream/stream_panel.tsx - Stream panel shell with mode controls, support states, and the live scroller
app/src/components/stream/stream_scroller.tsx - The scroll container: ring in order, roving focus, follow pill, and a throttled screen-reader digest
app/src/components/stream/stream_text_body.tsx - Capped diff lines in the shared row grammar with the "+N MORE  OPEN DIFF" and "NEW FILE" footers
app/src/components/system_load/load_meter.tsx - One strip meter: wide-tracked label and a segmented LED bar tinted by the meter's severity
app/src/components/system_load/load_panel.tsx - Fixed glass detail panel ("context window") placed below its meter and clamped to the window
app/src/components/system_load/load_sparkline.tsx - Slim 60 s trend line for a detail panel; samples enter at the right edge and age leftwards
app/src/components/system_load/panel_bodies.tsx - Lean per-meter panel bodies: how much is used, a verdict, a 60 s trend and the top programs
app/src/components/system_load/panel_parts.tsx - Detail-panel headline ("how much"), severity verdict line and shared micro-bar
app/src/components/system_load/process_list.tsx - TOP PROGRAMS list: grouped label, N badge, value and a micro-bar scaled to the largest row
app/src/components/system_load/system_load_cluster.tsx - Status-strip instrument window of CPU/MEM/GPU/DSK/NET meters; owns the 1 Hz load state and its panel
app/src/components/system_load/use_meter_panel.ts - Hover-intent, grace-close, focus and click-to-pin state for the system-load detail panel
app/src/components/tab_bar.tsx - Tab navigation with grouped repo dropdown support and scroll overflow arrows
app/src/components/tab_bar/launch_control.tsx - Per-repo launch control: the play button with transient feedback and, optionally, the target-menu chevron
app/src/components/tab_bar/launch_menu.tsx - Launch-target chooser listing a repo's candidates by kind and file name, default marked
app/src/components/tab_bar/main_nav.tsx - Icon keycap cluster for app-level destinations (Overview, Notes) ahead of the repo tabs
app/src/components/tab_bar/project_launch_context.tsx - Tab-bar project-launch provider: launch state for every tab plus the target menu and error notice overlays
app/src/components/tab_bar/tab_bar_dropdowns.tsx - Dropdown panels for single-repo and grouped-repo tab-bar actions
app/src/components/tab_bar/tab_bar_items.tsx - Focused tab item renderers for single and grouped repository tabs
app/src/components/tab_remove_button.tsx - "x" button for removing repos with confirmation
app/src/components/terminal/terminal_column.tsx - TERMINAL rail body for one repo: tab strip over the imperative xterm host, plus the starting/exited/failed/empty notices
app/src/components/terminal/terminal_copy.ts - Console-prompt copy, button labels, and tooltips for the terminal column
app/src/components/terminal/terminal_exit_notice.tsx - Console-prompt notice floating over a tab that is starting, has exited, or failed to start
app/src/components/terminal/terminal_tab_strip.tsx - Tablist of one repo's terminal tabs (PWSH n, x per tab, + at the end); arrows move focus, click or Enter activates
app/src/components/text_workspace_semantics.tsx - Theme-aware semantic text layer for the workspace editor
app/src/components/text_workspace.tsx - Shared textarea surface for notes and scratch file viewing
app/src/crosstalk/bubble.tsx - Agent speech card with hover-held lifetime, expansion and exit animation
app/src/crosstalk/conversation_test.ts - Reply context must belong to the latest exact participant pair and stay bounded
app/src/crosstalk/conversation.ts - Bounded recent pair context for recalling an expired message beside its reply
app/src/crosstalk/identity_test.ts - Repository colour attribution protects nested roots and ambiguous folder names
app/src/crosstalk/identity.ts - Agent identity and configured repository accents for delivered messages
app/src/crosstalk/main.tsx - Sole bounded speech-bubble renderer in the transparent companion window
app/src/crosstalk/speech.css - Theme-native glass speech cards with edge spring and dissolve motion
app/src/hooks/agent/agent_context_types.ts - Shared context and event handler types for the agent provider hook
app/src/hooks/agent/agent_diagnostics.ts - Agent diagnostics model and helpers for connection-state-driven status bar details
app/src/hooks/agent/use_agent_probe.ts - Probe the agent port when disconnected for diagnostics
app/src/hooks/agent/use_agent_shutdown.ts - Stop the WSL agent when the app window is closing
app/src/hooks/agent/use_agent_supervisor_helpers.ts - Shared parsing and request helpers for agent supervisor hook
app/src/hooks/agent/use_agent_supervisor.ts - Manage auto-start and restart of host-agent supervision with optional Windows WSL backend
app/src/hooks/agent/wsl_transport_errors.ts - Classifies WSL transport errors and clears stale errors on explicit backend recovery events
app/src/hooks/bundles/bundle_selection_defaults.ts - Bundle preset selection initialization and default-exclusion helpers
app/src/hooks/bundles/bundle_state_types.ts - Bundle state contracts shared by bundle hooks and UI
app/src/hooks/bundles/tree_drop_targeting.ts - Shared drop-target hit-testing, dwell-to-expand, and edge auto-scroll for the ZIPS tree Contract: every function here...
app/src/hooks/bundles/use_bundle_build_actions.ts - Build and cancel actions for bundle presets
app/src/hooks/bundles/use_bundle_events.ts - Agent event handling for bundle build state
app/src/hooks/bundles/use_bundle_inclusion.ts - Bundle-selection inclusion callbacks (root/select-all/select-none/directory/file toggles) for the ZIPS explorer
app/src/hooks/bundles/use_bundle_refresh.ts - Bundle list refresh flow with transient WSL retry handling
app/src/hooks/bundles/use_directory_listings.ts - Lazy directory listing state for the bundle explorer, re-listed in place when Git reports a change
app/src/hooks/bundles/use_entry_action_request.ts - Owns the ZIPS-tree entry-action transaction: refuse-first with conflict confirmation, inline error reporting
app/src/hooks/bundles/use_import_request.ts - Owns the import transaction: refuse-first with conflict confirmation, inline error reporting
app/src/hooks/bundles/use_tree_clipboard.ts - Cut/copy clipboard state for the ZIPS tree - cut moves and clears itself after paste, copy persists
app/src/hooks/bundles/use_tree_drop_import.ts - Owns the OS drag gesture over the ZIPS tree: hit-testing, dwell-expand, edge auto-scroll, self-drag latch
app/src/hooks/bundles/use_tree_keyboard.ts - Keyboard command map for the focused ZIPS tree list (navigation, expand/collapse, clipboard, rename, delete)
app/src/hooks/bundles/use_tree_row_drag.ts - In-tree pointer-drag of a row (or the whole selection) onto a folder row or the root
app/src/hooks/bundles/use_tree_selection.ts - Row selection state for the ZIPS tree - click/ctrl/shift semantics, pruned as rows disappear
app/src/hooks/images/image_tile_records.ts - Shared image tile records, Blob ownership and stable published pixel projections
app/src/hooks/images/image_tile_thumbnail.ts - Probes and gates fetched image bytes, retaining an original small image or a bounded thumbnail Blob
app/src/hooks/images/use_image_tiles.ts - Revision-bound image reads, bounded thumbnail retention and actual unfinished-request concurrency
app/src/hooks/overview/use_crosstalk.ts - Main-window Overview visibility and theme projected to the native companion
app/src/hooks/overview/use_overview_file_actions.ts - Repository-bound Overview context menus and user-triggered drag-out actions
app/src/hooks/overview/use_overview_git_states.ts - One shared-status subscription projects visible Git lanes and their turn glow
app/src/hooks/overview/use_overview_git_totals.ts - Changed-file and conflict totals across shown repositories, read from the shared status registry
app/src/hooks/overview/use_overview_history.ts - Serial initial and reconnect history refresh across connected roots, with host roots first
app/src/hooks/overview/use_overview_images.ts - Visible Overview image metadata projected into the shared revision-bound tile loader
app/src/hooks/overview/use_overview_selection.ts - Overview's one repository filter: working set, chip selection and the shown repositories for both columns
app/src/hooks/overview/use_overview.ts - Overview subscription and visible-only age clock
app/src/hooks/repo_workspace_diff_loaders.ts - Diff loaders for the repo workspace hook: text patches and two-sided image snapshots
app/src/hooks/repo_workspace_types.ts - RepoWorkspace union (note, text, image, diff, image diff) and path helpers for the workspace hook
app/src/hooks/source_control/source_control_commands.ts - Public stage/unstage/discard/commit/push/pull command surface over the serialized action runner
app/src/hooks/source_control/source_control_counts.ts - SOURCE tab change count: distinct changed files, not area rows
app/src/hooks/source_control/source_control_failures.ts - Route an agent rejection by the effect certainty it carries, never by its error code namespace
app/src/hooks/source_control/source_control_reconcile.ts - Bounded backoff loop that resolves an action whose outcome the UI could not observe
app/src/hooks/source_control/source_control_refresh.ts - Status refresh timing owner: trailing debounce, delayed retries, in-flight dirty flag, post-mutation de-dup
app/src/hooks/source_control/source_control_types.ts - State-machine and action contract exposed by useSourceControlState
app/src/hooks/source_control/use_source_control_state.ts - SOURCE write side over the shared status registry: serialized actions, reconciliation, notices, draft
app/src/hooks/source_control/use_source_control_status.ts - Read side of source-control status: demands the repo while mounted and selects its registry slot
app/src/hooks/source_control/use_tree_decorations.tsx - Context delivering the built tree decorations to the recursive bundle explorer rows
app/src/hooks/stream/use_deferred_click.ts - A single click that fires only once the double-click grace has passed; shared by file cards and image strips
app/src/hooks/stream/use_repo_stream.ts - Binds the active repo's Stream store to React: visibility, selection filter, card and tile actions
app/src/hooks/stream/use_stream_follow.ts - Follow-scroll for the Stream scroller: tail pin, unread count while unpinned, freeze, resume
app/src/hooks/stream/use_stream_images.ts - Projects Stream cards and host visibility to the shared bounded image tile owner
app/src/hooks/terminal/use_terminal_group.ts - Subscribes a component to one repo's terminal group snapshot from the module-level registry
app/src/hooks/terminal/use_terminal_host.ts - Adopts the active terminal tab into a host element for its mount, parks on cleanup, and keeps it fitted, themed and f...
app/src/hooks/terminal/use_terminal_lifecycle.ts - App-level terminal lifecycle: closes groups of removed repos, mirrors window foreground to cursor blink, closes every...
app/src/hooks/use_activity_host.ts - One app-lifetime dispatcher for Stream and all-repository Overview activity
app/src/hooks/use_agent.tsx - Agent context provider and connection management hook
app/src/hooks/use_app_navigation.ts - App view selection, remembered repository navigation and the handset landing intent for a repo tab
app/src/hooks/use_bundle_state.ts - Per-repo bundle state management with event subscription
app/src/hooks/use_client_hello.ts - Owns the clientHello lifecycle: per-session runner, retry backoff, resync and WSL re-hello
app/src/hooks/use_config_actions_extended.ts - Extended config actions for theme, legacy starred files, and recent files limit
app/src/hooks/use_config_actions_launch.ts - Config action persisting a repo's chosen project-launch target
app/src/hooks/use_config_actions_rail.ts - Config actions for persisted panel state: the rail section, the rail width, and the left files mode
app/src/hooks/use_config_actions_remove.ts - Config actions that remove a repo or a whole group and every per-repo map entry they own
app/src/hooks/use_config_actions.ts - Core config action factory functions for repo and bundle management
app/src/hooks/use_config_storage.ts - Config persistence + loading hook for use_config
app/src/hooks/use_config.tsx - Config persistence context provider and hook
app/src/hooks/use_deck_section.ts - One owner for the deck section: persisted right rail plus the handset-only FILES flag
app/src/hooks/use_drag_out_pointer.ts - Pointer handlers for the 6 px drag-out threshold with pointer capture, shared by rows, images, and stream cards
app/src/hooks/use_drag.ts - Drag-out logic with on-demand staging
app/src/hooks/use_effective_ui_mode.ts - Derives runtime effective UI mode from preferred mode and live window state
app/src/hooks/use_file_actions.ts - Hook for OS-level file operations (reveal in file manager, open file)
app/src/hooks/use_file_selection.ts - Multi-file selection state hook with shift-range and ctrl/cmd-toggle support
app/src/hooks/use_image_blob_url.ts - Base64 image payload to a revocable Blob URL for workspace image and image-diff panes
app/src/hooks/use_launch_candidates.ts - Fetches launch candidates for every configured repo root in one call; refreshes on root change, focus, or demand
app/src/hooks/use_mode_window_bounds_persistence.ts - Persists window bounds per mode from live resize events
app/src/hooks/use_mode_window_snap.ts - Applies per-mode window bounds when the active UI mode changes
app/src/hooks/use_motion_governor.ts - Pauses motion when window is not foreground (hidden, minimized, or unfocused) to save GPU
app/src/hooks/use_notes.ts - Repo or global note content hook with debounced save via Tauri commands
app/src/hooks/use_project_launch.ts - Project-launch actions over the repo candidates: effective target, launch, choose, and transient feedback
app/src/hooks/use_repo_state.ts - Per-repo file state management with event subscription
app/src/hooks/use_repo_workspace.ts - Repo-tab workspace state for notes, text buffers, image previews, and text/image diffs
app/src/hooks/use_resume_detector.ts - Detects likely OS sleep/wake resume using time gaps plus visibility/focus signals
app/src/hooks/use_startup_ready.ts - One-shot startup handshake to reveal main window after config load
app/src/hooks/use_system_load.ts - Owns the system-load sampling cadence: one request 1 s after the last resolves, only while visible
app/src/hooks/use_tab_bar_dropdown.ts - Owns tab-bar dropdown open state, trigger containment, and anchored positioning
app/src/hooks/use_tab_bar_scroll.ts - Scroll overflow detection, snap-to-next-tab, and active-tab reveal for the tab bar track
app/src/hooks/use_worktree_add.ts - Hook for adding worktrees to existing groups or single repos
app/src/lib/agent/activity_support.ts - Shared minimum agent contract for root-identified activity in Stream and Overview
app/src/lib/agent/agent_client_inspection_test.ts - Remote hover cancellation and settled-request cleanup at the WebSocket boundary
app/src/lib/agent/agent_client_legacy.ts - Legacy hostPath/windowsPath envelope normalization for older agent payloads
app/src/lib/agent/agent_client.ts - WebSocket client with reconnection and message correlation
app/src/lib/agent/agent_request_timeouts_test.ts - Every UI request timeout exceeds its host->WSL forward budget by a fixed margin
app/src/lib/agent/agent_request_timeouts.ts - Per-command UI request timeout ladder (strictly above the agent and host->WSL budgets)
app/src/lib/agent/client_hello_runner_test.ts - Single-flight coalescing, retry backoff, WSL generation re-hello and disposal of the hello runner
app/src/lib/agent/client_hello_runner.ts - Single-flight clientHello owner for one connection session with bounded retry backoff
app/src/lib/agent/connection_state.ts - Agent connection status types
app/src/lib/agent/error_codes.ts - Typed agent response error plus accessors for its code, message, and details
app/src/lib/agent/messages_import.ts - Typed helper for sending the drag-and-drop import command
app/src/lib/agent/messages_source_control.ts - Typed helpers for sending source-control status, diff, and action commands
app/src/lib/agent/messages_worktree.ts - Typed helper for sending the ZIPS-tree worktree action command
app/src/lib/agent/messages.ts - Typed helper functions for sending agent commands
app/src/lib/agent/transient_wsl_error_test.ts - Repo request retry classification treats UI request timeouts as transient
app/src/lib/agent/transient_wsl_error.ts - Detect transient WSL transport/bootstrap failures and request timeouts; compute retry delays
app/src/lib/bundles/bundle_selection_updates_test.ts - Regression coverage for selective inclusion beneath excluded bundle ancestors
app/src/lib/bundles/bundle_selection_updates.ts - Explicit bundle inclusion changes without enumerating or enabling sibling paths
app/src/lib/bundles/bundle_selection_visibility_test.ts - Tree file rows mirror the scanner: a globally excluded file is neither enabled nor included
app/src/lib/bundles/bundle_selection_visibility.ts - Shared path visibility helpers for bundle selection state
app/src/lib/bundles/flatten_visible_tree.ts - Flattens the lazily-loaded ZIPS tree into the exact visible row order the DOM renders
app/src/lib/crosstalk/repository.ts - Exact normalized root-prefix attribution for Crosstalk sender paths
app/src/lib/diff/diff_lines_test.ts - Golden line model for parsePatch over a unified patch and a combined conflict patch
app/src/lib/diff/diff_lines.ts - Pure unified/combined patch parser and the shared diff line model
app/src/lib/files/file_feed.ts - Auto file feed filtering, ranking, and row metric helpers
app/src/lib/files/files_mode.ts - The left file panel's mode vocabulary: the live Stream plus the three table sort modes
app/src/lib/files/relative_time.ts - One formatter for "Ns/Nm/Nh/Nd ago" labels shared by the file table and the stream
app/src/lib/format_bytes.ts - Byte-count formatting shared by bundle rows and image-diff pane footers
app/src/lib/icons/file_family.ts - Extension-to-language-family mapping for file-type icon resolution
app/src/lib/icons/file_icon.css - Per-family colors and base styling for file-type icons
app/src/lib/icons/file_icons.tsx - Devicon-derived SVG path data and FileIcon component for file-type icons
app/src/lib/icons/index.ts - Barrel export for file-type icon system
app/src/lib/images/image_tile_bounds.ts - Shared image tile byte, decode, concurrency and thumbnail retention bounds
app/src/lib/images/image_tile_pixels_test.ts - Revision-bound pixels: a read is accepted only for the announced bytes and mtime; the decode gate; the thumbnail fit;...
app/src/lib/images/image_tile_pixels.ts - Shared image preview eligibility, exact revision acceptance, source pixel gate and thumbnail fit
app/src/lib/images/image_tile_targets_test.ts - Cross-repository/root BEFORE isolation and one thumbnail budget across all source identities
app/src/lib/images/image_tile_targets.ts - Image tile source identities, newest-thumbnail retention and source-scoped BEFORE pairing
app/src/lib/launch/launch_target_test.ts - Effective launch target honours a persisted choice only while it is still a candidate
app/src/lib/launch/launch_target.ts - Pure launch-target resolution: persisted choice while still a candidate, else the top-ranked candidate
app/src/lib/overview/overview_activity.ts - Bounded live observations, historical paths and settled-detail sequence accounting
app/src/lib/overview/overview_backend_test.ts - Root-scoped watcher failure and backend-version recovery regression oracles
app/src/lib/overview/overview_edit.ts - Metadata-only settled edits with bounded excerpts from the shared patch parser
app/src/lib/overview/overview_edits_test.ts - Independent feed-entry retention, global identity and immutable publication oracles
app/src/lib/overview/overview_focus_test.ts - Active-work selection rejects idle roots and file mtimes, ages out, and re-enters on work
app/src/lib/overview/overview_focus.ts - Recent-work membership from observed edits, independent of watch health and open tabs
app/src/lib/overview/overview_history_test.ts - Reconnect history orders recent paths by observed time without inventing live activity
app/src/lib/overview/overview_lane_glow.ts - Git status and observed turns jointly determine a lane's visual signal
app/src/lib/overview/overview_lanes_test.ts - Exact lane attribution, concurrent sessions and stale turns cannot fabricate completion
app/src/lib/overview/overview_lanes.ts - Bounded exact-root agent session projection for Overview lane activity
app/src/lib/overview/overview_store_test.ts - Independent oracles for all-root retention, activity identity, bounds and watcher recovery
app/src/lib/overview/overview_store.ts - App-lifetime all-repository observations with event-triggered immutable publication
app/src/lib/overview/overview_types.ts - Bounded repository activity projections for the all-repository Overview
app/src/lib/source_control/change_badges.ts - Single badge map (letter, variant, label) for every source-control change kind
app/src/lib/source_control/conflict_count.ts - Conflicts that block a commit: listed unmerged paths plus unmerged paths above the configured root
app/src/lib/source_control/status_groups.ts - Pure split of a status into the SOURCE rail's sections: merge conflicts, staged changes, changes
app/src/lib/source_control/status_read_queue.ts - FIFO gate bounding how many source-control status reads run at once across every repository
app/src/lib/source_control/status_registry_test.ts - Status registry rules: read cap, release/dispose, stale generation, demanded routing, writer, hidden defer, WSL hold,...
app/src/lib/source_control/status_registry_types.ts - Public contract of the source-control status registry: slot view, read phase, writer, host state
app/src/lib/source_control/status_registry.ts - The one app-lifetime source-control status read owner: per-repo demand, LRU-kept statuses, capped reads, writer attach
app/src/lib/source_control/status_slot.ts - One repository's status slot and its read lifecycle: generation guard, hidden/held deferral, transient retry
app/src/lib/source_control/tree_decorations.ts - Pure projection of a source-control status into per-file and rolled-up per-directory tree decorations
app/src/lib/stream/stream_bounds.ts - Every numeric bound the Stream panel obeys; the only stream module with literals
app/src/lib/stream/stream_burst_card.ts - Pure burst card arithmetic: counts per op and kind, top directories, elapsed time
app/src/lib/stream/stream_burst_detect_test.ts - Distinct-path rate detection over fileChanged arrivals: open at the threshold, close on quiet
app/src/lib/stream/stream_burst_detect.ts - Pure distinct-path rate detector over fileChanged arrivals that opens and closes bursts
app/src/lib/stream/stream_cadence_test.ts - Cadence clamp and pressure band arithmetic
app/src/lib/stream/stream_cadence.ts - Pure cadence and pressure arithmetic for the Stream conductor
app/src/lib/stream/stream_card_body_test.ts - Body cap rules: a single oversized patch keeps its head; whole older segments fall out first
app/src/lib/stream/stream_card_body.ts - Builds and extends card bodies from fileDelta payloads under the EXPAND_CAP line bound
app/src/lib/stream/stream_card_grammar_test.ts - Line selection: head of a single segment, tail across several, one hunk per segment, newest offset
app/src/lib/stream/stream_card_grammar.ts - The one authority for how a Stream card reads: badge, rail, baseline chip, line selection, clock
app/src/lib/stream/stream_image_strip_test.ts - Strip reducer rules: open, append in order whatever the gap, replace in place, remove wins, new strip on max or a car...
app/src/lib/stream/stream_image_strip.ts - Pure reducer folding image deltas into the tail strip (or replacing a tile in the newest one), else opening a new strip
app/src/lib/stream/stream_ring_apply_burst_test.ts - Burst reducer rules: open at the threshold, absorb, close on quiet into a grace, eviction fall-through, membership ca...
app/src/lib/stream/stream_ring_apply_burst.ts - Pure reducers for fileChanged arrivals: burst detection and absorption, settling, backlog collapse
app/src/lib/stream/stream_ring_apply_support.ts - Shared reducer plumbing: reduce state seed, id allocation, card replacement, notices, path lookup
app/src/lib/stream/stream_ring_apply_test.ts - Delta reducer rules: create vs extend-newest-of-path, zero-stat skip, seq gap vs restart on deltas and counters, coun...
app/src/lib/stream/stream_ring_apply.ts - Pure reducers turning fileDelta and fileDeltaCounters events into cards, merges, and honesty notices
app/src/lib/stream/stream_ring_test.ts - Ring bounds: admit and evict order, history first, expanded exempt, notice bound, static sweep
app/src/lib/stream/stream_ring.ts - Pure ring operations: admit and evict, expand, notices, burst cards, history seed, static sweep
app/src/lib/stream/stream_store_registry.ts - The per-repo store registry: LRU-bounded, visible store pinned, one event router for every store
app/src/lib/stream/stream_store_seed_test.ts - Only the repo's own events land: cross-repo snapshots never seed, a foreign fileDelta prints no card, live rings neve...
app/src/lib/stream/stream_store_support.ts - Store-side pure helpers: browser timer deps, intake cap, history seed, transport facts, the selection remap, and the...
app/src/lib/stream/stream_store_test.ts - Conductor rules under injected timers: flush, idle wake, cadence, away admits, hidden pause and collapse, intake cap,...
app/src/lib/stream/stream_store.ts - Per-repo Stream store outside React: intake buffer, reducers, the cadence conductor, and the snapshot
app/src/lib/stream/stream_strip_types.ts - Ring member contracts for an image strip: the tiles one card holds and the body each tile carries
app/src/lib/stream/stream_strip_view.ts - How an image strip's head, rail, and footer read: count label, shared directory, op tally, clock span, bytes
app/src/lib/stream/stream_tile_targets_test.ts - Tile retention arithmetic: repo-scoped keys, newest MAX_IMAGE_TILES across strips, unfetchable tiles, BEFORE keys
app/src/lib/stream/stream_tile_targets.ts - Projects Stream strip slots and repository-scoped source identities into shared image targets
app/src/lib/stream/stream_types.ts - Card, ring, snapshot, transport, and store contracts for the per-repo Stream store
app/src/lib/stream/testing/stream_fixtures.ts - Event and card builders shared by the Stream unit tests; never imported by app code
app/src/lib/system_load/format.ts - Task Manager style number formatting for system-load meters and detail panels
app/src/lib/system_load/load_state.ts - Folds each system-load snapshot into the latest reading, a 60-sample history and the paging streak
app/src/lib/system_load/meters.ts - Strip meter readings (bar level, severity, spoken value) and the plain-language panel verdicts
app/src/lib/system_load/severity_test.ts - Memory severity and the three-sample paging rule from the system-load design
app/src/lib/system_load/severity.ts - Calm/busy/hot/paging classification for system-load meters (docs/design/system_load_design.md)
app/src/lib/tabs/tab_items.ts - Tab-bar items derived from the configured repos: standalone tabs and grouped (worktree) tabs
app/src/lib/terminal/terminal_flow.ts - Per-session monotonic output credit acknowledgements: coalesced after xterm parses bytes, or on receipt while the pag...
app/src/lib/terminal/terminal_ipc.ts - Typed Tauri invoke wrappers and the output-channel seam for terminal sessions (mirrors src-tauri terminal/frames.rs)
app/src/lib/terminal/terminal_keys.ts - Windows Terminal key and mouse policy for one xterm: copy/paste chords, Ctrl+C with a selection, right-click, Shift+E...
app/src/lib/terminal/terminal_output_scan.ts - Tells whether a pty output chunk paints anything (text or a line break) once escape sequences are skipped
app/src/lib/terminal/terminal_parking.ts - Off-screen parking host for terminal elements that are alive but not shown; sized so xterm open() and fit() still mea...
app/src/lib/terminal/terminal_registry.ts - Module-level owner of every terminal session grouped per repo: immutable snapshots for React, open/close/restart, ado...
app/src/lib/terminal/terminal_renderer.ts - WebGL renderer policy: attached once per session after its first adopt and kept while parked; DOM renderer on context...
app/src/lib/terminal/terminal_session_io.ts - One pty lifetime for a terminal tab: open handshake, queued and serialised input, output pump with credit acks, debou...
app/src/lib/terminal/terminal_session.ts - One terminal tab living outside React: the xterm instance and wrapper element, renderer adopt/park, fit, pty lifecycl...
app/src/lib/terminal/terminal_theme.ts - Reads deck colour and font tokens into xterm theme and options; an empty token leaves xterm's default in place
app/src/lib/terminal/terminal_types.ts - Frontend terminal session model: tab/group snapshots and the registry API the rail consumes
app/src/lib/theme/accent_utils.ts - Convert hex accent color to CSS variable values for runtime theming
app/src/lib/theme/texture_catalog.ts - Build-time texture catalog for theme substrate/dither selection
app/src/lib/window/effective_ui_mode_policy.ts - Resolves runtime effective UI mode from preferred mode and window state
app/src/lib/window/foreground.ts - Shared predicate for whether this window is truly foreground (visible + focused)
app/src/lib/window/mode_window_bounds.ts - Shared per-mode window bounds defaults, clamping, and resolution helpers
app/src/main.tsx - React entry point - mounts App with ConfigProvider and AgentProvider
app/src/shared/config.ts - Shared config barrel exports
app/src/shared/config/app_config.ts - AppConfig schema, types, and defaults
app/src/shared/config/bundle_presets.ts - Bundle preset schema, type, and defaults
app/src/shared/config/generated_code_globs.ts - Generated default code globs for extension-based classification. Generated by: scripts/classification/generate_code_c...
app/src/shared/config/glob_defaults.ts - Default glob patterns for docs, code, and ignores
app/src/shared/config/persisted_config_code_globs_migration.ts - Default-only additive migration for expanded code globs coverage.
app/src/shared/config/persisted_config_global_excludes_migration.ts - Legacy global excludes normalization and the model-dir pattern migration
app/src/shared/config/persisted_config_migrations.ts - Persisted config migrations and legacy normalization
app/src/shared/config/persisted_config_recommended_extensions_migration_test.ts - Every config is seeded once with the scene extensions; baseline-gated additions still respect trimmed lists
app/src/shared/config/persisted_config_recommended_extensions_migration.ts - Merge newly recommended exclude extensions into configs that still carry the recommended baseline
app/src/shared/config/persisted_config_repo_roots_migration.ts - Repo root migration helpers for persisted config normalization
app/src/shared/config/persisted_config.ts - Persisted config schema, types, and defaults
app/src/shared/config/repo_config.ts - RepoConfig schema and type
app/src/shared/config/repo_root.ts - Repo root authority union schema and path normalization helpers
app/src/shared/config/ui_state_schema.ts - Persisted UI state schema: rail section, left panel mode, window bounds
app/src/shared/config/version.ts - Persisted config schema version
app/src/shared/global_exclude_rules_test.ts - The frontend global-exclude matcher agrees with the Rust scanner on suffixes, names, and path segments
app/src/shared/global_exclude_rules.ts - Normalize global excludes and match names and paths the way the bundle scanner does
app/src/shared/global_excludes.ts - Global bundle exclude schema and UI options
app/src/shared/protocol_bundle_inspection.ts - Selection-aware bundle entry measurements and bounded largest-file results
app/src/shared/protocol_bundles.ts - Bundle-related agent protocol schemas and types
app/src/shared/protocol_events_delta.ts - Zod mirror of the agent fileDelta event: bounded content of one settled file change
app/src/shared/protocol_events.ts - Agent event and file metadata schemas shared by protocol envelope parsing
app/src/shared/protocol_file_meta.ts - Leaf file metadata enums shared by the file change and file delta event schemas
app/src/shared/protocol_import.ts - Drag-and-drop file import command/result schemas shared with the agent
app/src/shared/protocol_launch.ts - Tauri project-launch command response schemas (launch candidates and launch outcome)
app/src/shared/protocol_repo_commands.ts - Core repo watch, refresh, staging, file-read, handshake, and bundle-list command schemas
app/src/shared/protocol_repo_topology.ts - Repo topology and lazy directory listing protocol schemas
app/src/shared/protocol_source_control.ts - Source-control status, diff, and action command/result schemas shared with the agents
app/src/shared/protocol_system_load.ts - `sample_system_load` Tauri command response schema (mirrors crates/im_system_load/src/snapshot.rs)
app/src/shared/protocol_tr_fleet.ts - TR fleet command/response schemas for build-server status and recovery controls
app/src/shared/protocol_worktree.ts - ZIPS-tree worktree action (delete/move/copy/rename) command/result schemas shared with the agent
app/src/shared/protocol.ts - Agent<->UI WebSocket protocol unions and envelopes with Zod validation
app/src/shared/repo_utils.ts - Utility functions for repo ID generation and path handling
app/src/styles/a11y.css - Accessibility utilities - focus rings, disabled states, screen reader helpers
app/src/styles/agent_offline_banner.css - Banner styling for offline WSL agent diagnostics
app/src/styles/auto_files_controls.css - Header controls for the Auto files panel
app/src/styles/auto_files_responsive.css - Responsive rules for the Auto files panel
app/src/styles/auto_files_telemetry.css - Activity and pulse telemetry for Auto files rows
app/src/styles/auto_files.css - Unified Auto files table matching the ranked mockup reference
app/src/styles/badges.css - Bracket-style badge tags for status indicators [A] [M] [D] [U] [T] [STAGED] [LATEST]
app/src/styles/boot.css - Boot phase opacity gate - smooth fade-in when main window becomes ready
app/src/styles/bundle_build_button.css - Bundle build and cancel command button styles
app/src/styles/bundle_column_layout.css - Bundle column layout and preset selector styles
app/src/styles/bundle_column.css - Bundle column style entrypoint
app/src/styles/bundle_entry_hover.css - Readable bundle-size inspection on the shared glass popover surface
app/src/styles/bundle_file_explorer_drop.css - Drag-and-drop import hover states for the ZIPS explorer tree and directory rows
app/src/styles/bundle_file_explorer_selection.css - Row selection, cut-dim, drag ghost, and list focus states for the ZIPS tree
app/src/styles/bundle_file_explorer.css - Lazy bundle file explorer rows and file include glow states
app/src/styles/bundle_list.css - Bundle list rows, ready pulse, and metadata styles
app/src/styles/bundle_selection_panel.css - Bundle selection panel shell and shared file explorer controls
app/src/styles/chrome.css - Unified header chrome styles for tab bar, status bar, and banners
app/src/styles/columns.css - Standard deck grid layout: Auto files, the drag divider, and the rail at its persisted width
app/src/styles/confirm_modal.css - Confirmation dialog overlay with glass panel styling
app/src/styles/context_menu.css - Shared glass popover surface and right-click menu controls
app/src/styles/deck_section_switcher.css - Segmented icon-rocker deck section tablist shared by the handset deck and the right rail
app/src/styles/deck_splitter.css - The drag divider between the left deck panel and the rail: occupies the column gap, lights up on hover and while drag...
app/src/styles/diff_workspace.css - Read-only diff viewer styling mirroring the workspace editor shell
app/src/styles/drag_error_notice.css - Inline glass toast for drag errors
app/src/styles/effects.css - Deck chassis frame, substrate (grid + grain), vignette, and glass utilities
app/src/styles/empty_state_panel.css - Centered glass empty-state surface, subdued typography and repository action context
app/src/styles/global_notes.css - Global notes overlay sheet, corner close, and roomy markdown editor sizing
app/src/styles/handset_chassis.css - Handset v2 chassis frame, glow capsule accents, and section transitions
app/src/styles/handset_deck.css - Handset mode single-panel vertical deck layout
app/src/styles/image_diff_workspace.css - Side-by-side image diff panes, checkerboard slots, and handset stacking
app/src/styles/main_nav.css - Recessed keycap cluster for app-level navigation ahead of the repository tab track
app/src/styles/main.css - Global layout reset and base structure
app/src/styles/motion.css - Motion utilities, transition presets, and reduced-motion support
app/src/styles/options_controls.css - Buttons, text/number inputs, checkbox rows, and path display controls
app/src/styles/options_excludes.css - Collapsible sections, chevron toggle, and advanced grid/groups for excludes
app/src/styles/options_layout.css - Two-column grid layout, sections, rows, footer, and responsive fallback
app/src/styles/options_overlay.css - Overlay backdrop, panel shell, and keyframe animations
app/src/styles/options_theme.css - Theme section styles - color picker, texture picker, rename controls
app/src/styles/overview/overview_detail.css - Stream-family file head, uppercase meta chips, relative clock and the shared diff-row density
app/src/styles/overview/overview_feed.css - Singular chronological feed, repository section rules and Stream-family entry chassis
app/src/styles/overview/overview_git.css - Overview Git column: glass shell, Stream-family repository blocks, state lines and dense read-only rows
app/src/styles/overview/overview_image.css - Width-sized image workspace launcher on the Stream strip's slot and checkerboard
app/src/styles/overview/overview_panel.css - Overview feed glass panel, and the toolbar strip of active-repository filter chips and the monitoring badge
app/src/styles/overview/overview_view.css - Full-pane Overview: toolbar strip over the feed and Git column grid, and the file workspace without remounting
app/src/styles/overview/overview_workspace.css - Full-width Overview file workspace sizing without repository rails
app/src/styles/panels.css - Modular deck panel surfaces with framed edges and etched headers
app/src/styles/repo_rail.css - Right-rail panel with a slim section-switch header above the zips or source body
app/src/styles/scrollbars.css - Thin dark scrollbar styling with accent hints
app/src/styles/source_control_rows.css - Source Control change rows: icon, name over dir, badge, and hover stage/unstage action
app/src/styles/source_control_sections.css - Source Control collapsible section headers, bulk actions, and row containers
app/src/styles/source_control.css - Source Control column: status line, warnings, commit box, and notices
app/src/styles/status_bar.css - Status strip: connection LED, error ticker, instrument-window slot and options button
app/src/styles/stream/stream_card_body.css - Stream card bodies: the diff density override, footers and ghosts, burst and notice rows
app/src/styles/stream/stream_card_image.css - Stream image strips: the growing-then-wrapping tile grid, width-sized checkerboard slots, badges, names, the BEFORE/A...
app/src/styles/stream/stream_card.css - Stream card chassis: spine, head grid, focus and lifecycle states, narrow-chassis rules
app/src/styles/stream/stream_motion.css - Stream arrival choreography: keyframes, pressure bands, the governor carve-out, and reduced motion
app/src/styles/stream/stream_panel.css - Stream scroller, exceptional status, follow control and compact history rows
app/src/styles/system_load_panel.css - System-load detail panel: glass shell, header cue, headline and verdict, trend line, top programs
app/src/styles/system_load.css - Status-strip instrument window: severity-tinted LED meters and narrow-width relief
app/src/styles/tab_bar_dropdown.css - Dropdown-specific styles for tab bar worktree actions
app/src/styles/tab_bar_items.css - Repository navigation actions, group menus and add/remove controls
app/src/styles/tab_bar_launch.css - Project-launch play button, target chevron, target menu and launch error notice
app/src/styles/tab_bar.css - Tab bar navigation with ASCII-instrument bracketed labels
app/src/styles/terminal_column.css - Terminal rail body: tab strip, xterm host and session element, console-prompt notices
app/src/styles/text_workspace_semantics.css - Semantic Markdown rendering layer for workspace text editors
app/src/styles/text_workspace.css - Shared workspace layout and editor/viewer styling for notes, text, and images
app/src/styles/theme_accents.css - Default accent color variables (runtime values applied via inline styles in app.tsx)
app/src/styles/theme_dark.css - Dark glass vintage theme - fills semantic token slots
app/src/styles/theme_light.css - Light theme overrides - warm parchment/linen tones, muted and soft
app/src/styles/theme_warm.css - Warm theme overrides - golden hour amber tones, saturated and warm
app/src/styles/tokens.css - Design system tokens - spacing, radii, blur, shadows, typography, motion
app/src/tabs/repo_tab_file_panel.tsx - Chooses the Stream panel or the Auto files table for the repo tab's file slot
app/src/tabs/repo_tab_rail.tsx - Composes the ZIPS, SOURCE and TERMINAL rail bodies for one repo tab; RepoTab hands them to the rail or the handset deck
app/src/tabs/repo_tab.tsx - Generic repo tab component with Auto files, the right rail (zips | source | terminal), and the workspace
app/src/types/agent_supervisor.ts - Types for Tauri host-agent supervisor responses
app/src/types/app_paths.ts - TypeScript interface matching Rust AppPaths struct
app/src/vite_env.d.ts - Vite client type declarations
crates/im_agent/src/bundles/bundle_builder_blocking.rs - Blocking bundle build steps and filesystem operations
crates/im_agent/src/bundles/bundle_builder_tests.rs - Tests for bundle builder helpers
crates/im_agent/src/bundles/bundle_builder.rs - Bundle build orchestration using the im_bundle library
crates/im_agent/src/bundles/bundle_inspection.rs - Owned blocking inspection worker shared by host and WSL repository routes
crates/im_agent/src/bundles/bundle_lister.rs - Bundle listing and latest selection logic
crates/im_agent/src/bundles/bundle_progress.rs - Bundle progress forwarding from im_bundle to agent events
crates/im_agent/src/bundles/ignore_rules.rs - Centralized ignore patterns for bundle building and scanning
crates/im_agent/src/bundles/mod.rs - Bundle helpers for the agent
crates/im_agent/src/error/agent_error.rs - AgentError type and mapping to protocol error responses
crates/im_agent/src/error/mod.rs - Error module exports for the agent runtime
crates/im_agent/src/error/mutation_effect.rs - Outcome certainty (`details.effect`) carried by every source-control mutation error
crates/im_agent/src/lib.rs - Library root for the Intermediary WSL agent daemon
crates/im_agent/src/logging/json_logger_tests.rs - Unit tests for the JSON logger's stdio mirroring switches
crates/im_agent/src/logging/json_logger.rs - JSONL logger that writes each agent's own size-rotated log file and optionally mirrors to stdout/stderr
crates/im_agent/src/logging/log_file.rs - One agent's JSONL log file, rotated by size into a single previous generation
crates/im_agent/src/logging/mod.rs - Logging exports and helpers for the agent
crates/im_agent/src/main.rs - WSL agent daemon entry point
crates/im_agent/src/protocol/bundle_inspection.rs - Selection-aware bundle entry inspection request and response contracts
crates/im_agent/src/protocol/cancel_bundle_tests.rs - Protocol tests for cancellable bundle build messages
crates/im_agent/src/protocol/commands_import.rs - UI-to-agent command payload for importing external OS files into a repo directory
crates/im_agent/src/protocol/commands_source_control.rs - UI-to-agent source-control command payloads (status, diff, tagged actions)
crates/im_agent/src/protocol/commands_tr_fleet.rs - TR fleet command payloads for host-agent build-server status and recovery controls
crates/im_agent/src/protocol/commands_worktree.rs - UI-to-agent command payload for deleting, moving, copying, and renaming worktree entries
crates/im_agent/src/protocol/commands.rs - UI-to-agent command payloads for the WebSocket protocol
crates/im_agent/src/protocol/envelopes.rs - Protocol envelope types for request/response messaging
crates/im_agent/src/protocol/events_delta.rs - Wire types for the fileDelta event - what changed inside one file
crates/im_agent/src/protocol/events_legacy_wire.rs - Legacy hostPath/windowsPath wire shapes and conversions for staged-info and bundle-built events
crates/im_agent/src/protocol/events_runtime.rs - Runtime status and error event payloads
crates/im_agent/src/protocol/events.rs - Agent event payloads and file entry types
crates/im_agent/src/protocol/mod.rs - WebSocket protocol types for the agent
crates/im_agent/src/protocol/responses_import.rs - Agent-to-UI response payload listing the files one import landed in the worktree
crates/im_agent/src/protocol/responses_legacy_wire.rs - Legacy hostPath/windowsPath wire shapes and conversions for staged and bundle responses
crates/im_agent/src/protocol/responses_repo.rs - Repository topology and directory listing response payloads
crates/im_agent/src/protocol/responses_source_control.rs - Agent-to-UI source-control payloads: working-tree status, per-file diff, action outcome
crates/im_agent/src/protocol/responses_tr_fleet.rs - TR fleet response payload types for host-agent build-server control
crates/im_agent/src/protocol/responses_worktree.rs - Agent-to-UI response payload naming the entries one worktree action produced
crates/im_agent/src/protocol/responses.rs - Agent-to-UI response payloads for the WebSocket protocol
crates/im_agent/src/protocol/tests_bootstrap.rs - Client hello staging and repository root wire contracts
crates/im_agent/src/protocol/tests_bundles.rs - Bundle build and listing paths on command result and event wires
crates/im_agent/src/protocol/tests_delta.rs - Wire-shape tests for the fileDelta event and its payload union
crates/im_agent/src/protocol/tests_envelopes.rs - Protocol request, response and cancellation envelope roundtrips
crates/im_agent/src/protocol/tests_events.rs - Activity event root identity and staged path wire contracts
crates/im_agent/src/protocol/tests_file_access.rs - File staging, reading and directory listing wire contracts
crates/im_agent/src/protocol/tests_runtime_events.rs - Backend status, repository topology and watcher error event wires
crates/im_agent/src/protocol/tests_shutdown.rs - Wire-shape tests for the shutdown command and its result
crates/im_agent/src/protocol/tests_source_control.rs - Wire-shape tests for the source-control command and status payloads
crates/im_agent/src/protocol/tr_fleet_tests.rs - TR fleet protocol command/response serialization tests
crates/im_agent/src/repos/categorizer.rs - File kind classification based on globs and fallback heuristics
crates/im_agent/src/repos/delta/baseline_cache.rs - Byte-bounded LRU of the baseline (text or line fingerprints) last served per path, per repo
crates/im_agent/src/repos/delta/baseline.rs - One retained baseline - full text for ordinary files, 64-bit line fingerprints for large ones
crates/im_agent/src/repos/delta/delta_budget_tests.rs - Burst budget tests - per-emit charge, gone budget, causal refill (a flood never refills mid-run, a hot loop does), wi...
crates/im_agent/src/repos/delta/delta_budget.rs - The delta read budget - burst token bucket, causal refill, gone budget, per-window log gates, and the per-change char...
crates/im_agent/src/repos/delta/delta_build.rs - The blocking half of a text delta - diff settled content against its baseline and derive the baseline retained next
crates/im_agent/src/repos/delta/delta_cache_tests.rs - Baseline cache tests - byte budget, LRU eviction and the rename carry
crates/im_agent/src/repos/delta/delta_index_capture.rs - The index rung of the baseline ladder - stage-0 blob captured before the worktree read, fingerprinted off the runtime...
crates/im_agent/src/repos/delta/delta_limits.rs - Process-wide delta bounds - the one read-permit semaphore every repo's worker shares
crates/im_agent/src/repos/delta/delta_order_tests.rs - Resolver causal-order tests through the ReadSources seam - index blob before worktree, capture reused on re-settle, a...
crates/im_agent/src/repos/delta/delta_patch_tests.rs - Patch grammar, truncation and settled-read tests for the delta pipeline
crates/im_agent/src/repos/delta/delta_queue_tests.rs - Settle queue folding, latency ceiling, cap and op-collapse tests
crates/im_agent/src/repos/delta/delta_read.rs - Blocking stat-read-restat of one settled file for the delta pipeline
crates/im_agent/src/repos/delta/delta_reads.rs - The two reads a text delta depends on - index blob and settled worktree read - behind one seam the resolver is tested...
crates/im_agent/src/repos/delta/delta_resolve_tests.rs - Per-change decision tests - rename baseline move, first-sighting truncate guard, image metadata failures
crates/im_agent/src/repos/delta/delta_resolve_text_tests.rs - Text-arm tests - the zero-length first-sighting rule and the READ_DEADLINE bound on the permit wait
crates/im_agent/src/repos/delta/delta_resolve_text.rs - The text arm of the resolver - deadline-bounded owned read permits, the zero-length first-sighting rule, the baseline...
crates/im_agent/src/repos/delta/delta_resolve.rs - Resolves one settled change into a fileDelta payload - dispatch by kind and op, image metadata, the deletion card
crates/im_agent/src/repos/delta/delta_stamp.rs - RFC 3339 mtime stamps for one delta - now, a SystemTime, or the milliseconds a settled read reported
crates/im_agent/src/repos/delta/delta_test_support.rs - Resolver test seam - a scripted ReadSources recorder, a change fixture, and the harness that runs resolve against them
crates/im_agent/src/repos/delta/delta_worker_counters.rs - The withheld and dropped counters a worker owes the UI, the one sequence they share with fileDelta, and when a standa...
crates/im_agent/src/repos/delta/delta_worker_emit.rs - Builds the fileDelta wire event from one resolved change and the worker's stamp
crates/im_agent/src/repos/delta/delta_worker_evict.rs - Baseline eviction for changes that never reached the resolver - dropped marks (bounded record, overflow clears all) a...
crates/im_agent/src/repos/delta/delta_worker_tests.rs - Worker-level tests - a withheld rename evicts both baselines, counters consume the shared seq, the time-bound standal...
crates/im_agent/src/repos/delta/delta_worker.rs - The delta worker loop - drains settled changes, applies the burst budget, evicts stale baselines, stamps and publishe...
crates/im_agent/src/repos/delta/large_delta_tests.rs - Large-content tests - per-row cap, fingerprint baselines (diff, budget, deletion, rename-over) and the raised read bound
crates/im_agent/src/repos/delta/mod.rs - Bounded delta pipeline for fileDelta - named bounds, DeltaService owner, pure-core exports
crates/im_agent/src/repos/delta/patch_render.rs - Renders diff ops as hunks-only unified patch text - per-row length cap, PATCH_MAX_BYTES budget, removed rows omitted...
crates/im_agent/src/repos/delta/settle_change.rs - One pending change on the settle queue - its op vocabulary, op collapse, and deadline arithmetic
crates/im_agent/src/repos/delta/settle_queue.rs - Pure per-path trailing coalescer for the delta pipeline
crates/im_agent/src/repos/delta/unified_patch.rs - Hunks-only unified patch text and exact stats for one delta, against a text or fingerprint baseline
crates/im_agent/src/repos/file_activity.rs - Activity metadata updates for recent file ranking
crates/im_agent/src/repos/generated_code_extensions.rs - Generated extension list for fallback code classification in the Rust agent. Generated by: scripts/classification/gen...
crates/im_agent/src/repos/git_ignore/git_ignore_tests.rs - Real-repository tests for the Git ignore filter: info/exclude, tracked override, and event reload
crates/im_agent/src/repos/git_ignore/loader.rs - Gathers a repo's Git ignore files (tracked .gitignore, info/exclude, core.excludesFile) off the async runtime
crates/im_agent/src/repos/git_ignore/mod.rs - Watcher-side Git ignore filter: suppresses ignored untracked paths and reloads rules when ignore files change
crates/im_agent/src/repos/git_ignore/rules_tests.rs - Unit tests for Git ignore rule precedence, directory patterns, and negation
crates/im_agent/src/repos/git_ignore/rules.rs - Compiled Git ignore rules with Git's per-directory precedence and excluded-parent semantics
crates/im_agent/src/repos/ignore_matcher.rs - Ignore glob matcher for repo watcher
crates/im_agent/src/repos/image_file_reader_tests.rs - Image preview reader tests - mime gate, root escape, size bounds (process-wide and per-request), one-revision binding
crates/im_agent/src/repos/image_file_reader.rs - Repo-relative image file reader for in-app preview workspaces
crates/im_agent/src/repos/import/copy.rs - The import conflict pre-pass and the policy-specific copy that writes into the worktree
crates/im_agent/src/repos/import/mod.rs - Copying external OS files and folders into one directory of a repo worktree
crates/im_agent/src/repos/import/sources.rs - Source translation, per-source validation, and the bounded walk that plans an import
crates/im_agent/src/repos/import/tests_git_control.rs - Real linked-worktree imports refuse Git pointer files before every write policy
crates/im_agent/src/repos/import/tests_refusals.rs - Import refusal tests: every error the wire contract names, and the proof nothing was written
crates/im_agent/src/repos/import/tests_support.rs - Shared fixtures for the import tests: a worktree, an external source, and one call
crates/im_agent/src/repos/import/tests.rs - Import behaviour tests: what lands in the worktree under each conflict policy
crates/im_agent/src/repos/import/translate.rs - Turning the OS paths a drop delivered into paths this agent's own namespace can reach
crates/im_agent/src/repos/mod.rs - Repository scanning module exports
crates/im_agent/src/repos/mru_index.rs - MRU index for recent file changes
crates/im_agent/src/repos/recent_files_normalizer.rs - Normalize persisted recent-file entries against current filters
crates/im_agent/src/repos/recent_files_store_tests.rs - Recent files persistence migration regression tests
crates/im_agent/src/repos/recent_files_store.rs - Persist recent files with debounced atomic writes
crates/im_agent/src/repos/repo_directory_listing.rs - Lazy repo-relative directory listing for file explorer views
crates/im_agent/src/repos/repo_top_level.rs - Scan top-level entries and bounded nested bundle-selector directory paths
crates/im_agent/src/repos/repo_topology_change.rs - Detect watcher events that invalidate repo top-level metadata
crates/im_agent/src/repos/repo_watcher_delta_marks.rs - How watcher events mark the delta queue - one note per change, one rename per two-path arm
crates/im_agent/src/repos/repo_watcher_events.rs - Event handling for repo watcher changes and rename mapping
crates/im_agent/src/repos/repo_watcher_startup_tests.rs - Unit test that repo watch registration never follows symlinks out of the repo root
crates/im_agent/src/repos/repo_watcher_startup.rs - Repo watcher startup - notify watcher creation, background watch registration, initial tracked-path load
crates/im_agent/src/repos/repo_watcher_tests.rs - Unit tests for the repo watcher's initial-entries ignore filtering and rename delta marks
crates/im_agent/src/repos/repo_watcher.rs - Notify-based repo watcher with MRU, delta pipeline, and event emission
crates/im_agent/src/repos/source_control_watch/coalescer.rs - Rate-limit sourceControlChanged emission with a guaranteed trailing event
crates/im_agent/src/repos/source_control_watch/detector_tests.rs - Unit tests for the source-control change detector (tracked-set override, git metadata allowlist)
crates/im_agent/src/repos/source_control_watch/detector.rs - Decide whether a raw watcher event can move `git status` for a repo
crates/im_agent/src/repos/source_control_watch/git_dirs.rs - Resolve a repo's git dir and common dir so linked worktrees stay watched
crates/im_agent/src/repos/source_control_watch/mod.rs - Watcher-side source control signal: detection, coalescing, git dir resolution, tracked-set reload
crates/im_agent/src/repos/source_control_watch/source_control_watch_tests.rs - SourceControlWatch integration tests - burst coalescing and index-triggered tracked-set reload
crates/im_agent/src/repos/source_control_watch/tracked_set.rs - Tracked-path authority loaded from `git ls-files`, shared between the detector and its reloader
crates/im_agent/src/repos/text_file_reader.rs - Repo-relative UTF-8 text file reader for in-app scratch viewing
crates/im_agent/src/repos/watcher_error.rs - Watcher error classification and event shaping
crates/im_agent/src/repos/worktree/copy_entries.rs - Copying selected worktree entries into one destination folder through the import writer
crates/im_agent/src/repos/worktree/destination.rs - Resolving the destination folder of a worktree write, bounding its replace authorization, and proving the paths it cl...
crates/im_agent/src/repos/worktree/entries.rs - The repo-relative entry path law every worktree action shares, and the refusals it raises
crates/im_agent/src/repos/worktree/mod.rs - The four worktree entry actions (delete, move, copy, rename) behind one caller-locked owner
crates/im_agent/src/repos/worktree/move_entries.rs - Moving selected worktree entries into one destination folder, refused whole before the first rename
crates/im_agent/src/repos/worktree/rename.rs - Renaming one worktree entry in place, never over anything that already exists
crates/im_agent/src/repos/worktree/tests_copy.rs - In-repo copy tests: the import writer's behaviour, reached with repo-relative entries
crates/im_agent/src/repos/worktree/tests_move.rs - Move behaviour tests: what lands, what is refused whole, and what a folder may never do
crates/im_agent/src/repos/worktree/tests_no_replace.rs - Tests for the no-replace write a move performs at every destination the user did not authorize
crates/im_agent/src/repos/worktree/tests_rename.rs - Rename behaviour tests: what commits, which names are refused, and what is never replaced
crates/im_agent/src/repos/worktree/tests_support.rs - Shared fixtures for the worktree action tests: a worktree, its files, and one call
crates/im_agent/src/runtime/config_fingerprint.rs - Compute global and per-repo watcher-relevant config fingerprints
crates/im_agent/src/runtime/config.rs - Minimal app configuration structures for the agent runtime
crates/im_agent/src/runtime/mod.rs - Agent runtime exports
crates/im_agent/src/runtime/state_refresh_tests.rs - Refresh rejects stopped watchers; watch_repo replaces one whose root watch failed
crates/im_agent/src/runtime/state_watchers.rs - Watcher lifecycle helpers for agent runtime state
crates/im_agent/src/runtime/state.rs - Agent runtime state and option handlers
crates/im_agent/src/runtime/watcher_reconciliation.rs - Concurrent repository watcher reconciliation for agent clientHello bootstrap
crates/im_agent/src/server/connection_tests.rs - Request task cancellation tests for agent WebSocket connections
crates/im_agent/src/server/connection.rs - Per-connection WebSocket handling and request routing
crates/im_agent/src/server/connection/dispatch.rs - Command dispatch for WebSocket request handling
crates/im_agent/src/server/connection/repo_commands.rs - Repo file-read and topology command handlers for WebSocket dispatch
crates/im_agent/src/server/connection/request_cancellation.rs - Cooperative cancellation handles for active backend requests
crates/im_agent/src/server/connection/shutdown_command.rs - The `shutdown` command handler for the WSL agent: drain, answer, then exit
crates/im_agent/src/server/connection/source_control_commands.rs - Source-control command handlers for WebSocket dispatch (status, diff, actions)
crates/im_agent/src/server/event_bus_relay.rs - The per-connection relay - a drop-on-full stream lane under slot and byte ceilings, a backpressured control lane, and...
crates/im_agent/src/server/event_bus_tests.rs - Event bus tests - lane tagging at broadcast, a stream flood against a stalled writer, the byte ceiling gate, the drop...
crates/im_agent/src/server/event_bus.rs - Broadcast agent events to connected WebSocket clients - each event serialized once and tagged with its lane at broadc...
crates/im_agent/src/server/handshake_auth.rs - WSL-agent websocket handshake token validation utilities
crates/im_agent/src/server/mod.rs - WebSocket server module exports
crates/im_agent/src/server/runtime_identity.rs - Compute and expose the running agent executable identity during WebSocket handshake
crates/im_agent/src/server/session_log.rs - Connection lifecycle logging that keeps request-less sessions (the supervisor's health probe) at debug
crates/im_agent/src/server/shutdown.rs - The one drain-then-exit owner shared by the shutdown command and the process signals
crates/im_agent/src/server/shutdown/tests.rs - Unit tests for the drain gate: a held mutation keeps the drain waiting, and only idle reports drained
crates/im_agent/src/server/socket_writer_tests.rs - A lagged relay makes the socket writer close the client's connection with 1013
crates/im_agent/src/server/socket_writer.rs - The per-connection socket writer shared by both agents - responses first, then relay events, closing the socket when...
crates/im_agent/src/server/stdin_eof.rs - The supervisor's stdin pipe as a shutdown owner - EOF on fd 0 is a drain request
crates/im_agent/src/server/ws_server_tests.rs - The accept loop frees its port the moment shutdown starts, before any drain
crates/im_agent/src/server/ws_server.rs - WebSocket accept loop and connection dispatch
crates/im_agent/src/source_control/actions/mod.rs - Dispatches one source-control mutation and reads the status that follows it
crates/im_agent/src/source_control/actions/remote.rs - Push and pull for one repo root, including upstream selection
crates/im_agent/src/source_control/actions/stage.rs - Stage and unstage one section or an explicit path list, never a pathspec wildcard
crates/im_agent/src/source_control/actions/tests.rs - Real-git tempdir tests for stage, unstage, discard, push, and pull actions
crates/im_agent/src/source_control/commit/finalize.rs - Post-commit comparison of the landed tree against the reviewed one, split into hook-changed and hook-added paths
crates/im_agent/src/source_control/commit/mod.rs - Commit under the reviewed-snapshot precondition, with timeout recovery and hook reporting
crates/im_agent/src/source_control/commit/tests_hooks.rs - Real-git tests for what a commit hook did to a landed commit: reviewed rewrites and unreviewed additions are both rep...
crates/im_agent/src/source_control/commit/tests_preconditions.rs - Real-git tests binding a commit to the reviewed snapshot identity, and row/section ownership
crates/im_agent/src/source_control/commit/tests.rs - Real-git tests for the commit oracle, its snapshot precondition, and the landed-but-unread error
crates/im_agent/src/source_control/diff/image_sides.rs - Reads one image-diff side from a Git blob or from the working tree, bounded and base64-encoded
crates/im_agent/src/source_control/diff/image.rs - Chooses the before/after Git snapshots of one changed image and assembles both sides
crates/im_agent/src/source_control/diff/mod.rs - Bounded per-file unified diff capture for one repo root (index, worktree, or untracked)
crates/im_agent/src/source_control/diff/tests_image.rs - Real-git tempdir tests for before/after image-diff side selection
crates/im_agent/src/source_control/diff/tests.rs - Real-git tempdir tests for bounded per-file diff capture
crates/im_agent/src/source_control/discard/claim.rs - Per-target quarantine claim, verification, release, and rollback for discard
crates/im_agent/src/source_control/discard/entries.rs - Removing chosen worktree entries by claiming each into this repository's discard quarantine
crates/im_agent/src/source_control/discard/mod.rs - Discard exactly the confirmed targets, one at a time, under an operation-owned quarantine
crates/im_agent/src/source_control/discard/quarantine.rs - Quarantine directory naming and phase files for a discard operation, and the bounded startup sweep
crates/im_agent/src/source_control/discard/target.rs - Executes one discard target: claim, classify, mutate, and release/rollback the claim
crates/im_agent/src/source_control/discard/tests_entries.rs - Delete tests: what the quarantine holds afterwards, what is refused, and what a half-applied delete reports
crates/im_agent/src/source_control/discard/tests_quarantine.rs - Real-git tests for the discard quarantine's phase files, its per-target directories, and retention
crates/im_agent/src/source_control/discard/tests_stamps.rs - Real-git tests binding a discard to the exact file state the user reviewed (stamp, absence, order)
crates/im_agent/src/source_control/discard/tests_sweep.rs - Tests for the once-per-process discard quarantine sweep: what it finishes, what it spares, and what it survives
crates/im_agent/src/source_control/index_blob.rs - Reads one path's stage-0 index blob (`git show :0:./<rel>`) as bounded UTF-8 text for the delta baseline
crates/im_agent/src/source_control/locks/mod.rs - Mutation serialization keyed by the physical Git directory, plus the drain gate
crates/im_agent/src/source_control/locks/tests.rs - Real-git tests for mutation serialization by physical git dir, drain, and mutationInProgress
crates/im_agent/src/source_control/mod.rs - Git working-tree status, per-file diff, and index/commit/remote actions for one repo root
crates/im_agent/src/source_control/paths.rs - UI path validation and normalization, NUL-joined pathspec input, and the in-root containment guard
crates/im_agent/src/source_control/runner/failure.rs - Maps a Git command failure onto an AgentError and, for mutations, its proven effect
crates/im_agent/src/source_control/runner/git_version.rs - Once-per-process Git version probe guarding --pathspec-from-file support (Git 2.25+)
crates/im_agent/src/source_control/runner/mod.rs - spawn_blocking bridge and Git failure to AgentError mapping for source control
crates/im_agent/src/source_control/status/index_tree.rs - Read-only identity of the whole-repository index (`git write-tree` without writing)
crates/im_agent/src/source_control/status/mod.rs - Capture `git status --porcelain=v2` for one repo root and project it onto the wire shape
crates/im_agent/src/source_control/status/project.rs - Projects parsed porcelain-v2 status onto the SourceControlStatus wire shape for one root
crates/im_agent/src/source_control/status/snapshot.rs - One reviewed-snapshot identity over branch, HEAD, index tree, and in-progress merge state
crates/im_agent/src/source_control/status/stamp.rs - Size/mtime/presence reads for worktree and conflict entries, and the shared stamp reader
crates/im_agent/src/source_control/status/tests_projection.rs - Real-git tempdir tests for source-control status projection, the commit oracle, and error mapping
crates/im_agent/src/source_control/tests_support.rs - Real-git tempdir fixtures shared by the source-control tests
crates/im_agent/src/staging/layout_unc.rs - Translation of Windows WSL UNC paths into this distro's own POSIX paths
crates/im_agent/src/staging/layout.rs - Central staging layout derivation for file and bundle outputs
crates/im_agent/src/staging/mod.rs - Staging module exports
crates/im_agent/src/staging/stager.rs - Atomic staging of files into the host-accessible directory
crates/im_bundle/src/bin/im_bundle_cli.rs - CLI entry point for im_bundle - scans and writes bundle zip
crates/im_bundle/src/cancel.rs - Cooperative cancellation token for bundle scan and zip operations
crates/im_bundle/src/compression_policy.rs - Compression policy for bundle entries based on extension and size
crates/im_bundle/src/error.rs - Error types for bundle scanning and zip writing
crates/im_bundle/src/fs_atomic.rs - Atomic no-replace rename for Windows, Linux and macOS
crates/im_bundle/src/git_capture/command_child.rs - Stream worker threads, bounded pipe readers, and exit-status helpers for the Git runner
crates/im_bundle/src/git_capture/command_drain.rs - Bounded pipe drain for the Git runner: grace after exit, then termination of the whole process tree
crates/im_bundle/src/git_capture/command_failure.rs - Why a Git command produced no usable output, and the bounded streams it failed with
crates/im_bundle/src/git_capture/command_stop.rs - Forced stop of a running Git child: ask the process tree to end, then kill it and reap the child
crates/im_bundle/src/git_capture/command_tests.rs - Forced-stop tests for the bounded Git runner: process-group kill and detached stream readers
crates/im_bundle/src/git_capture/command_tree_owner.rs - Getting a process-tree owner for one Git command, and how being refused one is reported
crates/im_bundle/src/git_capture/command_tree.rs - The process tree one Git child owns (unix process group, Windows job object) and the live-tree registry
crates/im_bundle/src/git_capture/command.rs - Bounded, cancellable Git subprocess execution shared by bundle evidence and source control
crates/im_bundle/src/git_capture/diff_issue.rs - Artifact-specific issue classification for selected Git diff capture
crates/im_bundle/src/git_capture/diff.rs - Bounded selected-path Git diff, stat, and name-status capture
crates/im_bundle/src/git_capture/discovery.rs - Git discovery failure classification and raw prefix normalization
crates/im_bundle/src/git_capture/fake_git.rs - Test-only fake Git scripts handed to a test only once the kernel will exec them
crates/im_bundle/src/git_capture/finalize.rs - Git artifact finalization and working-tree coherence verdicts
crates/im_bundle/src/git_capture/ignored.rs - Reconcile selected archived files that Git status hides behind ignore rules
crates/im_bundle/src/git_capture/index_tree.rs - Read-only Git tree SHA of an index listing, matching `git write-tree`
crates/im_bundle/src/git_capture/index.rs - Bounded capture of the candidate index tree identity
crates/im_bundle/src/git_capture/initial_state.rs - Initial selected-delta, index-tree, and file fingerprint capture for a Git session
crates/im_bundle/src/git_capture/mod.rs - Versioned selection-bounded Git evidence capture for bundle archives
crates/im_bundle/src/git_capture/path.rs - Lossless Git path transport and model-readable quoting helpers
crates/im_bundle/src/git_capture/pathspec_batches.rs - Host-safe Git pathspec argument batching with atomic rename pairs
crates/im_bundle/src/git_capture/porcelain.rs - Strict parser for NUL-delimited Git porcelain-v2 records
crates/im_bundle/src/git_capture/prefix.rs - Shared bounded capture of the Git repository prefix and absolute git dir for a configured root
crates/im_bundle/src/git_capture/render_omitted.rs - Model-readable listing of changed paths the bundle selection omitted
crates/im_bundle/src/git_capture/render.rs - Selection-safe human-readable Git status and bundle handoff artifacts
crates/im_bundle/src/git_capture/session.rs - Git capture discovery, initial status, and safety-bound setup
crates/im_bundle/src/git_capture/status.rs - Raw porcelain-v2 Git status parsing and selection-safe projection
crates/im_bundle/src/git_capture/tests.rs - Failure, timeout, and drift tests for bounded Git capture
crates/im_bundle/src/git_capture/verification.rs - Streaming selected-file coherence verification for Git bundle capture
crates/im_bundle/src/git.rs - Public Git primitives shared by bundle evidence capture and agent source control
crates/im_bundle/src/global_excludes_summary.rs - Manifest-facing normalized summary for bundle global excludes
crates/im_bundle/src/global_excludes.rs - Normalize and apply user-configurable global excludes for bundle scanning
crates/im_bundle/src/inspection_tests.rs - Inspection selection parity, size ranking, bounded scans and containment regressions
crates/im_bundle/src/inspection.rs - Bounded metadata inspection using the archive's canonical selection predicate
crates/im_bundle/src/lib.rs - Library root for bundle scanning and zip creation
crates/im_bundle/src/macos_process_session.rs - macOS process-session ownership for native terminals and supervised agents
crates/im_bundle/src/manifest.rs - Bundle manifest structure and serialization
crates/im_bundle/src/omission.rs - Why a changed repository path fell outside the bundle selection
crates/im_bundle/src/plan.rs - Bundle plan schema and loader for im_bundle_cli
crates/im_bundle/src/process_job_termination.rs - Bounded forced termination and observation for a Windows Job Object
crates/im_bundle/src/process_job.rs - Windows Job Object ownership of a spawned process tree, shared by the Git runner and the app's agent supervisor
crates/im_bundle/src/progress_sink.rs - Progress sink interfaces for bundle build reporting
crates/im_bundle/src/progress.rs - Throttled NDJSON progress emitter for bundle scanning and zipping
crates/im_bundle/src/scanner.rs - Bundle scanning logic with ignore rules and exclusions
crates/im_bundle/src/selection_tests.rs - Canonical selection predicate regression coverage for current and missing paths
crates/im_bundle/src/selection.rs - Canonical bundle-selection predicate shared by scanning and Git capture
crates/im_bundle/src/writer_tests.rs - Tests for bundle writer behavior and progress ordering
crates/im_bundle/src/writer.rs - Bundle zip writer with scanning, manifest, and progress
crates/im_bundle/src/zip_entry.rs - Single file entry writer for bundle zip archives
crates/im_bundle/tests/descendant_selection_test.rs - Archive and Git evidence stay bounded to explicit descendants of excluded folders
crates/im_bundle/tests/git_evidence_test.rs - End-to-end witness tests for selection-bounded bundle Git evidence
crates/im_bundle/tests/git_large_selection_test.rs - Windows-scale witness for host-safe selected Git diff path batching
crates/im_bundle/tests/scanner_test.rs - Integration tests for bundle scanner behavior
crates/im_bundle/tests/size_capped_reads_test.rs - Ensures bundle writes only the bytes present at file-open time even if file grows
crates/im_host_agent/src/config.rs - Host agent environment configuration parsing
crates/im_host_agent/src/error_codes.rs - Shared host-agent error code constants for routing and WSL backend failures
crates/im_host_agent/src/lib.rs - Library root for the Intermediary host agent daemon
crates/im_host_agent/src/main.rs - Host agent daemon entry point
crates/im_host_agent/src/runtime/host_runtime_helpers.rs - Host-runtime helper functions for config parsing and the WSL hello payload
crates/im_host_agent/src/runtime/host_runtime/bundle_forwarding.rs - Host build-bundle preparation and cancellation helpers for HostRuntime
crates/im_host_agent/src/runtime/host_runtime/host_dispatch.rs - Host-rooted repo command dispatch onto the local backend for HostRuntime
crates/im_host_agent/src/runtime/host_runtime/mod.rs - Host runtime command routing and clientHello orchestration for host and WSL backends
crates/im_host_agent/src/runtime/host_runtime/shutdown_targets.rs - The two things a host-agent shutdown must reach: the WSL backend client and the host locks
crates/im_host_agent/src/runtime/host_runtime/wsl_routing.rs - WSL client ownership and lock-free WSL route resolution for HostRuntime
crates/im_host_agent/src/runtime/local_host_backend.rs - Host-native local backend for repo watch, staging, and bundle operations
crates/im_host_agent/src/runtime/local_host_import_backend.rs - Host-native file import execution that never holds the runtime lock across the copy
crates/im_host_agent/src/runtime/local_host_repo_backend.rs - Host-native repo read and topology operations
crates/im_host_agent/src/runtime/local_host_source_control_backend.rs - Host-native source-control execution that never holds the runtime lock across Git
crates/im_host_agent/src/runtime/local_host_worktree_backend.rs - Host-native worktree entry actions that never hold the runtime lock across the write
crates/im_host_agent/src/runtime/mod.rs - Host runtime exports for backend routing and local host handling
crates/im_host_agent/src/runtime/repo_backend.rs - Repo backend kind mapping for host-agent routing
crates/im_host_agent/src/runtime/router.rs - Repo-id command routing for host-agent backend selection
crates/im_host_agent/src/runtime/tr_fleet_service.rs - Host-agent TR fleet status polling and recovery action execution
crates/im_host_agent/src/server/connection.rs - Host-agent per-connection WebSocket handling and response serialization
crates/im_host_agent/src/server/dispatch_tests.rs - Dispatch tests with a scripted WSL backend: hello never waits on WSL, commands share one hello
crates/im_host_agent/src/server/dispatch.rs - Host-agent command dispatch: WSL forwards and long host work run after the runtime lock is released
crates/im_host_agent/src/server/handshake_auth.rs - Host-agent websocket handshake token and origin validation utilities
crates/im_host_agent/src/server/mod.rs - Host-agent WebSocket server module exports
crates/im_host_agent/src/server/request_dispatch.rs - Connection-scoped request dispatch and cancellation of the current hover inspection
crates/im_host_agent/src/server/shutdown_dispatch.rs - Host-agent shutdown: drain the WSL backend first, then this process, then exit
crates/im_host_agent/src/server/shutdown_dispatch/tests.rs - Unit tests for the WSL-unavailable/outstanding-mutation shutdown decision
crates/im_host_agent/src/server/ws_server.rs - Host-agent WebSocket accept loop and connection dispatch
crates/im_host_agent/src/wsl/mod.rs - WSL backend client module exports
crates/im_host_agent/src/wsl/wsl_backend_client.rs - Persistent WebSocket client for forwarding commands/events to the WSL backend agent
crates/im_host_agent/src/wsl/wsl_backend_client/client_loop.rs - The WSL backend connect/reconnect loop and the answers it gives while the backend is unreachable
crates/im_host_agent/src/wsl/wsl_backend_client/hello_reconciler.rs - The single in-flight WSL clientHello reconciliation per connection generation, owned by the WSL client
crates/im_host_agent/src/wsl/wsl_backend_client/hello_state_tests.rs - Unit tests for WSL hello desired/applied/failed state and waiter decisions
crates/im_host_agent/src/wsl/wsl_backend_client/hello_state.rs - Desired, applied and failed WSL clientHello state for the per-generation hello reconciler
crates/im_host_agent/src/wsl/wsl_backend_client/routed_forward.rs - WSL-routed command forwarding that awaits the shared generation hello, plus the hello's public surface
crates/im_host_agent/src/wsl/wsl_backend_client/tests_hello.rs - Hello reconciler tests: online once per generation, failure hand-off, root identity version gate
crates/im_host_agent/src/wsl/wsl_backend_client/tests_outstanding.rs - Unit tests for the outstanding-mutation ledger: decode-site clearing, timeouts, and offline answers
crates/im_host_agent/src/wsl/wsl_backend_client/tests_timeouts.rs - Unit tests for the per-command forward timeout ladder
crates/im_host_agent/src/wsl/wsl_backend_client/tests.rs - Unit tests for WSL backend forwarding, cancellation, and outstanding-mutation tracking
crates/im_host_agent/src/wsl/wsl_backend_client/timeouts.rs - The host->WSL request-timeout ladder and the agent-side worst case each tier covers
crates/im_host_agent/src/wsl/wsl_backend_connection.rs - Connected WSL backend request loop, ping liveness, and pending response handling
crates/im_host_agent/src/wsl/wsl_backend_link_tests.rs - Host->WSL link bounds - frame size, connect deadline, ping liveness against silent and live peers
crates/im_host_agent/src/wsl/wsl_backend_link.rs - Bounds on the host->WSL socket - connect+handshake deadline, ping cadence, silence and send limits
crates/im_host_agent/src/wsl/wsl_backend_messages.rs - WSL-backend message parsing and pending-response helpers
crates/im_host_agent/src/wsl/wsl_transport_status_tests.rs - WSL transport status tests: offline error de-noising and online recovery per generation
crates/im_host_agent/src/wsl/wsl_transport_status.rs - Generation-scoped wslBackendStatus online announcements and de-noised WSL transport error events
crates/im_system_load/src/cpu_identity.rs - Processor brand string from the registry and the logical processor count
crates/im_system_load/src/error.rs - Sampler error carrying the failed operation and its OS status
crates/im_system_load/src/file_description.rs - Friendly process labels from the executable's FileDescription, cached per image name
crates/im_system_load/src/gpu_adapters.rs - One-time DXGI adapter enumeration for GPU names, memory totals and LUIDs
crates/im_system_load/src/gpu_counters.rs - PDH GPU engine and memory counters joined to DXGI adapters and process ids
crates/im_system_load/src/gpu_instance_tests.rs - Tests for PDH GPU instance-name parsing
crates/im_system_load/src/gpu_instance.rs - Parses PDH GPU counter instance names into pid, adapter LUID, node and engine
crates/im_system_load/src/gpu_usage_tests.rs - Tests for the Task Manager GPU aggregation rule
crates/im_system_load/src/gpu_usage.rs - Task Manager GPU aggregation - per-engine sums, adapter busiest engine, process busiest engine
crates/im_system_load/src/lib.rs - Crate root - Windows system-load sampler producing SystemLoadSnapshot
crates/im_system_load/src/memory_status.rs - Physical memory and commit charge from GlobalMemoryStatusEx
crates/im_system_load/src/pdh_query.rs - One PDH query with English counter paths, formatted scalar reads and reused wildcard arrays
crates/im_system_load/src/process_groups.rs - Groups processes by image name and selects the union of the top groups per resource
crates/im_system_load/src/process_rates.rs - Per-process CPU share and I/O rate from deltas keyed by (pid, create time)
crates/im_system_load/src/process_table.rs - One NtQuerySystemInformation(SystemProcessInformation) walk per sample into reused records
crates/im_system_load/src/sampler.rs - Windows Sampler - owns the PDH query, adapter list and process state; assembles snapshots
crates/im_system_load/src/snapshot.rs - System-load wire snapshot serialized to the frontend (mirrors app/src/shared/protocol_system_load.ts)
crates/im_system_load/src/system_counters.rs - PDH counters for CPU utility and speed, paging, physical disks and network interfaces
crates/im_system_load/src/unsupported.rs - Non-Windows Sampler whose construction reports that sampling is unavailable
crates/im_system_load/src/wide.rs - UTF-16 conversions shared by the Windows FFI modules
scripts/classification/code_extensions_source.mjs - Pinned baseline + local overrides for code-classification file extensions.
scripts/classification/generate_code_classification_artifacts.mjs - Generate TS/Rust code-classification extension artifacts from a pinned source list.
scripts/fileledger/add_file_headers.mjs - Adds missing header comments (path + description) to source files using the ledger output.
scripts/fileledger/gen_file_ledger.mjs - Generates human+machine file ledgers for Intermediary code sources.
scripts/icons/generate_icons.mjs - Generate all icon sizes from a source PNG. Usage: node scripts/generate_icons.mjs [source.png] Default source: app/as...
scripts/icons/resize_preview_icons.mjs - Resize preview geometry icons from raw assets to display sizes. Outputs 40px (1x) and 80px (2x retina) versions.
scripts/release/bump_version.mjs - Update all release-facing version files to a single Intermediary version.
scripts/release/check_versions.mjs - Validate that all public Intermediary version files stay in lockstep.
scripts/release/stage_windows_release_assets.mjs - Collect Windows bundle outputs into a release artifact directory with sha256 files.
scripts/release/version_contract.mjs - Shared version-contract helpers for Intermediary release automation.
scripts/test/run_ts_tests.mjs - Bundles every app/src/**/*_test.ts(x) with esbuild into a temp dir and runs node --test over it.
scripts/zip/zip_bundles.mjs - Builds timestamped Intermediary zip bundles for ChatGPT context.
src-tauri/build.rs - Tauri build script
src-tauri/src/bin/intermediary.rs - Binary entry point for Tauri app
src-tauri/src/lib/agent/bundle_resources.rs - Resolves packaged agent resources and their version metadata
src-tauri/src/lib/agent/host_process_control.rs - Windows host-agent process detection and stale-port termination helpers
src-tauri/src/lib/agent/install_host_binary.rs - Resolve and copy the correct host-agent binary into an install bundle staging directory
src-tauri/src/lib/agent/install_runtime.rs - Agent bundle install/runtime helpers for version checks, file copying, and stale-host cleanup
src-tauri/src/lib/agent/install_tests.rs - Agent bundle installation and packaged-runtime identity regression tests
src-tauri/src/lib/agent/install.rs - Install bundled agent runtimes into app local data with platform-specific requirements
src-tauri/src/lib/agent/mod.rs - Host-agent supervisor module exports (with optional Windows WSL backend)
src-tauri/src/lib/agent/process_control.rs - Spawn helpers for host/WSL agents and readiness probing
src-tauri/src/lib/agent/process_control/log_tail.rs - Reads the agent log written since a spawn cursor, bounded by bytes and lines, for early-exit reporting
src-tauri/src/lib/agent/runtime_identity.rs - Bounded SHA-256 identity for packaged and installed agent executables
src-tauri/src/lib/agent/supervisor.rs - Public host-agent supervisor types and wiring
src-tauri/src/lib/agent/supervisor/graceful_stop.rs - Ask the managed host agent to drain and exit before any kill path runs
src-tauri/src/lib/agent/supervisor/graceful_stop/tests.rs - Ack-parsing/route tests for the graceful host stop against a fake agent socket
src-tauri/src/lib/agent/supervisor/host.rs - Host-agent startup and stale-port remediation for the supervisor
src-tauri/src/lib/agent/supervisor/lifecycle.rs - Host-agent-first supervisor lifecycle implementation with optional Windows WSL backend
src-tauri/src/lib/agent/supervisor/lifecycle/tests.rs - The lifecycle lock makes ensure/restart/stop/exit single-flight
src-tauri/src/lib/agent/supervisor/managed_processes.rs - Supervisor-owned child-process bookkeeping, stop, and reconciliation helpers
src-tauri/src/lib/agent/supervisor/managed_processes/tests.rs - State transitions for the supervisor's recorded process and the tree owner it carries
src-tauri/src/lib/agent/supervisor/probes.rs - Async supervisor probe helpers for port, websocket auth, and origin compatibility
src-tauri/src/lib/agent/supervisor/process_kill.rs - Blocking termination of a supervisor-owned process and the tree it started
src-tauri/src/lib/agent/supervisor/runtime.rs - Supervisor runtime path, port, and installed-bundle preference helpers
src-tauri/src/lib/agent/supervisor/shutdown_ws_client.rs - One authenticated shutdown request/response exchange with a managed agent
src-tauri/src/lib/agent/supervisor/shutdown.rs - App-exit teardown: stop agents, then free WSL VM RAM when the distro is idle
src-tauri/src/lib/agent/supervisor/state.rs - Shared supervisor process state and process-kind labels
src-tauri/src/lib/agent/supervisor/websocket_frame.rs - Minimal RFC 6455 client framing used by the supervisor's graceful-shutdown request
src-tauri/src/lib/agent/supervisor/websocket_probe.rs - Blocking websocket auth and origin probes used by the supervisor
src-tauri/src/lib/agent/supervisor/wsl_backend_record.rs - What the supervisor records about the WSL backend it owns this session
src-tauri/src/lib/agent/supervisor/wsl_control.rs - WSL backend termination and stale-port remediation for the supervisor
src-tauri/src/lib/agent/supervisor/wsl_logging.rs - Structured WSL backend ownership and authentication lifecycle logging
src-tauri/src/lib/agent/supervisor/wsl_mode.rs - WSL backend mode parsing and ownership-policy helpers for the supervisor
src-tauri/src/lib/agent/supervisor/wsl_runtime.rs - Shared WSL supervisor timing constants
src-tauri/src/lib/agent/supervisor/wsl_same_port_termination.rs - Same-port Intermediary WSL agent termination for supervisor remediation
src-tauri/src/lib/agent/supervisor/wsl_spawn.rs - Starting the WSL backend and recording it, with the stdin pipe that outlives the launch
src-tauri/src/lib/agent/supervisor/wsl_terminate_logging.rs - How a WSL emergency-stop outcome is named in the supervisor log
src-tauri/src/lib/agent/supervisor/wsl.rs - The ensure-running decision for the WSL backend: ownership detection, adoption, remediation
src-tauri/src/lib/agent/types.rs - Types for supervising host agent lifecycle with optional Windows WSL backend
src-tauri/src/lib/agent/websocket_auth_tests.rs - Durable websocket authentication token persistence tests
src-tauri/src/lib/agent/websocket_auth.rs - Pre-WebView websocket authentication state and durable token persistence
src-tauri/src/lib/agent/wsl_agent_discovery_tests.rs - Tests for WSL agent pid discovery parsing
src-tauri/src/lib/agent/wsl_agent_discovery.rs - In-distro discovery of the Intermediary WSL agent pids a stop is responsible for
src-tauri/src/lib/agent/wsl_agent_termination_channel.rs - The live in-distro channel an emergency WSL stop signals through
src-tauri/src/lib/agent/wsl_agent_termination_tests.rs - Tests for the WSL emergency stop's drain envelope and process-tree escalation
src-tauri/src/lib/agent/wsl_agent_termination.rs - The supervisor's WSL emergency stop - TERM, the agent's own drain, then its process trees
src-tauri/src/lib/agent/wsl_process_control_commands.rs - Login-shell command-line builders and quoting for launching and signalling the WSL agent
src-tauri/src/lib/agent/wsl_process_control.rs - WSL agent launch target resolution and spawning
src-tauri/src/lib/agent/wsl_process_probe_commands.rs - In-distro probe scripts that report Intermediary agent pids and distro idleness
src-tauri/src/lib/agent/wsl_process_tree_commands.rs - The in-distro script that kills a WSL agent's descendant process groups, then the agent
src-tauri/src/lib/agent/wsl_shutdown.rs - Conditional WSL distro teardown to free VM RAM when no interactive session remains
src-tauri/src/lib/commands/agent_control.rs - Tauri commands to manage host + optional WSL agent supervision
src-tauri/src/lib/commands/agent_probe.rs - Probe local host-agent port availability for diagnostics
src-tauri/src/lib/commands/config.rs - Tauri commands for config persistence
src-tauri/src/lib/commands/file_manager.rs - Open folders in the host OS file manager
src-tauri/src/lib/commands/file_open_policy.rs - Host launcher policy for opening text and non-text files
src-tauri/src/lib/commands/file_opener_paths.rs - Resolve repo-relative file paths to host-visible paths
src-tauri/src/lib/commands/file_opener.rs - Reveal files in file manager or open with default application
src-tauri/src/lib/commands/launch.rs - Tauri commands for project launch candidates and one-click launch-or-focus
src-tauri/src/lib/commands/mod.rs - Tauri command exports
src-tauri/src/lib/commands/notes.rs - Tauri commands for per-repo and global plain-text notes persistence
src-tauri/src/lib/commands/paths.rs - get_app_paths command implementation and path conversion utilities
src-tauri/src/lib/commands/reset.rs - Tauri command to clear staging artifacts and caches
src-tauri/src/lib/commands/startup_window_bounds.rs - Resolve and apply persisted launch bounds for startup windows
src-tauri/src/lib/commands/startup.rs - Startup readiness command for splashscreen -> main transition
src-tauri/src/lib/commands/terminal.rs - Tauri commands of the integrated terminal: open, raw-body write, resize, ack, close and the Rust-side clipboard read
src-tauri/src/lib/commands/wsl_distro.rs - Resolve WSL distro override from persisted app config for command-time path conversion
src-tauri/src/lib/config/generated_code_globs.rs - Generated default code globs for Rust-side persisted config migration. Generated by: scripts/classification/generate_...
src-tauri/src/lib/config/io.rs - Config file I/O with atomic writes and error handling
src-tauri/src/lib/config/io/global_excludes_migrations.rs - Versioned migrations of the recommended global-exclude baseline
src-tauri/src/lib/config/io/migration_tests.rs - Focused config migration regression tests
src-tauri/src/lib/config/io/repo_root_migration.rs - Legacy repository root migration helpers for config loading
src-tauri/src/lib/config/io/schema_migrations.rs - Versioned persisted-config schema migrations
src-tauri/src/lib/config/io/tests.rs - Unit tests for config I/O and migration behavior
src-tauri/src/lib/config/mod.rs - Configuration persistence module
src-tauri/src/lib/config/path.rs - Resolve persisted config file location for app commands and setup
src-tauri/src/lib/config/types.rs - Persisted configuration types for Intermediary
src-tauri/src/lib/config/types/model.rs - Supporting persisted configuration model types
src-tauri/src/lib/config/types/tests.rs - Tests for persisted configuration types
src-tauri/src/lib/config/types/ui_state.rs - Remembered UI state: rail section, left files mode, rail width, and window bounds
src-tauri/src/lib/config/types/validation.rs - Persisted configuration validation rules and invariants
src-tauri/src/lib/crosstalk/feed.rs - Bounded incremental reader of complete Crosstalk JSONL delivery records
src-tauri/src/lib/crosstalk/mod.rs - Overview visibility and lifecycle of the read-only Crosstalk companion
src-tauri/src/lib/crosstalk/observer.rs - One filesystem-notified feed worker tied to the native app lifetime
src-tauri/src/lib/crosstalk/placement.rs - Monitor-aware physical placement of the owned speech-bubble window
src-tauri/src/lib/crosstalk/pointer.rs - Visible-companion mouse hook yields only to a clicked overlapping window
src-tauri/src/lib/launch/candidates.rs - Top-level launch candidate discovery for repository roots, ranked by observed usage
src-tauri/src/lib/launch/launch_action.rs - Launches one top-level target: focus-or-start for Unreal projects, default handler for HTML
src-tauri/src/lib/launch/mod.rs - Project launch targets: top-level candidate discovery, usage ranking and launch-or-focus
src-tauri/src/lib/launch/recent_items.rs - Windows Recent-items evidence: when each candidate file was last opened through the shell
src-tauri/src/lib/launch/shell_link.rs - Minimal MS-SHLLINK reader that recovers a shortcut's target path from its LinkInfo
src-tauri/src/lib/launch/unreal_engine.rs - Resolves a .uproject's EngineAssociation to the UnrealEditor.exe that should open it
src-tauri/src/lib/launch/unreal_focus.rs - Finds an UnrealEditor.exe whose command line names a project and brings its window forward
src-tauri/src/lib/launch/unreal_recent.rs - Unreal Editor's own recent-projects evidence from every engine version's EditorSettings.ini
src-tauri/src/lib/macos_instance.rs - Mac application exclusivity before mutable startup and native activation
src-tauri/src/lib/mod.rs - Library root - Tauri setup and plugin registration
src-tauri/src/lib/obs/logging.rs - File-based logger writing to run_latest.txt
src-tauri/src/lib/obs/mod.rs - Observability module exports
src-tauri/src/lib/paths/app_paths.rs - Application path resolution logic
src-tauri/src/lib/paths/mod.rs - Path resolution module exports
src-tauri/src/lib/paths/repo_root_resolver.rs - Path-native repo root resolver for user-selected repo paths
src-tauri/src/lib/paths/wsl_convert.rs - Windows <-> WSL path conversion utilities
src-tauri/src/lib/system_load.rs - Hosts the lazily created system-load Sampler and the sample_system_load command
src-tauri/src/lib/terminal/clipboard.rs - Reads CF_UNICODETEXT from the Windows clipboard for terminal paste, because WebView2 cannot read the clipboard withou...
src-tauri/src/lib/terminal/exit_cell.rs - Set-once exit record of a session's child, with bounded waits for the threads that need it
src-tauri/src/lib/terminal/flow_gate.rs - Cumulative sent/consumed flow watermarks bounding terminal output publication
src-tauri/src/lib/terminal/frames.rs - Wire shapes shared with the frontend terminal client: open request/result, exit frame, close reasons and outcomes
src-tauri/src/lib/terminal/mod.rs - Integrated terminal backend: ConPTY-backed pwsh sessions owned by the Tauri process (module tree)
src-tauri/src/lib/terminal/output_sink.rs - Non-blocking detachable owner of a terminal session's bounded webview output channel
src-tauri/src/lib/terminal/reader_thread.rs - Retained terminal reader that drains to EOF and reports its final bounded-output result
src-tauri/src/lib/terminal/reaper.rs - Short external terminal reaper joining process, PTY-close, reader, and waiter ownership
src-tauri/src/lib/terminal/registry_shutdown.rs - Navigation and app-exit drains over atomically captured terminal transactions
src-tauri/src/lib/terminal/registry_tests.rs - Atomic admission and Opening-transaction regression tests for TerminalRegistry
src-tauri/src/lib/terminal/registry.rs - Atomic admission and lifecycle registry retaining every terminal transaction through its joined receipt
src-tauri/src/lib/terminal/session_close.rs - The one close routine of a session: console-first pty drop, bounded wait, Job Object escalation, last-resort kill
src-tauri/src/lib/terminal/session_open.rs - Opens a pwsh session for a repo root: validation, shell and start-dir resolution, spawn, and the open/open-failed log...
src-tauri/src/lib/terminal/session_recovery_tests.rs - Failed-open ownership and slow-first-pass shutdown recovery regressions
src-tauri/src/lib/terminal/session_spawn_cleanup.rs - Complete process-tree and PTY cleanup for terminal opens that fail after spawn
src-tauri/src/lib/terminal/session_spawn_tests.rs - Unix PTY lifecycle checks for output, joined exit, and attached process cleanup
src-tauri/src/lib/terminal/session_spawn.rs - Resource-symmetric terminal spawn into an already-admitted transaction
src-tauri/src/lib/terminal/session.rs - One live terminal session: pty ends, child killer and Job Object, flow gate, exit record, phase and output channel
src-tauri/src/lib/terminal/shell.rs - Native profile-loaded shell command and terminal environment
src-tauri/src/lib/terminal/spawn_faults.rs - Test-only one-shot worker and cleanup failures scoped to the spawning test thread
src-tauri/src/lib/terminal/start_dir.rs - Maps a repo root to the directory pwsh starts in and the WSL entry command it runs for a native WSL root
src-tauri/src/lib/terminal/transaction_receipts.rs - Final and unresolved terminal receipts retaining process ownership through recovery
src-tauri/src/lib/terminal/transaction.rs - One admitted terminal transaction from Opening through joined Terminal receipt
src-tauri/src/lib/terminal/waiter_thread.rs - Retained child waiter that records exit and requests the single external reaper
src-tauri/src/lib/terminal/windows_build.rs - Reads the host's CurrentBuildNumber so xterm can enable ConPTY-aware reflow; None wherever it cannot be known
src-tauri/src/lib/terminal/windows_command_line.rs - Exact UTF-16 command-line and environment encoding for the Windows terminal child
src-tauri/src/lib/terminal/windows_process.rs - CreateProcessW owner applying ConPTY and Job-list attributes in one exclusive creation call
src-tauri/src/lib/terminal/windows_pty.rs - Windows ConPTY pipe and master owner feeding the exclusive at-creation process seam
src-tauri/src/lib/terminal/worker_start.rs - Start barrier ensuring terminal worker handles are retained before either worker runs
src-tauri/src/lib/wsl_control.rs - Shared bounded non-login WSL stdin-script boundary and native-root validation
```
