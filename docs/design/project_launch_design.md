# Project Launch Design
Updated on: 2026-10-12
Owners: JL · Agents
Depends on: ADR-000, ADR-005, ADR-007, ADR-008, ADR-009, ADR-010

---

## Problem

JL moves between repositories and worktrees in Intermediary but still launches each
project's application by hand: the right worktree's `.uproject` in the right Unreal
engine for TriangleRain and Blorb, the main HTML page for Coilbound. The app should
know each project's launch target without setup and start it with one click.

## Goals

- A `▶` on every repository tab whose root has a launch target, launching that exact
  worktree's target without switching tabs.
- Zero setup: targets come from the root's top-level files, ranked by what JL has
  actually opened.
- One-click correction: a chevron lists the other targets; choosing one launches it
  and makes it the tab's persisted default.
- An Unreal project that is already open in an editor is focused, never opened twice.

## Non-goals

- Target kinds beyond Unreal projects and HTML pages (Blender, dev servers, scripts,
  editors). The candidate model admits new kinds without UI change.
- Searching below the repository's top level.
- Focusing a particular browser tab: an HTML launch always opens the page again.
- A settings surface for launch targets; the chevron choice is the only override.
- Program run counts (UserAssist): they record executables, not files, so they cannot
  separate one root's candidates. JL allowed the signal; v1 has no decision it changes.

## MVP

Candidate discovery (top-level directory listing of the root's host-visible path):

| Kind | Rule |
| --- | --- |
| Unreal | Each top-level `*.uproject`. |
| HTML | Each top-level `*.html` / `*.htm`, unless a top-level `package.json` marks them as build-tool entry pages (Vite/Tauri `index.html`). |

Ranking, highest first:

1. The tab's persisted choice (`launchTargets[repoId]`), while that file still exists.
2. Most recent usage evidence for the exact file: Unreal's own recent-projects list
   (`EditorSettings.ini` `RecentlyOpenedProjectFiles` for every installed engine
   version) and Windows Recent items (`.lnk` shortcuts whose target is the file).
3. Without evidence: Unreal before HTML, then a file whose stem equals the root's folder
   name, then name order.

Launch:

- Unreal: read the project's `EngineAssociation`; resolve the engine directory from
  `HKCU\Software\Epic Games\Unreal Engine\Builds` (source and custom builds),
  `HKLM\SOFTWARE\EpicGames\Unreal Engine\<version>`, or the Epic launcher's
  `LauncherInstalled.dat`; start `Engine\Binaries\Win64\UnrealEditor.exe "<project>"`.
  When an `UnrealEditor.exe` whose command line names this project is running, its
  main window is restored and brought forward instead.
- HTML: the file opens with Windows' default handler for `.html` (the default
  browser), through `\\wsl.localhost\…` for WSL roots.

All discovery, evidence reading and launching run in the Tauri host process off the UI
thread; no agent protocol, port, plugin or capability is added (ADR-010).

## Behaviour table

| Situation / input | Expected visible behaviour |
| --- | --- |
| Tab whose root has no candidate (blender-authoring, intermediary) | No `▶`; the tab is unchanged. |
| Tab with one candidate | `▶` on the tab; tooltip `Launch <file>`. No chevron. |
| Tab with several candidates | `▶` launches the top-ranked one; a chevron beside it opens a menu listing every candidate (kind and file name), the current default marked. |
| Candidate chosen from the chevron | It launches and becomes the tab's default, persisted across restarts. |
| `▶` on a worktree tab (TriangleRain-code) | That worktree's `.uproject` opens in the engine its `EngineAssociation` names, even when another worktree's editor is open. |
| `▶` on an Unreal project already open | The existing editor window is restored if minimised and brought to the front; no second editor starts. |
| `▶` on an HTML target | The page opens in the default browser. |
| Engine association unresolvable or the editor binary missing | The click starts nothing; the `▶` shows the error, naming the association. |
| Persisted default deleted or renamed | Ranking falls back to evidence; the stale choice is ignored. |
| Repository list changes, the window regains focus, or a chevron menu opens | Candidates refresh for every tab in one call; a launch re-checks that its file still exists before starting. |
| Group tab (TriangleRain main / code / lab) | The tab's `▶` launches the worktree the tab currently shows; each worktree row in the group dropdown has its own `▶`. |
| `▶` while that project's editor is still loading (process running, no window yet) | Nothing new starts. |
| Non-Windows host | Unreal targets are listed but launch reports that Unreal launch is Windows-only; HTML opens with the platform opener. |

## Accepted boundaries

- An editor opened without the project path on its command line (for example from the
  Unreal project browser) is not recognised as open, so `▶` starts a second editor.
- Recent-items evidence uses the shortcut's modification time, so a page opened outside
  Explorer's shell (typed into the browser) leaves no evidence.

## Acceptance

- In the installed Windows app, TriangleRain, TriangleRain-code, TriangleRain-lab,
  Blorb, blorb-code and blorb-lab each show `▶`, and each opens its own worktree's
  project in the correct engine (`UE_5.8_Blorb` for Blorb roots).
- A second `▶` on an open project focuses its editor.
- Coilbound's `▶` opens `Coilbound.html`; its chevron also lists
  `Coilbound-opening-film.html`, and choosing that one survives a restart.
- blender-authoring tabs and intermediary show no `▶`.
- JL owns acceptance of placement, look and feel.
