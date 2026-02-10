// Path: app/src/components/options/agent_section.tsx
// Description: Options panel controls for host + WSL agent lifecycle

import type React from "react";
import { OptionsFieldRow } from "./layout/options_field_row.js";

interface AgentSectionProps {
  agentAutoStart: boolean;
  setAgentAutoStart: (value: boolean) => void;
  supportsWsl: boolean;
  agentDistro: string | null;
  setAgentDistro: (value: string | null) => void;
  restartAgent: () => void;
}

export function AgentSection({
  agentAutoStart,
  setAgentAutoStart,
  supportsWsl,
  agentDistro,
  setAgentDistro,
  restartAgent,
}: AgentSectionProps): React.JSX.Element {
  return (
    <div className="options-section">
      <div className="options-section-title">Agent</div>
      <OptionsFieldRow
        label="Auto-start agent backend"
        title="Automatically launch the agent backend when the app starts"
        controlAlign="start"
        control={(
          <label className="vintage-toggle">
            <input
              type="checkbox"
              checked={agentAutoStart}
              onChange={(event) => {
                setAgentAutoStart(event.target.checked);
              }}
            />
            <span className="vintage-toggle-track" aria-hidden="true" />
          </label>
        )}
      />
      {supportsWsl ? (
        <OptionsFieldRow
          label="WSL distro override"
          title="Leave blank to use the default WSL distro for agent launch"
          controlAlign="stretch"
          control={(
            <input
              type="text"
              className="options-text-input"
              value={agentDistro ?? ""}
              placeholder="Ubuntu"
              onChange={(event) => {
                setAgentDistro(event.target.value);
              }}
            />
          )}
        />
      ) : null}
      <OptionsFieldRow
        label="Agent process"
        title="Stop and relaunch the agent backend process"
        controlAlign="stretch"
        control={(
          <button
            type="button"
            className="options-button"
            onClick={() => {
              restartAgent();
            }}
          >
            Restart agent
          </button>
        )}
      />
    </div>
  );
}
