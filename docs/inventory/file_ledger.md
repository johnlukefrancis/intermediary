# File Ledger

Scope: src-tauri, crates, app, scripts (extensions: .cjs, .css, .d.cts, .d.mts, .d.ts, .html, .js, .mjs, .mts, .py, .rs, .scss, .ts, .tsx)

```text
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
app/src/components/bundles/bundle_entry_rename_input.tsx - In-place rename input mounted in a ZIPS tree row's name slot
app/src/components/bundles/bundle_explorer_directory.tsx - Recursive directory node for the lazy bundle file explorer, with selection, drag, and rename
app/src/components/bundles/bundle_explorer_file_row.tsx - File row for the bundle explorer with icon-driven include/exclude toggle, selection, and rename
app/src/components/bundles/bundle_explorer_row_menu.tsx - Right-click menu for a ZIPS tree row - file/folder actions plus cut/copy/paste/rename/delete
app/src/components/bundles/bundle_explorer_tree.tsx - Top-level directory/file list for the bundle explorer, carrying drop-target attributes and tree keyboard focus
app/src/components/bundles/bundle_file_explorer.tsx - Lazy file explorer for bundle directory/file inclusion, selection, clipboard, drag-move, and rename
app/src/components/bundles/bundle_list.tsx - Single LATEST bundle row (inline, no header)
app/src/components/bundles/bundle_row.tsx - Individual bundle row with drag support
app/src/components/bundles/bundle_selection_panel.tsx - Bundle build controls and file explorer selection panel
app/src/components/bundles/indeterminate_checkbox.tsx - Checkbox component that supports the DOM indeterminate state
app/src/components/bundles/preset_selector.tsx - Preset tabs/buttons for bundle building
app/src/components/bundles/tree_interaction_context.tsx - Owns the ZIPS-tree click model (select/toggle/range/expand) and hands each row its interaction props
app/src/components/confirm_modal.tsx - Generic confirmation dialog with portal rendering
app/src/components/context_menu.tsx - Generic reusable right-click context menu with glass aesthetic
app/src/components/diff_workspace.tsx - Read-only unified/combined diff viewer inside the shared workspace shell; flags merge conflicts
app/src/components/drag_error_notice.tsx - Small inline error notice for drag failures
app/src/components/empty_repo_state.tsx - Empty state UI when no repos are configured
app/src/components/file_context_menu_items.ts - Shared context-menu item builders for repo-relative file actions
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
app/src/components/options/excludes/excludes_normalizers.ts - Normalization helpers for global excludes inputs
app/src/components/options/excludes/excludes_recommendations.ts - Helpers for recommended global excludes toggles
app/src/components/options/excludes/excludes_updates.ts - Pure update helpers for global excludes toggles
app/src/components/options/excludes/use_excludes_state.ts - State and handlers for the excludes section UI
app/src/components/options/general_section.tsx - Options panel section for general app settings
app/src/components/options/layout/options_field_row.tsx - Shared label/control row primitive for responsive options fields
app/src/components/options/output_folder_section.tsx - Options panel controls for staging output folder
app/src/components/options/reset_section.tsx - Options panel reset settings section with confirmation modal
app/src/components/options/texture_picker.tsx - Small texture picker popover for tab theme selection
app/src/components/options/theme_section.tsx - Options panel theme controls (warm mode toggle + texture/accent per tab)
app/src/components/repo_workspace_panel.tsx - Repo workspace renderer for notes, text buffers, image previews, and text/image diffs
app/src/components/source_control/source_control_body.tsx - Phase-dependent body of the Source Control column: empty states or the three sections
app/src/components/source_control/source_control_column.tsx - Source Control column frame: status line, warnings, commit box, notices, body, menus, confirms
app/src/components/source_control/source_control_commit_box.tsx - Commit message textarea (Ctrl+Enter) and compact COMMIT button for the Source Control column
app/src/components/source_control/source_control_context_menu.ts - Right-click menu items for a source-control row, composed over the shared file items
app/src/components/source_control/source_control_copy.ts - Action labels, branch labels, and empty-state/error/confirm copy for the Source Control column
app/src/components/source_control/source_control_icons.tsx - Inline 24x24 stroke glyphs for source-control controls (stroke supplied by CSS)
app/src/components/source_control/source_control_row.tsx - One changed-path row: icon, name over dir, change badge, and a hover stage/unstage action
app/src/components/source_control/source_control_section.tsx - Collapsible MERGE CONFLICTS / STAGED CHANGES / CHANGES section with capped rows and a bulk action
app/src/components/source_control/source_control_status_line.tsx - Branch, ahead/behind, HEAD sha, and refresh/pull/push controls for the Source Control column
app/src/components/source_control/source_control_warnings.tsx - Conflict alert and warning rows (omitted paths, truncated status) in the Source Control column
app/src/components/status_bar.tsx - Status bar with connection status LED, error display, and options button
app/src/components/tab_bar.tsx - Tab navigation with grouped repo dropdown support and scroll overflow arrows
app/src/components/tab_bar/tab_bar_dropdowns.tsx - Dropdown panels for single-repo and grouped-repo tab-bar actions
app/src/components/tab_bar/tab_bar_items.tsx - Focused tab item renderers for single and grouped repository tabs
app/src/components/tab_remove_button.tsx - "x" button for removing repos with confirmation
app/src/components/terminal/terminal_column.tsx - TERMINAL rail body for one repo: tab strip over the imperative xterm host, plus the starting/exited/failed/empty notices
app/src/components/terminal/terminal_copy.ts - Console-prompt copy, button labels, and tooltips for the terminal column
app/src/components/terminal/terminal_exit_notice.tsx - Console-prompt notice floating over a tab that is starting, has exited, or failed to start
app/src/components/terminal/terminal_tab_strip.tsx - Tablist of one repo's terminal tabs (PWSH n, x per tab, + at the end); arrows move focus, click or Enter activates
app/src/components/text_workspace_semantics.tsx - Theme-aware semantic text layer for the workspace editor
app/src/components/text_workspace.tsx - Shared textarea surface for notes and scratch file viewing
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
app/src/hooks/repo_workspace_diff_loaders.ts - Diff loaders for the repo workspace hook: text patches and two-sided image snapshots
app/src/hooks/repo_workspace_types.ts - RepoWorkspace union (note, text, image, diff, image diff) and path helpers for the workspace hook
app/src/hooks/source_control/source_control_commands.ts - Public stage/unstage/discard/commit/push/pull command surface over the serialized action runner
app/src/hooks/source_control/source_control_counts.ts - SOURCE tab change count: distinct changed files, not area rows
app/src/hooks/source_control/source_control_failures.ts - Route an agent rejection by the effect certainty it carries, never by its error code namespace
app/src/hooks/source_control/source_control_reconcile.ts - Bounded backoff loop that resolves an action whose outcome the UI could not observe
app/src/hooks/source_control/source_control_refresh.ts - Status refresh timing owner: trailing debounce, delayed retries, in-flight dirty flag, post-mutation de-dup
app/src/hooks/source_control/source_control_types.ts - State-machine and action contract exposed by useSourceControlState
app/src/hooks/source_control/use_source_control_state.ts - Per-repo source-control status state machine with event-driven refresh and serialized actions
app/src/hooks/source_control/use_tree_decorations.tsx - Context delivering the built tree decorations to the recursive bundle explorer rows
app/src/hooks/terminal/use_terminal_group.ts - Subscribes a component to one repo's terminal group snapshot from the module-level registry
app/src/hooks/terminal/use_terminal_host.ts - Adopts the active terminal tab into a host element for its mount, parks on cleanup, and keeps it fitted, themed and f...
app/src/hooks/terminal/use_terminal_lifecycle.ts - App-level terminal lifecycle: closes groups of removed repos, mirrors window foreground to cursor blink, closes every...
app/src/hooks/use_agent.tsx - Agent context provider and connection management hook
app/src/hooks/use_bundle_state.ts - Per-repo bundle state management with event subscription
app/src/hooks/use_client_hello.ts - Custom hook for clientHello lifecycle with reconnect support
app/src/hooks/use_config_actions_extended.ts - Extended config actions for theme, legacy starred files, and recent files limit
app/src/hooks/use_config_actions_rail.ts - Config actions for the persisted right rail: the deck section (zips | source | terminal) and the rail width
app/src/hooks/use_config_actions.ts - Core config action factory functions for repo and bundle management
app/src/hooks/use_config_storage.ts - Config persistence + loading hook for use_config
app/src/hooks/use_config.tsx - Config persistence context provider and hook
app/src/hooks/use_deck_section.ts - One owner for the deck section: persisted right rail plus the handset-only FILES flag
app/src/hooks/use_drag.ts - Drag-out logic with on-demand staging
app/src/hooks/use_effective_ui_mode.ts - Derives runtime effective UI mode from preferred mode and live window state
app/src/hooks/use_file_actions.ts - Hook for OS-level file operations (reveal in file manager, open file)
app/src/hooks/use_file_selection.ts - Multi-file selection state hook with shift-range and ctrl/cmd-toggle support
app/src/hooks/use_image_blob_url.ts - Base64 image payload to a revocable Blob URL for workspace image and image-diff panes
app/src/hooks/use_mode_window_bounds_persistence.ts - Persists window bounds per mode from live resize events
app/src/hooks/use_mode_window_snap.ts - Applies per-mode window bounds when the active UI mode changes
app/src/hooks/use_motion_governor.ts - Pauses motion when window is not foreground (hidden, minimized, or unfocused) to save GPU
app/src/hooks/use_notes.ts - Per-repo note content hook with debounced save via Tauri commands
app/src/hooks/use_repo_state.ts - Per-repo file state management with event subscription
app/src/hooks/use_repo_workspace.ts - Repo-tab workspace state for notes, text buffers, image previews, and text/image diffs
app/src/hooks/use_resume_detector.ts - Detects likely OS sleep/wake resume using time gaps plus visibility/focus signals
app/src/hooks/use_startup_ready.ts - One-shot startup handshake to reveal main window after config load
app/src/hooks/use_tab_bar_dropdown.ts - Owns tab-bar dropdown open state, trigger containment, and anchored positioning
app/src/hooks/use_tab_bar_scroll.ts - Scroll overflow detection and snap-to-next-tab for the tab bar track
app/src/hooks/use_worktree_add.ts - Hook for adding worktrees to existing groups or single repos
app/src/lib/agent/agent_client_legacy.ts - Legacy hostPath/windowsPath envelope normalization for older agent payloads
app/src/lib/agent/agent_client.ts - WebSocket client with reconnection and message correlation
app/src/lib/agent/agent_request_timeouts.ts - Per-command UI request timeout ladder (strictly above the agent and host->WSL budgets)
app/src/lib/agent/connection_state.ts - Agent connection status types
app/src/lib/agent/error_codes.ts - Typed agent response error plus accessors for its code, message, and details
app/src/lib/agent/messages_import.ts - Typed helper for sending the drag-and-drop import command
app/src/lib/agent/messages_source_control.ts - Typed helpers for sending source-control status, diff, and action commands
app/src/lib/agent/messages_worktree.ts - Typed helper for sending the ZIPS-tree worktree action command
app/src/lib/agent/messages.ts - Typed helper functions for sending agent commands
app/src/lib/agent/transient_wsl_error.ts - Detect transient WSL transport/bootstrap failures and compute retry delays
app/src/lib/bundles/bundle_selection_visibility.ts - Shared path visibility helpers for bundle selection state
app/src/lib/bundles/flatten_visible_tree.ts - Flattens the lazily-loaded ZIPS tree into the exact visible row order the DOM renders
app/src/lib/files/file_feed.ts - Auto file feed filtering, ranking, and row metric helpers
app/src/lib/format_bytes.ts - Byte-count formatting shared by bundle rows and image-diff pane footers
app/src/lib/icons/file_family.ts - Extension-to-language-family mapping for file-type icon resolution
app/src/lib/icons/file_icon.css - Per-family colors and base styling for file-type icons
app/src/lib/icons/file_icons.tsx - Devicon-derived SVG path data and FileIcon component for file-type icons
app/src/lib/icons/index.ts - Barrel export for file-type icon system
app/src/lib/source_control/change_badges.ts - Single badge map (letter, variant, label) for every source-control change kind
app/src/lib/source_control/conflict_count.ts - Conflicts that block a commit: listed unmerged paths plus unmerged paths above the configured root
app/src/lib/source_control/tree_decorations.ts - Pure projection of a source-control status into per-file and rolled-up per-directory tree decorations
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
app/src/lib/terminal/terminal_theme.ts - Reads the deck's --terminal-* and --font-mono tokens into xterm theme and options; an empty token leaves xterm's defa...
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
app/src/shared/config/persisted_config_global_excludes_migration.ts - Global excludes migrations and legacy preset normalization
app/src/shared/config/persisted_config_migrations.ts - Persisted config migrations and legacy normalization
app/src/shared/config/persisted_config_repo_roots_migration.ts - Repo root migration helpers for persisted config normalization
app/src/shared/config/persisted_config.ts - Persisted config schema, types, and defaults
app/src/shared/config/repo_config.ts - RepoConfig schema and type
app/src/shared/config/repo_root.ts - Repo root authority union schema and path normalization helpers
app/src/shared/config/version.ts - Persisted config schema version
app/src/shared/global_excludes.ts - Global bundle exclude schema and UI options
app/src/shared/protocol_bundles.ts - Bundle-related agent protocol schemas and types
app/src/shared/protocol_events.ts - Agent event and file metadata schemas shared by protocol envelope parsing
app/src/shared/protocol_import.ts - Drag-and-drop file import command/result schemas shared with the agent
app/src/shared/protocol_repo_commands.ts - Core repo watch, refresh, staging, file-read, handshake, and bundle-list command schemas
app/src/shared/protocol_repo_topology.ts - Repo topology and lazy directory listing protocol schemas
app/src/shared/protocol_source_control.ts - Source-control status, diff, and action command/result schemas shared with the agents
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
app/src/styles/bundle_file_explorer_drop.css - Drag-and-drop import hover states for the ZIPS explorer tree and directory rows
app/src/styles/bundle_file_explorer_selection.css - Row selection, cut-dim, drag ghost, and list focus states for the ZIPS tree
app/src/styles/bundle_file_explorer.css - Lazy bundle file explorer rows and file include glow states
app/src/styles/bundle_list.css - Bundle list rows, ready pulse, and metadata styles
app/src/styles/bundle_selection_panel.css - Bundle selection panel shell and shared file explorer controls
app/src/styles/chrome.css - Unified header chrome styles for tab bar, status bar, and banners
app/src/styles/columns.css - Standard deck grid layout: Auto files, the drag divider, and the rail at its persisted width
app/src/styles/confirm_modal.css - Confirmation dialog overlay with glass panel styling
app/src/styles/context_menu.css - Right-click context menu with glass aesthetic
app/src/styles/deck_section_switcher.css - Segmented icon-rocker deck section tablist shared by the handset deck and the right rail
app/src/styles/deck_splitter.css - The drag divider between the left deck panel and the rail: occupies the column gap, lights up on hover and while drag...
app/src/styles/diff_workspace.css - Read-only diff viewer styling mirroring the workspace editor shell
app/src/styles/drag_error_notice.css - Inline glass toast for drag errors
app/src/styles/effects.css - Deck chassis frame, substrate (grid + grain), vignette, and glass utilities
app/src/styles/empty_repo_state.css - Empty state display when no repositories are configured
app/src/styles/handset_chassis.css - Handset v2 chassis frame, glow capsule accents, and section transitions
app/src/styles/handset_deck.css - Handset mode single-panel vertical deck layout
app/src/styles/image_diff_workspace.css - Side-by-side image diff panes, checkerboard slots, and handset stacking
app/src/styles/main.css - Global layout reset and base structure
app/src/styles/motion.css - Motion utilities, transition presets, and reduced-motion support
app/src/styles/options_controls.css - Buttons, text/number inputs, checkbox rows, and path display controls
app/src/styles/options_excludes.css - Collapsible sections, chevron toggle, and advanced grid/groups for excludes
app/src/styles/options_layout.css - Two-column grid layout, sections, rows, footer, and responsive fallback
app/src/styles/options_overlay.css - Overlay backdrop, panel shell, and keyframe animations
app/src/styles/options_theme.css - Theme section styles - color picker, texture picker, rename controls
app/src/styles/panels.css - Modular deck panel surfaces with framed edges and etched headers
app/src/styles/repo_rail.css - Right-rail panel with a slim section-switch header above the zips or source body
app/src/styles/scrollbars.css - Thin dark scrollbar styling with accent hints
app/src/styles/source_control_rows.css - Source Control change rows: icon, name over dir, badge, and hover stage/unstage action
app/src/styles/source_control_sections.css - Source Control collapsible section headers, bulk actions, and row containers
app/src/styles/source_control.css - Source Control column: status line, warnings, commit box, and notices
app/src/styles/status_bar.css - Status bar with connection LED, error display, and options button
app/src/styles/tab_bar_dropdown.css - Dropdown-specific styles for tab bar worktree actions
app/src/styles/tab_bar.css - Tab bar navigation with ASCII-instrument bracketed labels
app/src/styles/terminal_column.css - Terminal rail body: tab strip, xterm host and session element, console-prompt notices
app/src/styles/text_workspace_semantics.css - Semantic Markdown rendering layer for workspace text editors
app/src/styles/text_workspace.css - Shared workspace layout and editor/viewer styling for notes, text, and images
app/src/styles/theme_accents.css - Default accent color variables (runtime values applied via inline styles in app.tsx)
app/src/styles/theme_dark.css - Dark glass vintage theme - fills semantic token slots
app/src/styles/theme_light.css - Light theme overrides - warm parchment/linen tones, muted and soft
app/src/styles/theme_warm.css - Warm theme overrides - golden hour amber tones, saturated and warm
app/src/styles/tokens.css - Design system tokens - spacing, radii, blur, shadows, typography, motion
app/src/tabs/repo_tab_rail.tsx - Composes the ZIPS, SOURCE and TERMINAL rail bodies for one repo tab; RepoTab hands them to the rail or the handset deck
app/src/tabs/repo_tab.tsx - Generic repo tab component with Auto files, the right rail (zips | source | terminal), and the workspace
app/src/types/agent_supervisor.ts - Types for Tauri host-agent supervisor responses
app/src/types/app_paths.ts - TypeScript interface matching Rust AppPaths struct
app/src/vite_env.d.ts - Vite client type declarations
crates/im_agent/src/bundles/bundle_builder_blocking.rs - Blocking bundle build steps and filesystem operations
crates/im_agent/src/bundles/bundle_builder_tests.rs - Tests for bundle builder helpers
crates/im_agent/src/bundles/bundle_builder.rs - Bundle build orchestration using the im_bundle library
crates/im_agent/src/bundles/bundle_lister.rs - Bundle listing and latest selection logic
crates/im_agent/src/bundles/bundle_progress.rs - Bundle progress forwarding from im_bundle to agent events
crates/im_agent/src/bundles/ignore_rules.rs - Centralized ignore patterns for bundle building and scanning
crates/im_agent/src/bundles/mod.rs - Bundle helpers for the agent
crates/im_agent/src/error/agent_error.rs - AgentError type and mapping to protocol error responses
crates/im_agent/src/error/mod.rs - Error module exports for the agent runtime
crates/im_agent/src/error/mutation_effect.rs - Outcome certainty (`details.effect`) carried by every source-control mutation error
crates/im_agent/src/lib.rs - Library root for the Intermediary WSL agent daemon
crates/im_agent/src/logging/json_logger.rs - JSONL logger that writes to agent_latest.log and optionally mirrors to stdout/stderr
crates/im_agent/src/logging/mod.rs - Logging exports and helpers for the agent
crates/im_agent/src/main.rs - WSL agent daemon entry point
crates/im_agent/src/protocol/cancel_bundle_tests.rs - Protocol tests for cancellable bundle build messages
crates/im_agent/src/protocol/commands_import.rs - UI-to-agent command payload for importing external OS files into a repo directory
crates/im_agent/src/protocol/commands_source_control.rs - UI-to-agent source-control command payloads (status, diff, tagged actions)
crates/im_agent/src/protocol/commands_tr_fleet.rs - TR fleet command payloads for host-agent build-server status and recovery controls
crates/im_agent/src/protocol/commands_worktree.rs - UI-to-agent command payload for deleting, moving, copying, and renaming worktree entries
crates/im_agent/src/protocol/commands.rs - UI-to-agent command payloads for the WebSocket protocol
crates/im_agent/src/protocol/envelopes.rs - Protocol envelope types for request/response messaging
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
crates/im_agent/src/protocol/tests_shutdown.rs - Wire-shape tests for the shutdown command and its result
crates/im_agent/src/protocol/tests_source_control.rs - Wire-shape tests for the source-control command and status payloads
crates/im_agent/src/protocol/tests.rs - Protocol envelope serialization and backward-compat tests
crates/im_agent/src/protocol/tr_fleet_tests.rs - TR fleet protocol command/response serialization tests
crates/im_agent/src/repos/categorizer.rs - File kind classification based on globs and fallback heuristics
crates/im_agent/src/repos/file_activity.rs - Activity metadata updates for recent file ranking
crates/im_agent/src/repos/generated_code_extensions.rs - Generated extension list for fallback code classification in the Rust agent. Generated by: scripts/classification/gen...
crates/im_agent/src/repos/ignore_matcher.rs - Ignore glob matcher for repo watcher
crates/im_agent/src/repos/image_file_reader.rs - Repo-relative image file reader for in-app preview workspaces
crates/im_agent/src/repos/import/copy.rs - The import conflict pre-pass and the policy-specific copy that writes into the worktree
crates/im_agent/src/repos/import/mod.rs - Copying external OS files and folders into one directory of a repo worktree
crates/im_agent/src/repos/import/sources.rs - Source translation, per-source validation, and the bounded walk that plans an import
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
crates/im_agent/src/repos/repo_watcher_events.rs - Event handling for repo watcher changes and rename mapping
crates/im_agent/src/repos/repo_watcher_tests.rs - Unit tests for the repo watcher's initial-entries ignore filtering
crates/im_agent/src/repos/repo_watcher.rs - Notify-based repo watcher with MRU and event emission
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
crates/im_agent/src/runtime/config_fingerprint.rs - Compute watcher-relevant config fingerprint
crates/im_agent/src/runtime/config.rs - Minimal app configuration structures for the agent runtime
crates/im_agent/src/runtime/mod.rs - Agent runtime exports
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
crates/im_agent/src/server/event_bus.rs - Broadcast agent events to connected WebSocket clients
crates/im_agent/src/server/handshake_auth.rs - WSL-agent websocket handshake token validation utilities
crates/im_agent/src/server/mod.rs - WebSocket server module exports
crates/im_agent/src/server/runtime_identity.rs - Compute and expose the running agent executable identity during WebSocket handshake
crates/im_agent/src/server/shutdown.rs - The one drain-then-exit owner shared by the shutdown command and the process signals
crates/im_agent/src/server/shutdown/tests.rs - Unit tests for the drain gate: a held mutation keeps the drain waiting, and only idle reports drained
crates/im_agent/src/server/stdin_eof.rs - The supervisor's stdin pipe as a shutdown owner - EOF on fd 0 is a drain request
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
crates/im_bundle/src/fs_atomic.rs - Rename that refuses to replace an existing destination, on the two platforms the product runs on
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
crates/im_bundle/src/lib.rs - Library root for bundle scanning and zip creation
crates/im_bundle/src/manifest.rs - Bundle manifest structure and serialization
crates/im_bundle/src/omission.rs - Why a changed repository path fell outside the bundle selection
crates/im_bundle/src/plan.rs - Bundle plan schema and loader for im_bundle_cli
crates/im_bundle/src/process_job_termination.rs - Bounded forced termination and observation for a Windows Job Object
crates/im_bundle/src/process_job.rs - Windows Job Object ownership of a spawned process tree, shared by the Git runner and the app's agent supervisor
crates/im_bundle/src/progress_sink.rs - Progress sink interfaces for bundle build reporting
crates/im_bundle/src/progress.rs - Throttled NDJSON progress emitter for bundle scanning and zipping
crates/im_bundle/src/scanner.rs - Bundle scanning logic with ignore rules and exclusions
crates/im_bundle/src/selection.rs - Canonical bundle-selection predicate shared by scanning and Git capture
crates/im_bundle/src/writer_tests.rs - Tests for bundle writer behavior and progress ordering
crates/im_bundle/src/writer.rs - Bundle zip writer with scanning, manifest, and progress
crates/im_bundle/src/zip_entry.rs - Single file entry writer for bundle zip archives
crates/im_bundle/tests/git_evidence_test.rs - End-to-end witness tests for selection-bounded bundle Git evidence
crates/im_bundle/tests/git_large_selection_test.rs - Windows-scale witness for host-safe selected Git diff path batching
crates/im_bundle/tests/scanner_test.rs - Integration tests for bundle scanner behavior
crates/im_bundle/tests/size_capped_reads_test.rs - Ensures bundle writes only the bytes present at file-open time even if file grows
crates/im_host_agent/src/config.rs - Host agent environment configuration parsing
crates/im_host_agent/src/error_codes.rs - Shared host-agent error code constants for routing and WSL backend failures
crates/im_host_agent/src/lib.rs - Library root for the Intermediary host agent daemon
crates/im_host_agent/src/main.rs - Host agent daemon entry point
crates/im_host_agent/src/runtime/host_runtime_helpers.rs - Host-runtime helper functions for config parsing and repo-command metadata
crates/im_host_agent/src/runtime/host_runtime/bundle_forwarding.rs - Build-bundle host dispatch and WSL forwarding helpers for HostRuntime
crates/im_host_agent/src/runtime/host_runtime/host_dispatch.rs - Host-rooted repo command dispatch onto the local backend for HostRuntime
crates/im_host_agent/src/runtime/host_runtime/mod.rs - Host runtime command routing and clientHello orchestration for host and WSL backends
crates/im_host_agent/src/runtime/host_runtime/shutdown_targets.rs - The two things a host-agent shutdown must reach: the WSL backend client and the host locks
crates/im_host_agent/src/runtime/host_runtime/wsl_routing_tests.rs - WSL transport transition tests for host runtime routing
crates/im_host_agent/src/runtime/host_runtime/wsl_routing.rs - WSL forwarding, generation-aware clientHello replay, and transport error emission for HostRuntime
crates/im_host_agent/src/runtime/host_runtime/wsl_transport_epoch_state.rs - Tracks WSL transport error emission by backend connection generation for de-noised offline transitions
crates/im_host_agent/src/runtime/local_host_backend.rs - Host-native local backend for repo watch, staging, and bundle operations
crates/im_host_agent/src/runtime/local_host_import_backend.rs - Host-native file import execution that never holds the runtime lock across the copy
crates/im_host_agent/src/runtime/local_host_repo_backend.rs - Host-native repo read and topology operations
crates/im_host_agent/src/runtime/local_host_source_control_backend.rs - Host-native source-control execution that never holds the runtime lock across Git
crates/im_host_agent/src/runtime/local_host_worktree_backend.rs - Host-native worktree entry actions that never hold the runtime lock across the write
crates/im_host_agent/src/runtime/mod.rs - Host runtime exports for backend routing and local host handling
crates/im_host_agent/src/runtime/repo_backend.rs - Repo backend kind mapping for host-agent routing
crates/im_host_agent/src/runtime/router.rs - Repo-id command routing for host-agent backend selection
crates/im_host_agent/src/runtime/tr_fleet_service.rs - Host-agent TR fleet status polling and recovery action execution
crates/im_host_agent/src/runtime/wsl_client_hello_cache.rs - Caches and fingerprints latest WSL clientHello payload for resilient bootstrap replay
crates/im_host_agent/src/server/connection.rs - Host-agent per-connection WebSocket handling and response serialization
crates/im_host_agent/src/server/dispatch.rs - Host-agent command dispatch over routed runtime backends
crates/im_host_agent/src/server/handshake_auth.rs - Host-agent websocket handshake token and origin validation utilities
crates/im_host_agent/src/server/mod.rs - Host-agent WebSocket server module exports
crates/im_host_agent/src/server/shutdown_dispatch.rs - Host-agent shutdown: drain the WSL backend first, then this process, then exit
crates/im_host_agent/src/server/shutdown_dispatch/tests.rs - Unit tests for the WSL-unavailable/outstanding-mutation shutdown decision
crates/im_host_agent/src/server/ws_server.rs - Host-agent WebSocket accept loop and connection dispatch
crates/im_host_agent/src/wsl/mod.rs - WSL backend client module exports
crates/im_host_agent/src/wsl/wsl_backend_client.rs - Persistent WebSocket client for forwarding commands/events to the WSL backend agent
crates/im_host_agent/src/wsl/wsl_backend_client/client_loop.rs - The WSL backend connect/reconnect loop and the answers it gives while the backend is unreachable
crates/im_host_agent/src/wsl/wsl_backend_client/tests_outstanding.rs - Unit tests for the outstanding-mutation ledger: decode-site clearing, timeouts, and offline answers
crates/im_host_agent/src/wsl/wsl_backend_client/tests_timeouts.rs - Unit tests for the per-command forward timeout ladder
crates/im_host_agent/src/wsl/wsl_backend_client/tests.rs - Unit tests for WSL backend forwarding, cancellation, and outstanding-mutation tracking
crates/im_host_agent/src/wsl/wsl_backend_client/timeouts.rs - The host->WSL request-timeout ladder and the agent-side worst case each tier covers
crates/im_host_agent/src/wsl/wsl_backend_connection.rs - Connected WSL backend request loop and pending response handling
crates/im_host_agent/src/wsl/wsl_backend_messages.rs - WSL-backend message parsing and pending-response helpers
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
scripts/zip/zip_bundles.mjs - Builds timestamped Intermediary zip bundles for ChatGPT context.
src-tauri/build.rs - Tauri build script
src-tauri/src/bin/intermediary.rs - Binary entry point for Tauri app
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
src-tauri/src/lib/commands/mod.rs - Tauri command exports
src-tauri/src/lib/commands/notes.rs - Tauri commands for per-repo plain-text notes persistence
src-tauri/src/lib/commands/paths.rs - get_app_paths command implementation and path conversion utilities
src-tauri/src/lib/commands/reset.rs - Tauri command to clear staging artifacts and caches
src-tauri/src/lib/commands/startup_window_bounds.rs - Resolve and apply persisted launch bounds for startup windows
src-tauri/src/lib/commands/startup.rs - Startup readiness command for splashscreen -> main transition
src-tauri/src/lib/commands/terminal.rs - Tauri commands of the integrated terminal: open, raw-body write, resize, ack, close and the Rust-side clipboard read
src-tauri/src/lib/commands/wsl_distro.rs - Resolve WSL distro override from persisted app config for command-time path conversion
src-tauri/src/lib/config/generated_code_globs.rs - Generated default code globs for Rust-side persisted config migration. Generated by: scripts/classification/generate_...
src-tauri/src/lib/config/io.rs - Config file I/O with atomic writes and error handling
src-tauri/src/lib/config/io/migration_tests.rs - Focused config migration regression tests
src-tauri/src/lib/config/io/repo_root_migration.rs - Legacy repository root migration helpers for config loading
src-tauri/src/lib/config/io/schema_migrations.rs - Versioned persisted-config schema migrations
src-tauri/src/lib/config/io/tests.rs - Unit tests for config I/O and migration behavior
src-tauri/src/lib/config/mod.rs - Configuration persistence module
src-tauri/src/lib/config/path.rs - Resolve persisted config file location for app commands and setup
src-tauri/src/lib/config/types.rs - Persisted configuration types for Intermediary
src-tauri/src/lib/config/types/model.rs - Supporting persisted configuration model types
src-tauri/src/lib/config/types/tests.rs - Tests for persisted configuration types
src-tauri/src/lib/config/types/validation.rs - Persisted configuration validation rules and invariants
src-tauri/src/lib/mod.rs - Library root - Tauri setup and plugin registration
src-tauri/src/lib/obs/logging.rs - File-based logger writing to run_latest.txt
src-tauri/src/lib/obs/mod.rs - Observability module exports
src-tauri/src/lib/paths/app_paths.rs - Application path resolution logic
src-tauri/src/lib/paths/mod.rs - Path resolution module exports
src-tauri/src/lib/paths/repo_root_resolver.rs - Path-native repo root resolver for user-selected repo paths
src-tauri/src/lib/paths/wsl_convert.rs - Windows <-> WSL path conversion utilities
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
src-tauri/src/lib/terminal/session_spawn_cleanup.rs - Complete process-tree and PTY cleanup for terminal opens that fail after spawn
src-tauri/src/lib/terminal/session_spawn_tests.rs - Lifecycle oracle of a spawned session on the Linux toolchain: bytes then exit frame, and the console-first close
src-tauri/src/lib/terminal/session_spawn.rs - Resource-symmetric terminal spawn into an already-admitted transaction
src-tauri/src/lib/terminal/session.rs - One live terminal session: pty ends, child killer and Job Object, flow gate, exit record, phase and output channel
src-tauri/src/lib/terminal/shell.rs - Profile-faithful PowerShell command and exact inherited environment for terminal spawn
src-tauri/src/lib/terminal/start_dir.rs - Maps a repo root to the directory pwsh starts in and the WSL entry command it runs for a native WSL root
src-tauri/src/lib/terminal/transaction.rs - One admitted terminal transaction from Opening through joined Terminal receipt
src-tauri/src/lib/terminal/waiter_thread.rs - Retained child waiter that records exit and requests the single external reaper
src-tauri/src/lib/terminal/windows_build.rs - Reads the host's CurrentBuildNumber so xterm can enable ConPTY-aware reflow; None wherever it cannot be known
src-tauri/src/lib/terminal/windows_command_line.rs - Exact UTF-16 command-line and environment encoding for the Windows terminal child
src-tauri/src/lib/terminal/windows_process.rs - CreateProcessW owner applying ConPTY and Job-list attributes in one exclusive creation call
src-tauri/src/lib/terminal/windows_pty.rs - Windows ConPTY pipe and master owner feeding the exclusive at-creation process seam
src-tauri/src/lib/terminal/worker_start.rs - Start barrier ensuring terminal worker handles are retained before either worker runs
src-tauri/src/lib/wsl_control.rs - Shared bounded non-login WSL stdin-script boundary and native-root validation
```
