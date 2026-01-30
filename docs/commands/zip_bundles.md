# Zip Bundles Commands
Updated on: 2026-01-30
Owners: JL · Agents
Depends on: ADR-000, ADR-012

Commands for creating Intermediary context bundles for ChatGPT.

## VS Code Tasks

### Zip: bundles

Builds all three bundle types (Full, App, Docs).

Task name: `Zip: bundles`

### Zip: docs only

Builds only the Docs bundle.

Task name: `Zip: docs only`

## Command Line Usage

All bundles:

```bash
node scripts/zip/zip_bundles.mjs
```

Specific bundles:

```bash
# Full codebase bundle only
node scripts/zip/zip_bundles.mjs --full

# App code bundle only (app/ + src-tauri/)
node scripts/zip/zip_bundles.mjs --app

# Documentation bundle only (docs/)
node scripts/zip/zip_bundles.mjs --docs
```

Latest alias only (removes timestamped versions):

```bash
node scripts/zip/zip_bundles.mjs --latest-only
```

## Output

Bundles are written to `scripts/zip/output/`:

| Bundle | Contents | Typical Size |
|--------|----------|--------------|
| `Intermediary_Full_*.zip` | Complete codebase | ~500KB |
| `Intermediary_App_*.zip` | Frontend + Tauri only | ~200KB |
| `Intermediary_Docs_*.zip` | Documentation only | ~100KB |

Each bundle includes:
- `_MANIFEST.md` with file list and git SHA
- Timestamped filename: `Intermediary_{type}_{timestamp}_{gitsha}.zip`
- Latest alias: `Intermediary_{type}_latest.zip`

## Bundle Contents

### Full Bundle

- `app/` - Frontend source
- `src-tauri/` - Tauri backend
- `agent/` - WSL agent
- `crates/` - Rust crates
- `docs/` - Documentation
- `scripts/` - Build scripts
- `.vscode/` - VS Code config
- Root configs (package.json, Cargo.toml, etc.)

### App Bundle

- `app/` - Frontend source
- `src-tauri/` - Tauri backend
- Essential root configs

### Docs Bundle

- `docs/` - All documentation
- `scripts/` - Scripts (for reference)
- `.vscode/` - VS Code config

## Excluded from All Bundles

- `node_modules/`
- `target/`
- `dist/`
- `.git/`
- `logs/`
- `scripts/zip/output/`
- `src-tauri/icons/`
- `*.log`, `*.pyc`

## Typical Usage

Before sharing context with ChatGPT:

```bash
# Build all bundles
node scripts/zip/zip_bundles.mjs

# Or just docs for a quick update
node scripts/zip/zip_bundles.mjs --docs
```

Then drag `Intermediary_*_latest.zip` into the ChatGPT chat.
