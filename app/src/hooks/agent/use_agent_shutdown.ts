// Path: app/src/hooks/agent/use_agent_shutdown.ts
// Description: Stop the WSL agent when the app window is closing

import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

export function useAgentShutdown(): void {
  useEffect(() => {
    const handleBeforeUnload = (): void => {
      void invoke("stop_agent").catch((err: unknown) => {
        console.error("[AgentProvider] stop_agent failed:", err);
      });
    };

    window.addEventListener("beforeunload", handleBeforeUnload);
    return () => {
      window.removeEventListener("beforeunload", handleBeforeUnload);
    };
  }, []);
}
