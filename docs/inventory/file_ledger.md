# File Ledger

Scope: src-tauri, app, agent, scripts (extensions: .cjs, .css, .d.cts, .d.mts, .d.ts, .html, .js, .mjs, .mts, .py, .rs, .scss, .ts, .tsx)

```text
app/index.html - index module
app/src/app.tsx - Root component with tab state management
app/src/components/layout/three_column.tsx - Three-column layout component (Docs | Code | Zips)
app/src/components/tab_bar.tsx - Tab navigation component
app/src/main.tsx - React entry point - mounts App to DOM
app/src/shared/config.ts - AppConfig Zod schema and types
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
