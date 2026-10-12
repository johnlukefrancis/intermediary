// Path: app/src/components/tab_bar/project_launch_context.tsx
// Description: Tab-bar project-launch provider: launch state for every tab plus the target menu and error notice overlays

import type React from "react";
import { createContext, useCallback, useContext, useEffect, useState } from "react";
import { useConfig } from "../../hooks/use_config.js";
import { useProjectLaunch, type ProjectLaunch } from "../../hooks/use_project_launch.js";
import { useTabBarDropdown } from "../../hooks/use_tab_bar_dropdown.js";
import { DEFAULT_ACCENT_HEX, hexToAccentCssVars } from "../../lib/theme/accent_utils.js";
import { LaunchMenu } from "./launch_menu.js";

interface ProjectLaunchContextValue extends ProjectLaunch {
  isMenuOpen: (repoId: string) => boolean;
  toggleMenu: (event: React.MouseEvent, repoId: string) => void;
  registerMenuTrigger: (repoId: string, node: HTMLButtonElement | null) => void;
  registerLaunchTrigger: (sourceId: string, node: HTMLButtonElement | null) => void;
}

const ProjectLaunchContext = createContext<ProjectLaunchContextValue | null>(null);

interface ProjectLaunchProviderProps {
  navRef: React.RefObject<HTMLElement | null>;
  trackRef: React.RefObject<HTMLDivElement | null>;
  children: React.ReactNode;
}

export function ProjectLaunchProvider({
  navRef,
  trackRef,
  children,
}: ProjectLaunchProviderProps): React.JSX.Element {
  const { config, setLaunchTarget } = useConfig();
  const menu = useTabBarDropdown({ navRef, trackRef });
  const notice = useTabBarDropdown({ navRef, trackRef });
  const [noticeRepoId, setNoticeRepoId] = useState<string | null>(null);
  const { openDropdown: openNotice } = notice;
  const handleError = useCallback((repoId: string, sourceId: string) => {
    setNoticeRepoId(repoId);
    openNotice(sourceId);
  }, [openNotice]);
  const launch = useProjectLaunch({
    repos: config.repos,
    distroOverride: config.agentDistro,
    launchTargets: config.launchTargets,
    setLaunchTarget,
    onError: handleError,
  });

  const noticeFeedback = noticeRepoId ? launch.feedbackFor(noticeRepoId) : null;
  const noticeMessage =
    noticeFeedback?.phase === "error" && noticeFeedback.sourceId === notice.openDropdownId
      ? noticeFeedback.message
      : null;
  const { openDropdownId: openNoticeId, closeDropdown: closeNotice } = notice;
  useEffect(() => {
    if (openNoticeId !== null && noticeMessage === null) closeNotice();
  }, [openNoticeId, noticeMessage, closeNotice]);

  const { refresh } = launch;
  const { openDropdownId: menuRepoId, toggleDropdown: toggleMenuDropdown } = menu;
  const toggleMenu = useCallback((event: React.MouseEvent, repoId: string) => {
    if (menuRepoId !== repoId) refresh();
    toggleMenuDropdown(event, repoId);
  }, [menuRepoId, refresh, toggleMenuDropdown]);

  const value: ProjectLaunchContextValue = {
    ...launch,
    isMenuOpen: (repoId) => menuRepoId === repoId,
    toggleMenu,
    registerMenuTrigger: menu.registerDropdownTrigger,
    registerLaunchTrigger: notice.registerDropdownTrigger,
  };

  const accentFor = (repoId: string): React.CSSProperties => {
    const repo = config.repos.find((r) => r.repoId === repoId);
    const themeKey = repo?.groupId ?? repoId;
    const accentHex = config.tabThemes[themeKey]?.accentHex ?? DEFAULT_ACCENT_HEX;
    return hexToAccentCssVars(accentHex) as React.CSSProperties;
  };
  const menuTarget = menuRepoId ? launch.targetFor(menuRepoId) : null;

  return (
    <ProjectLaunchContext.Provider value={value}>
      {children}
      {menuRepoId && menuTarget && (
        <div className="tab-bar-dropdown-anchor" ref={menu.dropdownRef}
          style={menu.dropdownAnchorStyle ?? undefined}>
          <LaunchMenu
            candidates={launch.candidatesFor(menuRepoId)}
            target={menuTarget}
            accentStyle={accentFor(menuRepoId)}
            onChoose={(relativePath) => {
              launch.choose(menuRepoId, `tab:${menuRepoId}`, relativePath);
              menu.closeDropdown();
            }}
          />
        </div>
      )}
      {noticeRepoId && noticeMessage && notice.dropdownAnchorStyle && (
        <div className="tab-bar-dropdown-anchor" ref={notice.dropdownRef}
          style={notice.dropdownAnchorStyle}>
          <div className="launch-notice" role="alert" style={accentFor(noticeRepoId)}>
            {noticeMessage}
          </div>
        </div>
      )}
    </ProjectLaunchContext.Provider>
  );
}

export function useProjectLaunchContext(): ProjectLaunchContextValue {
  const context = useContext(ProjectLaunchContext);
  if (!context) {
    throw new Error("useProjectLaunchContext must be used within ProjectLaunchProvider");
  }
  return context;
}
