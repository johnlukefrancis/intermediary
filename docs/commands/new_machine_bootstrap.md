# New Machine Bootstrap
Updated on: 2026-09-05
Owners: JL · Agents
Depends on: ADR-006, ADR-012

Steps to bring a fresh Windows + WSL laptop up to the same agent config as the
main PC. Run each block in the shell named in its heading. Steps are ordered by
dependency; the global-config clone needs GitHub auth first.

## 0. Running WSL steps from a PowerShell prompt

The `!` prefix in Claude Code runs in PowerShell, where `sudo` is Windows'
disabled sudo, not Ubuntu's. Wrap each WSL block like this so it runs inside
the distro with a real TTY for the password prompt.

```powershell
wsl -e bash -lc "sudo apt update && sudo apt install -y gh build-essential pkg-config libssl-dev curl unzip"
wsl -e bash -lc "curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.3/install.sh | bash"
wsl -e bash -lic "nvm install --lts && corepack enable && corepack prepare pnpm@latest --activate"
wsl -e bash -lc "gh auth login --hostname github.com --git-protocol https --web && gh auth setup-git"
wsl -e bash -lc "gh repo list --limit 200"
```

If the sudo password prompt fails to appear through `!`, open Windows Terminal,
pick the Ubuntu profile, and paste the bash blocks from steps 1 to 3 directly.

## 1. WSL: install core tooling

Installs GitHub CLI and build tools via apt, then Node via nvm. The WSL
addendum rules nvm as the only Node authority in WSL; never install apt or
nodesource Node. The second block needs a fresh interactive shell so nvm is
loaded.

```bash
sudo apt update && sudo apt install -y gh build-essential pkg-config libssl-dev curl unzip
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.3/install.sh | bash
```

```bash
nvm install --lts && corepack enable && corepack prepare pnpm@latest --activate
```

## 2. WSL: authenticate GitHub

Browser-based login. Use `HTTPS` and let `gh` install the git credential helper
so `git clone` works for private repos.

```bash
gh auth login --hostname github.com --git-protocol https --web
gh auth setup-git
```

## 3. WSL: find the global-config repo

Lists every repo on the account, including private ones.

```bash
gh repo list --limit 200
```

## 4. Windows (PowerShell): install Git, GitHub CLI, Gitleaks, Rust

The global config repo is `johnlukefrancis/global-agent-config`. Its worktree
is the Windows home and its git metadata lives in `~\.agent-config\.git`, so
Windows git is the authority, not WSL. Gitleaks is required by the pre-push
hook. Enable Windows Developer Mode first (Settings > System > For developers)
so unprivileged symlinks work; the projection depends on them.

```powershell
winget install --id Git.Git -e
winget install --id GitHub.cli -e
winget install --id Gitleaks.Gitleaks -e
winget install --id Python.Python.3.13 -e
winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override "--wait --passive --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
winget install --id Rustlang.Rustup -e
git config --global user.name "John Luke"
git config --global user.email "johnfultonfrank@gmail.com"
```

The Build Tools line is multi-GB and asks for UAC; rustup needs it for the
MSVC linker. Set the same git identity inside WSL. Open a fresh terminal
after this so PATH picks up the new tools.

## 5. Windows (PowerShell): authenticate and clone bare metadata

Reuses the WSL gh token so you only log in once. Back up any existing
`~\.claude\settings.json` first; the checkout overwrites it.

```powershell
wsl -e bash -lc "gh auth token" | gh auth login --hostname github.com --with-token
gh auth setup-git
Copy-Item "$HOME\.claude\settings.json" "$HOME\.claude\backups\settings.json.pre_agent_config"
git clone --bare https://github.com/johnlukefrancis/global-agent-config.git "$HOME\.agent-config\.git"
```

## 6. Windows (PowerShell): configure and check out tracked paths only

Never run a broad checkout over the home directory. These four paths are the
entire tracked scope.

```powershell
$gd = "$HOME\.agent-config\.git"
git --git-dir=$gd config core.bare false
git --git-dir=$gd config core.worktree C:/Users/Johnf
git --git-dir=$gd config core.excludesFile C:/Users/Johnf/.agent-config/scope.gitignore
git --git-dir=$gd config core.autocrlf false
git --git-dir=$gd config core.safecrlf true
git --git-dir=$gd config core.hooksPath C:/Users/Johnf/.agent-config/repo-hooks
git --git-dir=$gd config remote.origin.fetch "+refs/heads/*:refs/remotes/origin/*"
git --git-dir=$gd fetch origin
git --git-dir=$gd branch --set-upstream-to=origin/main main
Set-Location $HOME
git --git-dir=$gd --work-tree=$HOME checkout HEAD -- .agent-config .codex .claude .local
[Environment]::SetEnvironmentVariable('Path', "$([Environment]::GetEnvironmentVariable('Path','User'));%USERPROFILE%\.local\bin", 'User')
```

## 7. Windows (PowerShell): apply the Claude MCP manifest

Registers the tracked, secret-free MCP servers into `~\.claude.json`. Backs up
the runtime state first.

```powershell
claude-mcp-config apply
```

## 8. WSL: bootstrap and run the projection

`jl-agent-sync` copies the Windows authority into WSL, installs the shell
aliases, and creates the shared Claude skill links on Windows. External skill
links whose target is absent on this machine (currently
`C:\Code\SpriteAuthoring`) are skipped and reported by `--check`; that is the
expected state on the laptop.

```bash
mkdir -p ~/bin
cp /mnt/c/Users/Johnf/.agent-config/wsl/bin/jl-agent-sync /mnt/c/Users/Johnf/.agent-config/wsl/bin/agent-git ~/bin/
sed -i 's/\r$//' ~/bin/jl-agent-sync ~/bin/agent-git
chmod +x ~/bin/jl-agent-sync ~/bin/agent-git
~/bin/jl-agent-sync && ~/bin/jl-agent-sync --check
```

## 9. WSL: Rust, browser shim, and project clones

Rust in WSL is native rustup, not the Windows wrapper the main PC uses. The
shim lets `gh` and similar tools open links in the Windows browser. Projects
live under `~/dev` on ext4; TriangleRain and Unreal stay on the main PC.

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
mkdir -p ~/.local/bin
printf '#!/usr/bin/env bash\nexec /mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe -NoProfile -Command "Start-Process \\"$1\\""\n' > ~/.local/bin/xdg-open
chmod +x ~/.local/bin/xdg-open && ln -sfn ~/.local/bin/xdg-open ~/.local/bin/wslview
printf '\nexport PATH="$HOME/.local/bin:$PATH"\nexport BROWSER="$HOME/.local/bin/xdg-open"\n' >> ~/.bashrc
git clone https://github.com/johnlukefrancis/intermediary.git ~/dev/intermediary
git clone https://github.com/johnlukefrancis/GlitchFish.git ~/dev/GlitchFish
```

GlitchFish then needs `npm install` and `npx playwright install chromium`
as the user, plus `npx playwright install-deps chromium` as root. Its
`npm run shot` harness does not finish under WSL software WebGL; see the
GlitchFish handoff notes.

## Verify

```powershell
agent-git status
```

```bash
ls -la ~/.claude ~/.codex && gh auth status && jl-agent-sync --check
```
