// Path: app/src/hooks/source_control/use_tree_decorations.tsx
// Description: Context delivering the built tree decorations to the recursive bundle explorer rows

import type React from "react";
import { createContext, useContext, useMemo, type ReactNode } from "react";
import type { SourceControlStatus } from "../../shared/protocol.js";
import {
  buildTreeDecorations,
  EMPTY_TREE_DECORATIONS,
  type DirectoryDecoration,
  type FileDecoration,
  type TreeDecorations,
} from "../../lib/source_control/tree_decorations.js";

/** Decorations are adornment: rows stay mountable outside the provider, so the default is empty. */
const TreeDecorationsContext = createContext<TreeDecorations>(EMPTY_TREE_DECORATIONS);

interface TreeDecorationsProviderProps {
  status: SourceControlStatus | null;
  children: ReactNode;
}

export function TreeDecorationsProvider({
  status,
  children,
}: TreeDecorationsProviderProps): React.JSX.Element {
  const decorations = useMemo(() => buildTreeDecorations(status), [status]);
  return (
    <TreeDecorationsContext.Provider value={decorations}>
      {children}
    </TreeDecorationsContext.Provider>
  );
}

export function useFileDecoration(path: string): FileDecoration | null {
  return useContext(TreeDecorationsContext).files.get(path) ?? null;
}

export function useDirectoryDecoration(path: string): DirectoryDecoration | null {
  return useContext(TreeDecorationsContext).directories.get(path) ?? null;
}
