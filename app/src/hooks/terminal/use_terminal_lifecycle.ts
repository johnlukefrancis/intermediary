// Path: app/src/hooks/terminal/use_terminal_lifecycle.ts
// Description: App-level terminal lifecycle: closes groups of removed repos, mirrors window foreground to cursor blink, closes every session on unload

import { useEffect } from "react";
import { terminalRegistry } from "../../lib/terminal/terminal_registry.js";
import type { RepoRoot } from "../../shared/config.js";

export function useTerminalLifecycle(
  configuredRepoRoots: ReadonlyMap<string, RepoRoot>,
  foreground: boolean
): void {
  useEffect(() => {
    terminalRegistry.retainRepos(configuredRepoRoots);
  }, [configuredRepoRoots]);

  useEffect(() => {
    terminalRegistry.setForeground(foreground);
  }, [foreground]);

  useEffect(() => {
    const onBeforeUnload = (): void => {
      terminalRegistry.closeAll();
    };
    window.addEventListener("beforeunload", onBeforeUnload);
    return () => {
      window.removeEventListener("beforeunload", onBeforeUnload);
    };
  }, []);
}
