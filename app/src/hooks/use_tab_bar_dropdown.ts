// Path: app/src/hooks/use_tab_bar_dropdown.ts
// Description: Owns tab-bar dropdown open state, trigger containment, and anchored positioning

import type React from "react";
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";

const DROPDOWN_VIEWPORT_GUTTER_PX = 12;

interface UseTabBarDropdownArgs {
  navRef: React.RefObject<HTMLElement | null>;
  trackRef: React.RefObject<HTMLDivElement | null>;
}

export function useTabBarDropdown({
  navRef,
  trackRef,
}: UseTabBarDropdownArgs) {
  const [openDropdownId, setOpenDropdownId] = useState<string | null>(null);
  const [dropdownAnchorStyle, setDropdownAnchorStyle] =
    useState<React.CSSProperties | null>(null);
  const dropdownRef = useRef<HTMLDivElement>(null);
  const triggerRefs = useRef(new Map<string, HTMLButtonElement>());

  const closeDropdown = useCallback(() => {
    setOpenDropdownId(null);
  }, []);

  const registerDropdownTrigger = useCallback(
    (dropdownId: string, node: HTMLButtonElement | null) => {
      if (node) {
        triggerRefs.current.set(dropdownId, node);
        return;
      }
      triggerRefs.current.delete(dropdownId);
    },
    []
  );

  const toggleDropdown = useCallback(
    (event: React.MouseEvent, dropdownId: string) => {
      event.stopPropagation();
      setOpenDropdownId((prev) => (prev === dropdownId ? null : dropdownId));
    },
    []
  );

  useEffect(() => {
    if (!openDropdownId) return;
    const activeDropdownId = openDropdownId;

    function handleClickOutside(event: MouseEvent) {
      const target = event.target as Node | null;
      const activeTrigger = triggerRefs.current.get(activeDropdownId);
      if (target && dropdownRef.current?.contains(target)) {
        return;
      }
      if (target && activeTrigger?.contains(target)) {
        return;
      }
      setOpenDropdownId(null);
    }

    document.addEventListener("mousedown", handleClickOutside);
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
    };
  }, [openDropdownId]);

  useLayoutEffect(() => {
    if (!openDropdownId) {
      setDropdownAnchorStyle(null);
      return;
    }

    const nav = navRef.current;
    const track = trackRef.current;
    const trigger = triggerRefs.current.get(openDropdownId);
    if (!nav || !track || !trigger) {
      return;
    }

    const updatePosition = () => {
      const navRect = nav.getBoundingClientRect();
      const triggerRect = trigger.getBoundingClientRect();
      const dropdownWidth = dropdownRef.current?.offsetWidth ?? 0;
      const desiredRightEdge = triggerRect.right - navRect.left;
      const clampedRightEdge =
        dropdownWidth > 0
          ? Math.min(
              Math.max(desiredRightEdge, dropdownWidth + DROPDOWN_VIEWPORT_GUTTER_PX),
              navRect.width - DROPDOWN_VIEWPORT_GUTTER_PX
            )
          : desiredRightEdge;

      setDropdownAnchorStyle({
        left: `${Math.round(clampedRightEdge)}px`,
        top: `${Math.round(triggerRect.bottom - navRect.top)}px`,
      });
    };

    updatePosition();
    window.addEventListener("resize", updatePosition);
    track.addEventListener("scroll", updatePosition, { passive: true });
    return () => {
      window.removeEventListener("resize", updatePosition);
      track.removeEventListener("scroll", updatePosition);
    };
  }, [navRef, openDropdownId, trackRef]);

  return {
    closeDropdown,
    dropdownAnchorStyle,
    dropdownRef,
    openDropdownId,
    registerDropdownTrigger,
    toggleDropdown,
  };
}
