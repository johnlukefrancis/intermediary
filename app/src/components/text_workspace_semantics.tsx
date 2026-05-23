// Path: app/src/components/text_workspace_semantics.tsx
// Description: Theme-aware semantic text layer for the workspace editor

import type React from "react";
import { useMemo } from "react";

export type TextWorkspaceSemanticMode = "markdown" | "plain";

interface TextWorkspaceSemanticsProps {
  value: string;
  placeholder: string;
  mode: TextWorkspaceSemanticMode;
}

type InlineSegmentKind = "code" | "emphasis" | "link" | "strong" | "text";

interface InlineSegment {
  kind: InlineSegmentKind;
  text: string;
}

const INLINE_TOKEN_PATTERN =
  /(`[^`\n]+`|\[[^\]\n]+\]\([^) \n][^)\n]*\)|\*\*[^*\n]+\*\*|__[^_\n]+__|\*[^*\n]+\*|_[^_\n]+_)/g;
const MARKDOWN_PARSE_CHAR_LIMIT = 120_000;

export function TextWorkspaceSemantics({
  value,
  placeholder,
  mode,
}: TextWorkspaceSemanticsProps): React.JSX.Element {
  const isEmpty = value.length === 0;
  const content = isEmpty ? placeholder : value;
  const shouldRenderMarkdown =
    mode === "markdown" && !isEmpty && value.length <= MARKDOWN_PARSE_CHAR_LIMIT;
  const renderedContent = useMemo(
    () => shouldRenderMarkdown ? renderMarkdown(content) : content,
    [content, shouldRenderMarkdown]
  );

  return (
    <div
      className="text-workspace-semantic-layer"
      data-empty={isEmpty || undefined}
      data-mode={mode}
      data-limited={mode === "markdown" && !shouldRenderMarkdown && !isEmpty ? true : undefined}
      aria-hidden="true"
    >
      <div className="text-workspace-semantic-content">
        {renderedContent}
      </div>
    </div>
  );
}

function renderMarkdown(value: string): React.ReactNode {
  const normalized = value.replace(/\r\n/g, "\n").replace(/\r/g, "\n");
  const lines = normalized.split("\n");
  let isCodeBlock = false;

  return lines.map((line, index) => {
    const fenceMatch = line.match(/^(\s*)(`{3,}|~{3,})(.*)$/);
    const isFence = fenceMatch !== null;
    const wasCodeBlock = isCodeBlock;
    if (isFence) isCodeBlock = !isCodeBlock;

    return (
      <span key={`${index}:${line}`} className={lineClassName(line, wasCodeBlock, isFence)}>
        {wasCodeBlock || isFence ? line : renderMarkdownLine(line)}
        {index < lines.length - 1 ? "\n" : null}
      </span>
    );
  });
}

function lineClassName(line: string, isCodeBlock: boolean, isFence: boolean): string {
  if (isFence) return "tw-md-line tw-md-code-fence";
  if (isCodeBlock) return "tw-md-line tw-md-code-block";
  if (/^\s{0,3}#{1,6}(\s|$)/.test(line)) return "tw-md-line tw-md-heading";
  if (/^\s*>/.test(line)) return "tw-md-line tw-md-quote";
  if (/^\s*(?:[-*+]|\d+[.)])\s+/.test(line)) return "tw-md-line tw-md-list";
  if (/^\s*(?:-{3,}|\*{3,}|_{3,})\s*$/.test(line)) return "tw-md-line tw-md-rule";
  return "tw-md-line";
}

function renderMarkdownLine(line: string): React.ReactNode {
  const heading = line.match(/^(\s{0,3})(#{1,6})(\s.*)?$/);
  if (heading) {
    return (
      <>
        {heading[1]}
        <span className="tw-md-marker">{heading[2]}</span>
        {renderInlineSegments(heading[3] ?? "")}
      </>
    );
  }

  const quote = line.match(/^(\s*>+\s?)(.*)$/);
  if (quote) {
    return (
      <>
        <span className="tw-md-marker">{quote[1]}</span>
        {renderInlineSegments(quote[2] ?? "")}
      </>
    );
  }

  const list = line.match(/^(\s*)([-*+]|\d+[.)])(\s+)(.*)$/);
  if (list) {
    return (
      <>
        {list[1]}
        <span className="tw-md-marker">{list[2]}</span>
        {list[3]}
        {renderInlineSegments(list[4] ?? "")}
      </>
    );
  }

  return renderInlineSegments(line);
}

function renderInlineSegments(line: string): React.ReactNode {
  return parseInlineSegments(line).map((segment, index) => (
    <span key={`${index}:${segment.text}`} className={`tw-md-inline tw-md-inline-${segment.kind}`}>
      {segment.text}
    </span>
  ));
}

function parseInlineSegments(line: string): InlineSegment[] {
  const segments: InlineSegment[] = [];
  let lastIndex = 0;

  for (const match of line.matchAll(INLINE_TOKEN_PATTERN)) {
    const matchIndex = match.index;
    if (matchIndex > lastIndex) {
      segments.push({ kind: "text", text: line.slice(lastIndex, matchIndex) });
    }
    const text = match[0];
    segments.push({ kind: inlineSegmentKind(text), text });
    lastIndex = matchIndex + text.length;
  }

  if (lastIndex < line.length) {
    segments.push({ kind: "text", text: line.slice(lastIndex) });
  }

  return segments.length > 0 ? segments : [{ kind: "text", text: "" }];
}

function inlineSegmentKind(text: string): InlineSegmentKind {
  if (text.startsWith("`")) return "code";
  if (text.startsWith("[")) return "link";
  if (text.startsWith("**") || text.startsWith("__")) return "strong";
  return "emphasis";
}
