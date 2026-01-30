# WSL Node.js + pnpm Setup
Updated on: 2026-01-30
Owners: JL · Agents
Depends on: ADR-000, ADR-012

Install Node.js and pnpm inside WSL so the agent can run there (not via Windows binaries).

## Option A (recommended): nvm + Node 20 + corepack

```bash
# Install nvm
curl -fsSL https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.7/install.sh | bash

# Load nvm in this shell
export NVM_DIR="$HOME/.nvm"
[ -s "$NVM_DIR/nvm.sh" ] && . "$NVM_DIR/nvm.sh"

# Install Node 20 and enable pnpm via corepack
nvm install 20
nvm use 20
corepack enable
corepack prepare pnpm@latest --activate

# Verify
node -v
pnpm -v
```

## Option B (system packages)

```bash
sudo apt update
sudo apt install -y nodejs npm

# Ensure pnpm is available via corepack (Node 18+ recommended)
corepack enable
corepack prepare pnpm@latest --activate

# Verify
node -v
pnpm -v
```

## Notes

- Ensure the WSL shell you use for tasks has access to the same Node installation.
- If you already installed Node in WSL, confirm `node -v` and `pnpm -v` work before running the agent.
