# File Ledger

Scope: src-tauri, app, agent, scripts (extensions: .cjs, .css, .d.cts, .d.mts, .d.ts, .html, .js, .mjs, .mts, .py, .rs, .scss, .ts, .tsx)

```text
agent/src/main.ts - Agent entry point - bootstraps WebSocket server and watchers
agent/src/repos/repo_top_level.ts - Scan top-level directories and files in a repo
agent/src/repos/repo_watcher.ts - Chokidar file watcher setup with event emission and ignore patterns
agent/src/server/router.ts - Request dispatch and response building for WebSocket protocol
agent/src/server/ws_server.ts - WebSocket server lifecycle on localhost:3141
agent/src/staging/path_bridge.ts - WSL to Windows path conversion for staging files
agent/src/staging/stager.ts - Atomic file copy with debounced auto-staging
agent/src/util/categorizer.ts - File kind classification (docs/code/other) based on path patterns
agent/src/util/errors.ts - Error types and helpers for the agent
agent/src/util/logger.ts - Structured logging to console with ISO timestamps
agent/src/util/ring_buffer.ts - Generic circular buffer for recent file changes per repo
app/index.html - index module
app/src/app.tsx - Root component with tab state management
app/src/components/layout/three_column.tsx - Three-column layout component (Docs | Code | Zips)
app/src/components/tab_bar.tsx - Tab navigation component
app/src/main.tsx - React entry point - mounts App to DOM
app/src/shared/config.ts - AppConfig Zod schema and types
app/src/shared/ids.ts - Shared identifiers for tabs and worktrees
app/src/shared/protocol.ts - Agent<->UI WebSocket protocol types with Zod validation
app/src/styles/columns.css - columns module
app/src/styles/main.css - main module
app/src/styles/tab_bar.css - tab bar module
app/src/tabs/intermediary_tab.tsx - Intermediary project tab
app/src/tabs/texture_portal_tab.tsx - TexturePortal project tab
app/src/tabs/triangle_rain_tab.tsx - Triangle Rain project tab with worktree selector
app/src/vite_env.d.ts - Vite client type declarations
scripts/fileledger/add_file_headers.mjs - Adds missing header comments (path + description) to source files using the ledger output.
scripts/fileledger/gen_file_ledger.mjs - Generates human+machine file ledgers for Intermediary code sources.
scripts/icons/generate_icons.mjs - Generate all icon sizes from a source PNG. Usage: node scripts/generate_icons.mjs [source.png] Default source: app/as...
scripts/icons/resize_preview_icons.mjs - Resize preview geometry icons from raw assets to display sizes. Outputs 40px (1x) and 80px (2x retina) versions.
scripts/zip/zip_bundles.mjs - Builds timestamped Intermediary zip bundles for ChatGPT context.
src-tauri/build.rs - Tauri build script
src-tauri/src/bin/intermediary.rs - Binary entry point for Tauri app
src-tauri/src/lib/commands/mod.rs - Tauri command exports
src-tauri/src/lib/commands/paths.rs - get_app_paths command implementation
src-tauri/src/lib/mod.rs - Library root - Tauri setup and plugin registration
src-tauri/src/lib/obs/logging.rs - File-based logger writing to run_latest.txt
src-tauri/src/lib/obs/mod.rs - Observability module exports
src-tauri/src/lib/paths/app_paths.rs - Application path resolution logic
src-tauri/src/lib/paths/mod.rs - Path resolution module exports
src-tauri/src/lib/paths/wsl_convert.rs - Windows <-> WSL path conversion utilities
```
