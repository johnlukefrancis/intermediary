// Path: app/src/components/bundles/bundle_subdir_tree.tsx
// Description: Nested subdirectory checkbox rows for bundle selection

import React, { useCallback, useMemo, useState } from "react";

interface BundleSubdirTreeProps {
  parentDir: string;
  subdirs: string[];
  isParentSelected: boolean;
  excludedSubdirs: Set<string>;
  onSubdirToggle: (subdirPath: string) => void;
}

interface SubdirTreeNode {
  name: string;
  relativePath: string;
  children: SubdirTreeNode[];
}

interface MutableSubdirTreeNode {
  name: string;
  relativePath: string;
  childrenByName: Map<string, MutableSubdirTreeNode>;
}

function sanitizedCheckboxId(path: string): string {
  return `subdir-checkbox-${path.replace(/[^a-zA-Z0-9]/g, "-")}`;
}

function createMutableNode(name: string, relativePath: string): MutableSubdirTreeNode {
  return {
    name,
    relativePath,
    childrenByName: new Map(),
  };
}

function freezeMutableNode(node: MutableSubdirTreeNode): SubdirTreeNode {
  return {
    name: node.name,
    relativePath: node.relativePath,
    children: [...node.childrenByName.values()].map(freezeMutableNode),
  };
}

function buildSubdirTree(subdirs: string[]): SubdirTreeNode[] {
  const rootsByName = new Map<string, MutableSubdirTreeNode>();

  for (const subdir of subdirs) {
    const parts = subdir.split("/").filter(Boolean);
    let siblings = rootsByName;
    let relativePath = "";

    for (const part of parts) {
      relativePath = relativePath ? `${relativePath}/${part}` : part;
      let node = siblings.get(part);
      if (!node) {
        node = createMutableNode(part, relativePath);
        siblings.set(part, node);
      }
      siblings = node.childrenByName;
    }
  }

  return [...rootsByName.values()].map(freezeMutableNode);
}

function depthClass(depth: number): string {
  return `subdir-row--depth-${Math.min(depth, 3)}`;
}

function hasExcludedAncestor(path: string, excludedSubdirs: Set<string>): boolean {
  const parts = path.split("/").filter(Boolean);
  for (let index = 1; index < parts.length; index += 1) {
    const ancestor = parts.slice(0, index).join("/");
    if (excludedSubdirs.has(ancestor)) {
      return true;
    }
  }
  return false;
}

interface BundleSubdirNodeProps {
  node: SubdirTreeNode;
  depth: number;
  parentDir: string;
  isParentSelected: boolean;
  excludedSubdirs: Set<string>;
  expandedSubdirs: Set<string>;
  onExpandToggle: (subdirPath: string) => void;
  onSubdirToggle: (subdirPath: string) => void;
}

function BundleSubdirNode({
  node,
  depth,
  parentDir,
  isParentSelected,
  excludedSubdirs,
  expandedSubdirs,
  onExpandToggle,
  onSubdirToggle,
}: BundleSubdirNodeProps): React.JSX.Element {
  const subdirPath = `${parentDir}/${node.relativePath}`;
  const subdirCheckboxId = sanitizedCheckboxId(subdirPath);
  const isExcluded = excludedSubdirs.has(subdirPath);
  const isInheritedExcluded = hasExcludedAncestor(subdirPath, excludedSubdirs);
  const isEnabled = isParentSelected && !isInheritedExcluded;
  const isIncluded = isEnabled && !isExcluded;
  const canExpand = node.children.length > 0;
  const isExpanded = expandedSubdirs.has(subdirPath);

  return (
    <>
      <div
        className={`subdir-row ${depthClass(depth)}${!isEnabled ? " subdir-row--disabled" : ""}`}
      >
        {canExpand ? (
          <button
            className="dir-expand-btn"
            type="button"
            onClick={() => { onExpandToggle(subdirPath); }}
            aria-label={isExpanded ? "Collapse" : "Expand"}
            aria-expanded={isExpanded}
          >
            {isExpanded ? "▼" : "▶"}
          </button>
        ) : (
          <span className="dir-expand-spacer" />
        )}
        <label className="vintage-toggle">
          <input
            id={subdirCheckboxId}
            type="checkbox"
            checked={isIncluded}
            disabled={!isEnabled}
            onChange={() => { onSubdirToggle(subdirPath); }}
          />
          <span className="vintage-toggle-track" />
        </label>
        <label
          className={`subdir-label${!isEnabled ? " disabled" : ""}`}
          htmlFor={subdirCheckboxId}
          title={subdirPath}
        >
          {node.name}
        </label>
      </div>
      {isExpanded && node.children.map((child) => (
        <BundleSubdirNode
          key={`${parentDir}/${child.relativePath}`}
          node={child}
          depth={depth + 1}
          parentDir={parentDir}
          isParentSelected={isParentSelected}
          excludedSubdirs={excludedSubdirs}
          expandedSubdirs={expandedSubdirs}
          onExpandToggle={onExpandToggle}
          onSubdirToggle={onSubdirToggle}
        />
      ))}
    </>
  );
}

export function BundleSubdirTree({
  parentDir,
  subdirs,
  isParentSelected,
  excludedSubdirs,
  onSubdirToggle,
}: BundleSubdirTreeProps): React.JSX.Element {
  const tree = useMemo(() => buildSubdirTree(subdirs), [subdirs]);
  const [expandedSubdirs, setExpandedSubdirs] = useState<Set<string>>(() => new Set());

  const handleExpandToggle = useCallback((subdirPath: string) => {
    setExpandedSubdirs((prev) => {
      const next = new Set(prev);
      if (next.has(subdirPath)) {
        next.delete(subdirPath);
      } else {
        next.add(subdirPath);
      }
      return next;
    });
  }, []);

  return (
    <div className="subdir-list">
      {tree.map((node) => (
        <BundleSubdirNode
          key={`${parentDir}/${node.relativePath}`}
          node={node}
          depth={1}
          parentDir={parentDir}
          isParentSelected={isParentSelected}
          excludedSubdirs={excludedSubdirs}
          expandedSubdirs={expandedSubdirs}
          onExpandToggle={handleExpandToggle}
          onSubdirToggle={onSubdirToggle}
        />
      ))}
    </div>
  );
}
