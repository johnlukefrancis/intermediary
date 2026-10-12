// Path: app/src/crosstalk/identity.ts
// Description: Agent identity and configured repository accents for delivered messages

import { DEFAULT_ACCENT_HEX } from "../lib/theme/accent_utils.js";
import { repoRootFromInputPath, repoRootKey } from "../shared/config/repo_root.js";

export interface Message {
  id: string;
  from: string;
  to: string;
  text: string;
  sent: number;
  cwd: string | null;
}

export interface RepoStyle { path: string; label: string; accent: string }
export interface Presentation { active: boolean; theme: string; repos: RepoStyle[] }

export function agent(address: string): { client: string; folder: string; tag: string } {
  const match = /^([^@]+)@(.+)#([^#]+)$/.exec(address);
  return { client: match?.[1] ?? "agent", folder: match?.[2] ?? address, tag: match?.[3] ?? "" };
}

function key(path: string): string {
  const root = repoRootFromInputPath(path);
  return root ? repoRootKey(root).replace(/\\/g, "/").replace(/\/$/, "") : path;
}

export function repoStyle(message: Message, repos: readonly RepoStyle[]): RepoStyle {
  const sender = agent(message.from);
  const cwd = message.cwd ? key(message.cwd) : null;
  const matches = cwd ? repos.filter(repo => {
    const path = key(repo.path);
    return cwd === path || cwd.startsWith(`${path}/`);
  }).sort((a, b) => b.path.length - a.path.length) : [];
  if (matches[0]) return matches[0];
  const folders = repos.filter(repo => repo.path.replace(/[\\/]+$/, "").split(/[\\/]/).at(-1)
    ?.toLowerCase().replace(/[^a-z0-9]/g, "-").replace(/^-+|-+$/g, "") === sender.folder);
  // A unique folder also identifies the same configured repo's Windows/WSL twin.
  if (folders.length === 1 && folders[0]) return folders[0];
  return { path: "", label: sender.folder, accent: DEFAULT_ACCENT_HEX };
}
