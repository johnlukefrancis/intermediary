// Path: app/src/components/tab_bar.tsx
// Description: Tab navigation component with Triangle Rain worktree dropdown

import type React from "react";
import { useState, useCallback, useEffect, useRef } from "react";
import type { TabId, WorktreeId } from "../shared/protocol";
import { useConfig } from "../hooks/use_config.js";
import "../styles/tab_bar.css";

interface TabBarProps {
  activeTab: TabId;
  onTabChange: (tab: TabId) => void;
}

const TABS: { id: TabId; label: string }[] = [
  { id: "texture-portal", label: "TexturePortal" },
  { id: "intermediary", label: "Intermediary" },
  { id: "triangle-rain", label: "Triangle Rain" },
];

const WORKTREES: { id: WorktreeId; label: string }[] = [
  { id: "tr-engine", label: "tr-engine" },
];

export function TabBar({ activeTab, onTabChange }: TabBarProps): React.JSX.Element {
  const { config, setLastTriangleRainWorktreeId } = useConfig();
  const [dropdownOpen, setDropdownOpen] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  const selectedWorktree = config.uiState.lastTriangleRainWorktreeId ?? "tr-engine";
  const isTriangleRainActive = activeTab === "triangle-rain";

  // Close dropdown on outside click
  useEffect(() => {
    if (!dropdownOpen) return;

    function handleClickOutside(e: MouseEvent) {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        setDropdownOpen(false);
      }
    }

    document.addEventListener("mousedown", handleClickOutside);
    return () => { document.removeEventListener("mousedown", handleClickOutside); };
  }, [dropdownOpen]);

  // Close dropdown when switching away from TR tab
  useEffect(() => {
    if (!isTriangleRainActive) {
      setDropdownOpen(false);
    }
  }, [isTriangleRainActive]);

  const handleWorktreeSelect = useCallback(
    (worktreeId: WorktreeId) => {
      setLastTriangleRainWorktreeId(worktreeId);
      setDropdownOpen(false);
    },
    [setLastTriangleRainWorktreeId]
  );

  const toggleDropdown = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
    setDropdownOpen((prev) => !prev);
  }, []);

  return (
    <nav className="tab-bar">
      {TABS.map((tab) => (
        <button
          key={tab.id}
          className={`tab-button ${activeTab === tab.id ? "active" : ""}`}
          onClick={() => {
            onTabChange(tab.id);
          }}
          type="button"
          aria-current={activeTab === tab.id ? "page" : undefined}
        >
          <span className="tab-label">{tab.label}</span>
        </button>
      ))}

      {isTriangleRainActive && (
        <div className="worktree-dropdown-container" ref={dropdownRef}>
          <button
            className="worktree-chevron"
            onClick={toggleDropdown}
            type="button"
            aria-expanded={dropdownOpen}
            aria-label="Select worktree"
          >
            {dropdownOpen ? "▲" : "▼"}
          </button>

          {dropdownOpen && (
            <div className="worktree-dropdown">
              {WORKTREES.map((wt) => {
                // eslint-disable-next-line @typescript-eslint/no-unnecessary-condition -- WorktreeId will expand
                const isSelected = selectedWorktree === wt.id;
                return (
                  <button
                    key={wt.id}
                    className={`worktree-dropdown-item ${isSelected ? "selected" : ""}`}
                    onClick={() => { handleWorktreeSelect(wt.id); }}
                    type="button"
                  >
                    <span className="worktree-radio">{isSelected ? "●" : "○"}</span>
                    {wt.label}
                  </button>
                );
              })}
            </div>
          )}
        </div>
      )}
    </nav>
  );
}
