// Path: app/src/components/options/agent_section.tsx
// Description: Options panel controls for host + WSL agent lifecycle

import type React from "react";

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
      <div className="options-row">
        <span className="options-row-label">Auto-start agent backend</span>
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
      </div>
      {supportsWsl ? (
        <div
          className="options-row stacked"
          title="Leave blank to use the default WSL distro for agent launch"
        >
          <span className="options-row-label">WSL distro override</span>
          <input
            type="text"
            className="options-text-input"
            value={agentDistro ?? ""}
            placeholder="Ubuntu"
            onChange={(event) => {
              setAgentDistro(event.target.value);
            }}
          />
        </div>
      ) : null}
      <div className="options-row">
        <button
          type="button"
          className="options-button"
          onClick={() => {
            restartAgent();
          }}
        >
          Restart agent
        </button>
      </div>
    </div>
  );
}
