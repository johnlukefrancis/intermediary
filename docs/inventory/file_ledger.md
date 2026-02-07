# File Ledger

Scope: src-tauri, crates, app, scripts (extensions: .cjs, .css, .d.cts, .d.mts, .d.ts, .html, .js, .mjs, .mts, .py, .rs, .scss, .ts, .tsx)

```text
app/index.html - index module
app/src/app.tsx - Root component with config-driven tab state management
app/src/components/add_repo_button.tsx - "+" button for adding new repositories via directory picker
app/src/components/agent_offline_banner.tsx - Banner with diagnostics when the host agent endpoint is offline
app/src/components/bundles/bundle_column.tsx - Main bundles column component
app/src/components/bundles/bundle_list.tsx - Single LATEST bundle row (inline, no header)
app/src/components/bundles/bundle_row.tsx - Individual bundle row with drag support
app/src/components/bundles/bundle_selection_panel.tsx - Selection UI for bundle building (root toggle, dir checkboxes, subdir exclusions)
app/src/components/bundles/preset_selector.tsx - Preset tabs/buttons for bundle building
app/src/components/confirm_modal.tsx - Generic confirmation dialog with portal rendering
app/src/components/drag_error_notice.tsx - Small inline error notice for drag failures
app/src/components/empty_repo_state.tsx - Empty state UI when no repos are configured
app/src/components/file_list_column.tsx - Column wrapper that renders a list of FileRow components
app/src/components/file_row.tsx - Draggable file row with file-type icon, click-to-copy, and star toggle
app/src/components/group_remove_button.tsx - Remove button for grouped repos with confirmation
app/src/components/layout/three_column.tsx - Three-column layout component with modular deck panels (Docs | Code | Zips)
app/src/components/options_overlay.tsx - Full-screen transparent overlay with options panel for app settings
app/src/components/options/agent_section.tsx - Options panel controls for host + WSL agent lifecycle
app/src/components/options/excludes_section.tsx - Excludes configuration section for the options panel
app/src/components/options/excludes/advanced_group.tsx - Collapsible checkbox group for advanced excludes options
app/src/components/options/excludes/excludes_normalizers.ts - Normalization helpers for global excludes inputs
app/src/components/options/excludes/excludes_recommendations.ts - Helpers for recommended global excludes toggles
app/src/components/options/excludes/excludes_updates.ts - Pure update helpers for global excludes toggles
app/src/components/options/excludes/use_excludes_state.ts - State and handlers for the excludes section UI
app/src/components/options/general_section.tsx - Options panel section for general app settings
app/src/components/options/output_folder_section.tsx - Options panel controls for staging output folder
app/src/components/options/reset_section.tsx - Options panel reset settings section with confirmation modal
app/src/components/options/texture_picker.tsx - Small texture picker popover for tab theme selection
app/src/components/options/theme_section.tsx - Options panel theme controls (warm mode toggle + texture/accent per tab)
app/src/components/status_bar.tsx - Status bar with connection status LED, error display, and options button
app/src/components/tab_bar.tsx - Tab navigation with grouped repo dropdown support
app/src/components/tab_bar/tab_bar_items.tsx - Focused tab item renderers for single and grouped repository tabs
app/src/components/tab_remove_button.tsx - "x" button for removing repos with confirmation
app/src/hooks/agent/use_agent_probe.ts - Probe the agent port when disconnected for diagnostics
app/src/hooks/agent/use_agent_shutdown.ts - Stop the WSL agent when the app window is closing
app/src/hooks/agent/use_agent_supervisor.ts - Manage auto-start and restart of host-agent supervision with optional Windows WSL backend
app/src/hooks/use_agent.tsx - Agent context provider and connection management hook
app/src/hooks/use_bundle_state.ts - Per-repo bundle state management with event subscription
app/src/hooks/use_client_hello.ts - Custom hook for clientHello lifecycle with reconnect support
app/src/hooks/use_config_actions_extended.ts - Extended config actions for theme, starred files, and recent files limit
app/src/hooks/use_config_actions.ts - Core config action factory functions for repo and bundle management
app/src/hooks/use_config_storage.ts - Config persistence + loading hook for use_config
app/src/hooks/use_config.tsx - Config persistence context provider and hook
app/src/hooks/use_drag.ts - Drag-out logic with on-demand staging
app/src/hooks/use_motion_governor.ts - Pauses motion when window is hidden/minimized to save GPU
app/src/hooks/use_repo_state.ts - Per-repo file state management with event subscription
app/src/hooks/use_starred_files.ts - Hook exposing starred file state and actions for a repo
app/src/hooks/use_worktree_add.ts - Hook for adding worktrees to existing groups or single repos
app/src/lib/agent/agent_client.ts - WebSocket client with reconnection and message correlation
app/src/lib/agent/connection_state.ts - Agent connection status types
app/src/lib/agent/messages.ts - Typed helper functions for sending agent commands
app/src/lib/icons/file_family.ts - Extension-to-language-family mapping for file-type icon resolution
app/src/lib/icons/file_icon.css - Per-family colors and base styling for file-type icons
app/src/lib/icons/file_icons.tsx - Devicon-derived SVG path data and FileIcon component for file-type icons
app/src/lib/icons/index.ts - Barrel export for file-type icon system
app/src/lib/theme/accent_utils.ts - Convert hex accent color to CSS variable values for runtime theming
app/src/lib/theme/texture_catalog.ts - Build-time texture catalog for theme substrate/dither selection
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
app/src/shared/protocol.ts - Agent<->UI WebSocket protocol types with Zod validation
app/src/shared/repo_utils.ts - Utility functions for repo ID generation and path handling
app/src/styles/a11y.css - Accessibility utilities - focus rings, disabled states, screen reader helpers
app/src/styles/agent_offline_banner.css - Banner styling for offline WSL agent diagnostics
app/src/styles/badges.css - Bracket-style badge tags for status indicators [A] [M] [D] [STAGED] [LATEST]
app/src/styles/bundle_column.css - Hardware-style bundle column with segmented controls and command buttons
app/src/styles/chrome.css - Unified header chrome styles for tab bar, status bar, and banners
app/src/styles/columns.css - Three-column deck grid layout with intentional gutters (Docs | Code | Zips)
app/src/styles/confirm_modal.css - Confirmation dialog overlay with glass panel styling
app/src/styles/drag_error_notice.css - Inline glass toast for drag errors
app/src/styles/effects.css - Deck chassis frame, substrate (grid + grain), vignette, and glass utilities
app/src/styles/empty_repo_state.css - Empty state display when no repositories are configured
app/src/styles/file_row.css - File row with file-type icon, bottom change glow, and full-row drag
app/src/styles/main.css - Global layout reset and base structure
app/src/styles/motion.css - Motion utilities, transition presets, and reduced-motion support
app/src/styles/options_controls.css - Buttons, text/number inputs, checkbox rows, and path display controls
app/src/styles/options_excludes.css - Collapsible sections, chevron toggle, and advanced grid/groups for excludes
app/src/styles/options_layout.css - Two-column grid layout, sections, rows, footer, and responsive fallback
app/src/styles/options_overlay.css - Overlay backdrop, panel shell, close button, and keyframe animations
app/src/styles/options_theme.css - Theme section styles - color picker, texture picker, rename controls
app/src/styles/panels.css - Modular deck panel surfaces with framed edges and etched headers
app/src/styles/scrollbars.css - Thin dark scrollbar styling with accent hints
app/src/styles/status_bar.css - Status bar with connection LED, error display, and options button
app/src/styles/tab_bar_dropdown.css - Dropdown-specific styles for tab bar worktree actions
app/src/styles/tab_bar.css - Tab bar navigation with ASCII-instrument bracketed labels
app/src/styles/theme_accents.css - Default accent color variables (runtime values applied via inline styles in app.tsx)
app/src/styles/theme_dark.css - Dark glass vintage theme - fills semantic token slots
app/src/styles/theme_warm.css - Warm theme overrides - saturated caramel/toffee brown tones
app/src/styles/tokens.css - Design system tokens - spacing, radii, blur, shadows, typography, motion
app/src/tabs/repo_tab.tsx - Generic repo tab component with 3-column layout
app/src/types/agent_supervisor.ts - Types for Tauri host-agent supervisor responses
app/src/types/app_paths.ts - TypeScript interface matching Rust AppPaths struct
app/src/vite_env.d.ts - Vite client type declarations
crates/im_agent/src/bundles/bundle_builder_blocking.rs - Blocking bundle build steps and filesystem operations
crates/im_agent/src/bundles/bundle_builder_tests.rs - Tests for bundle builder helpers
crates/im_agent/src/bundles/bundle_builder.rs - Bundle build orchestration using the im_bundle library
crates/im_agent/src/bundles/bundle_lister.rs - Bundle listing and latest selection logic
crates/im_agent/src/bundles/bundle_progress.rs - Bundle progress forwarding from im_bundle to agent events
crates/im_agent/src/bundles/git_info.rs - Best-effort git info lookup for bundle manifests
crates/im_agent/src/bundles/ignore_rules.rs - Centralized ignore patterns for bundle building and scanning
crates/im_agent/src/bundles/mod.rs - Bundle helpers for the agent
crates/im_agent/src/error/agent_error.rs - AgentError type and mapping to protocol error responses
crates/im_agent/src/error/mod.rs - Error module exports for the agent runtime
crates/im_agent/src/lib.rs - Library root for the Intermediary WSL agent daemon
crates/im_agent/src/logging/json_logger.rs - JSONL logger that writes to agent_latest.log and stdout/stderr
crates/im_agent/src/logging/mod.rs - Logging exports and helpers for the agent
crates/im_agent/src/main.rs - WSL agent daemon entry point
crates/im_agent/src/protocol/commands.rs - UI-to-agent command payloads for the WebSocket protocol
crates/im_agent/src/protocol/envelopes.rs - Protocol envelope types for request/response messaging
crates/im_agent/src/protocol/events.rs - Agent event payloads and file entry types
crates/im_agent/src/protocol/mod.rs - WebSocket protocol types for the agent
crates/im_agent/src/protocol/responses.rs - Agent-to-UI response payloads for the WebSocket protocol
crates/im_agent/src/protocol/tests.rs - Protocol envelope serialization tests
crates/im_agent/src/repos/categorizer.rs - File kind classification based on globs and fallback heuristics
crates/im_agent/src/repos/generated_code_extensions.rs - Generated extension list for fallback code classification in the Rust agent. Generated by: scripts/classification/gen...
crates/im_agent/src/repos/ignore_matcher.rs - Ignore glob matcher for repo watcher
crates/im_agent/src/repos/mod.rs - Repository scanning module exports
crates/im_agent/src/repos/mru_index.rs - MRU index for recent file changes
crates/im_agent/src/repos/recent_files_store.rs - Persist recent files with debounced atomic writes
crates/im_agent/src/repos/repo_top_level.rs - Scan top-level directories and files in a repo
crates/im_agent/src/repos/repo_watcher_events.rs - Event handling for repo watcher changes and rename mapping
crates/im_agent/src/repos/repo_watcher.rs - Notify-based repo watcher with MRU and event emission
crates/im_agent/src/repos/watcher_error.rs - Watcher error classification and event shaping
crates/im_agent/src/runtime/config_fingerprint.rs - Compute watcher-relevant config fingerprint
crates/im_agent/src/runtime/config.rs - Minimal app configuration structures for the agent runtime
crates/im_agent/src/runtime/mod.rs - Agent runtime exports
crates/im_agent/src/runtime/state_watchers.rs - Watcher lifecycle helpers for agent runtime state
crates/im_agent/src/runtime/state.rs - Agent runtime state and option handlers
crates/im_agent/src/server/connection.rs - Per-connection WebSocket handling and request routing
crates/im_agent/src/server/connection/dispatch.rs - Command dispatch for WebSocket request handling
crates/im_agent/src/server/event_bus.rs - Broadcast agent events to connected WebSocket clients
crates/im_agent/src/server/mod.rs - WebSocket server module exports
crates/im_agent/src/server/ws_server.rs - WebSocket accept loop and connection dispatch
crates/im_agent/src/staging/mod.rs - Staging module exports
crates/im_agent/src/staging/path_bridge.rs - Staging path bridging between WSL and Windows layouts
crates/im_agent/src/staging/stager.rs - Atomic staging of files into the Windows-accessible directory
crates/im_bundle/src/bin/im_bundle_cli.rs - CLI entry point for im_bundle - scans and writes bundle zip
crates/im_bundle/src/compression_policy.rs - Compression policy for bundle entries based on extension and size
crates/im_bundle/src/error.rs - Error types for bundle scanning and zip writing
crates/im_bundle/src/global_excludes.rs - Normalize and apply user-configurable global excludes for bundle scanning
crates/im_bundle/src/lib.rs - Library root for bundle scanning and zip creation
crates/im_bundle/src/manifest.rs - Bundle manifest structure and serialization
crates/im_bundle/src/plan.rs - Bundle plan schema and loader for im_bundle_cli
crates/im_bundle/src/progress_sink.rs - Progress sink interfaces for bundle build reporting
crates/im_bundle/src/progress.rs - Throttled NDJSON progress emitter for bundle scanning and zipping
crates/im_bundle/src/scanner.rs - Bundle scanning logic with ignore rules and exclusions
crates/im_bundle/src/writer_tests.rs - Tests for bundle writer behavior and progress ordering
crates/im_bundle/src/writer.rs - Bundle zip writer with scanning, manifest, and progress
crates/im_bundle/tests/scanner_test.rs - Integration tests for bundle scanner behavior
crates/im_bundle/tests/size_capped_reads_test.rs - Ensures bundle writes only the bytes present at file-open time even if file grows
crates/im_host_agent/src/config.rs - Host agent environment configuration parsing
crates/im_host_agent/src/error_codes.rs - Shared host-agent error code constants for routing and WSL backend failures
crates/im_host_agent/src/lib.rs - Library root for the Intermediary host agent daemon
crates/im_host_agent/src/main.rs - Host agent daemon entry point
crates/im_host_agent/src/runtime/host_runtime_helpers.rs - Host-runtime helper functions for config parsing and repo-command metadata
crates/im_host_agent/src/runtime/host_runtime.rs - Host runtime that routes protocol commands to Windows-local or WSL backend
crates/im_host_agent/src/runtime/local_windows_backend.rs - Windows-native local backend for repo watch, staging, and bundle operations
crates/im_host_agent/src/runtime/mod.rs - Host runtime exports for backend routing and local Windows handling
crates/im_host_agent/src/runtime/repo_backend.rs - Repo backend kind mapping for host-agent routing
crates/im_host_agent/src/runtime/router.rs - Repo-id command routing for host-agent backend selection
crates/im_host_agent/src/server/connection.rs - Host-agent per-connection WebSocket handling and response serialization
crates/im_host_agent/src/server/dispatch.rs - Host-agent command dispatch over routed runtime backends
crates/im_host_agent/src/server/mod.rs - Host-agent WebSocket server module exports
crates/im_host_agent/src/server/ws_server.rs - Host-agent WebSocket accept loop and connection dispatch
crates/im_host_agent/src/wsl/mod.rs - WSL backend client module exports
crates/im_host_agent/src/wsl/wsl_backend_client.rs - Persistent WebSocket client for forwarding commands/events to the WSL backend agent
crates/im_host_agent/src/wsl/wsl_backend_messages.rs - WSL-backend message parsing and pending-response helpers
scripts/classification/code_extensions_source.mjs - Pinned baseline + local overrides for code-classification file extensions.
scripts/classification/generate_code_classification_artifacts.mjs - Generate TS/Rust code-classification extension artifacts from a pinned source list.
scripts/fileledger/add_file_headers.mjs - Adds missing header comments (path + description) to source files using the ledger output.
scripts/fileledger/gen_file_ledger.mjs - Generates human+machine file ledgers for Intermediary code sources.
scripts/icons/generate_icons.mjs - Generate all icon sizes from a source PNG. Usage: node scripts/generate_icons.mjs [source.png] Default source: app/as...
scripts/icons/resize_preview_icons.mjs - Resize preview geometry icons from raw assets to display sizes. Outputs 40px (1x) and 80px (2x retina) versions.
scripts/zip/zip_bundles.mjs - Builds timestamped Intermediary zip bundles for ChatGPT context.
src-tauri/build.rs - Tauri build script
src-tauri/src/bin/intermediary.rs - Binary entry point for Tauri app
src-tauri/src/lib/agent/install.rs - Install bundled agent runtimes into app local data with platform-specific requirements
src-tauri/src/lib/agent/mod.rs - Host-agent supervisor module exports (with optional Windows WSL backend)
src-tauri/src/lib/agent/process_control.rs - Spawn helpers for host/WSL agents and readiness probing
src-tauri/src/lib/agent/supervisor_helpers.rs - Shared state and helper utilities for host-agent supervision with optional Windows WSL backend
src-tauri/src/lib/agent/supervisor.rs - Public host-agent supervisor types and wiring
src-tauri/src/lib/agent/supervisor/lifecycle.rs - Host-agent-first supervisor lifecycle implementation with optional Windows WSL backend
src-tauri/src/lib/agent/supervisor/processes.rs - Process lifecycle helpers for host/WSL supervisor tasks
src-tauri/src/lib/agent/types.rs - Types for supervising host agent lifecycle with optional Windows WSL backend
src-tauri/src/lib/commands/agent_control.rs - Tauri commands to manage host + optional WSL agent supervision
src-tauri/src/lib/commands/agent_probe.rs - Probe local host-agent port availability for diagnostics
src-tauri/src/lib/commands/config.rs - Tauri commands for config persistence
src-tauri/src/lib/commands/file_manager.rs - Open folders in the host OS file manager
src-tauri/src/lib/commands/mod.rs - Tauri command exports
src-tauri/src/lib/commands/paths.rs - get_app_paths command implementation and path conversion utilities
src-tauri/src/lib/commands/reset.rs - Tauri command to clear staging artifacts and caches
src-tauri/src/lib/config/generated_code_globs.rs - Generated default code globs for Rust-side persisted config migration. Generated by: scripts/classification/generate_...
src-tauri/src/lib/config/io.rs - Config file I/O with atomic writes and error handling
src-tauri/src/lib/config/io/tests.rs - Unit tests for config I/O and migration behavior
src-tauri/src/lib/config/mod.rs - Configuration persistence module
src-tauri/src/lib/config/types.rs - Persisted configuration types for Intermediary
src-tauri/src/lib/config/types/validation.rs - Persisted configuration validation rules and invariants
src-tauri/src/lib/mod.rs - Library root - Tauri setup and plugin registration
src-tauri/src/lib/obs/logging.rs - File-based logger writing to run_latest.txt
src-tauri/src/lib/obs/mod.rs - Observability module exports
src-tauri/src/lib/paths/app_paths.rs - Application path resolution logic
src-tauri/src/lib/paths/mod.rs - Path resolution module exports
src-tauri/src/lib/paths/repo_root_resolver.rs - Path-native repo root resolver for user-selected repo paths
src-tauri/src/lib/paths/wsl_convert.rs - Windows <-> WSL path conversion utilities
```
