# File Ledger

Scope: src-tauri, crates, app, agent, scripts (extensions: .cjs, .css, .d.cts, .d.mts, .d.ts, .html, .js, .mjs, .mts, .py, .rs, .scss, .ts, .tsx)

```text
agent/src/agent_runtime.ts - Watcher lifecycle helpers and shutdown logic for the agent runtime
agent/src/bundles/bundle_builder.ts - Orchestrates bundle building process (single timestamped file, no accumulation)
agent/src/bundles/bundle_lister.ts - Find the single bundle file for a preset
agent/src/bundles/bundle_types.ts - Type definitions for bundle building
agent/src/bundles/git_info.ts - Best-effort git info extraction for bundle manifests
agent/src/bundles/ignore_rules.test.ts - Unit tests for bundle ignore rules
agent/src/bundles/ignore_rules.ts - Centralized ignore patterns for bundle building
agent/src/bundles/rust_bundle_cli.ts - Run the Rust im_bundle_cli to scan and build bundle zips with progress parsing
agent/src/commands/client_hello.ts - Handles clientHello command with watcher-safe idempotency
agent/src/dev/staging_probe.ts - Minimal code file for staging detection tests
agent/src/main.ts - Agent entry point - bootstraps WebSocket server and watchers
agent/src/repos/mru_index.ts - MRU (Most Recently Used) index for recent file changes with unique-by-path semantics
agent/src/repos/recent_files_store.ts - Persistence layer for recent files with debounced atomic writes
agent/src/repos/repo_top_level.ts - Scan top-level directories and files in a repo
agent/src/repos/repo_watcher.ts - Chokidar file watcher setup with event emission and ignore patterns
agent/src/server/router.ts - Request dispatch and response building for WebSocket protocol
agent/src/server/ws_server.ts - WebSocket server lifecycle on 0.0.0.0:3141
agent/src/staging/path_bridge.ts - WSL to Windows path conversion for staging files
agent/src/staging/stager.test.ts - Unit tests for staging path validation
agent/src/staging/stager.ts - Atomic file copy with debounced auto-staging
agent/src/util/categorizer.ts - File kind classification (docs/code/other) based on path patterns
agent/src/util/config_fingerprint.ts - Computes stable fingerprints for watcher-relevant config to detect changes
agent/src/util/errors.ts - Error types and helpers for the agent
agent/src/util/logger.ts - Structured logging to console with ISO timestamps
agent/src/util/ring_buffer.ts - Generic circular buffer for recent file changes per repo
app/index.html - index module
app/src/app.tsx - Root component with config-driven tab state management
app/src/components/add_repo_button.tsx - "+" button for adding new repositories via directory picker
app/src/components/bundles/bundle_column.tsx - Main bundles column component
app/src/components/bundles/bundle_list.tsx - Single LATEST bundle row (inline, no header)
app/src/components/bundles/bundle_row.tsx - Individual bundle row with drag support
app/src/components/bundles/bundle_selection_panel.tsx - Selection UI for bundle building (root toggle, dir checkboxes, subdir exclusions)
app/src/components/bundles/preset_selector.tsx - Preset tabs/buttons for bundle building
app/src/components/confirm_modal.tsx - Generic confirmation dialog with portal rendering
app/src/components/drag_error_notice.tsx - Small inline error notice for drag failures
app/src/components/empty_repo_state.tsx - Empty state UI when no repos are configured
app/src/components/file_list_column.tsx - Column wrapper that renders a list of FileRow components
app/src/components/file_row.tsx - Draggable file row with click-to-copy, drag handle, and star toggle
app/src/components/layout/three_column.tsx - Three-column layout component with modular deck panels (Docs | Code | Zips)
app/src/components/options_overlay.tsx - Full-screen transparent overlay with options panel for app settings
app/src/components/options/excludes_section.tsx - Excludes configuration section for the options panel
app/src/components/options/excludes/advanced_group.tsx - Collapsible checkbox group for advanced excludes options
app/src/components/options/excludes/excludes_normalizers.ts - Normalization helpers for global excludes inputs
app/src/components/options/excludes/excludes_recommendations.ts - Helpers for recommended global excludes toggles
app/src/components/options/excludes/excludes_updates.ts - Pure update helpers for global excludes toggles
app/src/components/options/excludes/use_excludes_state.ts - State and handlers for the excludes section UI
app/src/components/options/texture_picker.tsx - Small texture picker popover for tab theme selection
app/src/components/options/theme_section.tsx - Options panel theme controls (texture + accent per tab)
app/src/components/status_bar.tsx - Status bar with connection status LED, error display, and options button
app/src/components/tab_bar.tsx - Tab navigation with grouped repo dropdown support
app/src/components/tab_remove_button.tsx - "x" button for removing repos with confirmation
app/src/hooks/use_agent.tsx - Agent context provider and connection management hook
app/src/hooks/use_bundle_state.ts - Per-repo bundle state management with event subscription
app/src/hooks/use_client_hello.ts - Custom hook for clientHello lifecycle with reconnect support
app/src/hooks/use_config_actions_extended.ts - Extended config actions for theme, starred files, and recent files limit
app/src/hooks/use_config_actions.ts - Core config action factory functions for repo and bundle management
app/src/hooks/use_config_storage.ts - Config persistence + loading hook for use_config
app/src/hooks/use_config.tsx - Config persistence context provider and hook
app/src/hooks/use_drag.ts - Drag-out logic with on-demand staging
app/src/hooks/use_repo_state.ts - Per-repo file state management with event subscription
app/src/hooks/use_starred_files.ts - Hook exposing starred file state and actions for a repo
app/src/hooks/use_worktree_add.ts - Hook for adding worktrees to existing groups or single repos
app/src/lib/agent/agent_client.ts - WebSocket client with reconnection and message correlation
app/src/lib/agent/connection_state.ts - Agent connection status types
app/src/lib/agent/messages.ts - Typed helper functions for sending agent commands
app/src/lib/theme/accent_utils.ts - Convert hex accent color to CSS variable values for runtime theming
app/src/lib/theme/texture_catalog.ts - Build-time texture catalog for theme substrate/dither selection
app/src/main.tsx - React entry point - mounts App with ConfigProvider and AgentProvider
app/src/shared/config.ts - Shared config barrel exports
app/src/shared/config/app_config.ts - AppConfig schema, types, and defaults
app/src/shared/config/bundle_presets.ts - Bundle preset schema, type, and defaults
app/src/shared/config/glob_defaults.ts - Default glob patterns for docs, code, and ignores
app/src/shared/config/persisted_config_migrations.ts - Persisted config migrations and legacy normalization
app/src/shared/config/persisted_config.ts - Persisted config schema, types, and defaults
app/src/shared/config/repo_config.ts - RepoConfig schema and type
app/src/shared/config/version.ts - Persisted config schema version
app/src/shared/global_excludes.ts - Global bundle exclude schema and UI options
app/src/shared/protocol.ts - Agent<->UI WebSocket protocol types with Zod validation
app/src/shared/repo_utils.ts - Utility functions for repo ID generation and path handling
app/src/styles/a11y.css - Accessibility utilities - focus rings, disabled states, screen reader helpers
app/src/styles/badges.css - Bracket-style badge tags for status indicators [A] [M] [D] [STAGED] [LATEST]
app/src/styles/bundle_column.css - Hardware-style bundle column with segmented controls and command buttons
app/src/styles/chrome.css - Unified header chrome styles for tab bar, status bar, and banners
app/src/styles/columns.css - Three-column deck grid layout with intentional gutters (Docs | Code | Zips)
app/src/styles/confirm_modal.css - Confirmation dialog overlay with glass panel styling
app/src/styles/drag_error_notice.css - Inline glass toast for drag errors
app/src/styles/effects.css - Deck chassis frame, substrate (grid + grain), vignette, and glass utilities
app/src/styles/empty_repo_state.css - Empty state display when no repositories are configured
app/src/styles/file_row.css - Hardware-style file row with drag handle and star toggle
app/src/styles/main.css - Global layout reset and base structure
app/src/styles/motion.css - Motion utilities, transition presets, and reduced-motion support
app/src/styles/options_overlay.css - Full-screen transparent overlay with centered glass panel for app options
app/src/styles/panels.css - Modular deck panel surfaces with framed edges and etched headers
app/src/styles/scrollbars.css - Thin dark scrollbar styling with accent hints
app/src/styles/status_bar.css - Status bar with connection LED, error display, and options button
app/src/styles/tab_bar_dropdown.css - Dropdown-specific styles for tab bar worktree actions
app/src/styles/tab_bar.css - Tab bar navigation with ASCII-instrument bracketed labels
app/src/styles/theme_accents.css - Default accent color variables (runtime values applied via inline styles in app.tsx)
app/src/styles/theme_dark.css - Dark glass vintage theme - fills semantic token slots
app/src/styles/tokens.css - Design system tokens - spacing, radii, blur, shadows, typography, motion
app/src/tabs/repo_tab.tsx - Generic repo tab component with 3-column layout
app/src/types/app_paths.ts - TypeScript interface matching Rust AppPaths struct
app/src/vite_env.d.ts - Vite client type declarations
crates/im_bundle/src/bin/im_bundle_cli.rs - CLI entry point for im_bundle - scans and writes bundle zip
crates/im_bundle/src/compression_policy.rs - Compression policy for bundle entries based on extension and size
crates/im_bundle/src/error.rs - Error types for bundle scanning and zip writing
crates/im_bundle/src/global_excludes.rs - Normalize and apply user-configurable global excludes for bundle scanning
crates/im_bundle/src/lib.rs - Library root for bundle scanning and zip creation
crates/im_bundle/src/manifest.rs - Bundle manifest structure and serialization
crates/im_bundle/src/plan.rs - Bundle plan schema and loader for im_bundle_cli
crates/im_bundle/src/progress.rs - Throttled NDJSON progress emitter for bundle scanning and zipping
crates/im_bundle/src/scanner.rs - Bundle scanning logic with ignore rules and exclusions
crates/im_bundle/src/writer.rs - Bundle zip writer with scanning, manifest, and progress
crates/im_bundle/tests/scanner_test.rs - Integration tests for bundle scanner behavior
crates/im_bundle/tests/size_capped_reads_test.rs - Ensures bundle writes only the bytes present at file-open time even if file grows
scripts/fileledger/add_file_headers.mjs - Adds missing header comments (path + description) to source files using the ledger output.
scripts/fileledger/gen_file_ledger.mjs - Generates human+machine file ledgers for Intermediary code sources.
scripts/icons/generate_icons.mjs - Generate all icon sizes from a source PNG. Usage: node scripts/generate_icons.mjs [source.png] Default source: app/as...
scripts/icons/resize_preview_icons.mjs - Resize preview geometry icons from raw assets to display sizes. Outputs 40px (1x) and 80px (2x retina) versions.
scripts/zip/zip_bundles.mjs - Builds timestamped Intermediary zip bundles for ChatGPT context.
src-tauri/build.rs - Tauri build script
src-tauri/src/bin/intermediary.rs - Binary entry point for Tauri app
src-tauri/src/lib/commands/config.rs - Tauri commands for config persistence
src-tauri/src/lib/commands/file_manager.rs - Open folders in OS file manager (Windows Explorer)
src-tauri/src/lib/commands/mod.rs - Tauri command exports
src-tauri/src/lib/commands/paths.rs - get_app_paths command implementation and path conversion utilities
src-tauri/src/lib/commands/wsl.rs - WSL host resolution for Windows->WSL agent connections
src-tauri/src/lib/config/io.rs - Config file I/O with atomic writes and error handling
src-tauri/src/lib/config/mod.rs - Configuration persistence module
src-tauri/src/lib/config/types.rs - Persisted configuration types for Intermediary
src-tauri/src/lib/mod.rs - Library root - Tauri setup and plugin registration
src-tauri/src/lib/obs/logging.rs - File-based logger writing to run_latest.txt
src-tauri/src/lib/obs/mod.rs - Observability module exports
src-tauri/src/lib/paths/app_paths.rs - Application path resolution logic
src-tauri/src/lib/paths/mod.rs - Path resolution module exports
src-tauri/src/lib/paths/wsl_convert.rs - Windows <-> WSL path conversion utilities
```
