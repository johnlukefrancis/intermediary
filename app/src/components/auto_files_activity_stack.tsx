// Path: app/src/components/auto_files_activity_stack.tsx
// Description: Consolidated activity waveform and pulse strip for Auto files rows

import type React from "react";
import type { FileActivityGraphColumn } from "../lib/files/file_feed.js";

const ACTIVITY_GRAPH_ROWS = 6;

interface AutoFilesActivityStackProps {
  activityGraph: readonly FileActivityGraphColumn[];
  pulse: readonly number[];
}

function pulseStyle(count: number, pulseMax: number): React.CSSProperties & {
  "--pulse-alpha": string;
} {
  return { "--pulse-alpha": String(0.22 + (count / pulseMax) * 0.78) };
}

function activityColumnStyle(column: FileActivityGraphColumn): React.CSSProperties & {
  "--activity-strength": string;
  "--activity-roughness": string;
} {
  return {
    "--activity-strength": column.value.toFixed(3),
    "--activity-roughness": column.roughness.toFixed(3),
  };
}

function graphDotJitter(
  column: FileActivityGraphColumn,
  columnIndex: number,
  rowIndex: number
): number {
  const unit = (column.roughness * 5.13 + columnIndex * 0.173 + rowIndex * 0.311) % 1;
  return (unit - 0.5) * 0.12;
}

function graphDotThreshold(rowIndex: number): number {
  return (rowIndex + 0.55) / ACTIVITY_GRAPH_ROWS;
}

function isGraphDotLit(
  column: FileActivityGraphColumn,
  columnIndex: number,
  rowIndex: number
): boolean {
  return column.value + graphDotJitter(column, columnIndex, rowIndex) >= graphDotThreshold(rowIndex);
}

function isGraphDotEdge(column: FileActivityGraphColumn, rowIndex: number): boolean {
  return Math.abs(column.value - graphDotThreshold(rowIndex)) <= 0.12;
}

function isGraphDotRough(
  column: FileActivityGraphColumn,
  columnIndex: number,
  rowIndex: number
): boolean {
  const unit = (column.roughness * 11.37 + columnIndex * 0.271 + rowIndex * 0.419) % 1;
  return unit > 0.62;
}

export function AutoFilesActivityStack({
  activityGraph,
  pulse,
}: AutoFilesActivityStackProps): React.JSX.Element {
  const pulseMax = Math.max(...pulse, 1);
  const displayPulse = [...pulse].reverse();

  return (
    <div className="auto-files-activity-stack" aria-label="Activity waveform and 24-hour pulse">
      <div className="auto-files-pulse" aria-hidden="true">
        {displayPulse.map((count, index) => (
          <span
            key={index}
            className="auto-files-pulse-dot"
            style={pulseStyle(count, pulseMax)}
            data-lit={count > 0 || undefined}
          />
        ))}
      </div>
      <div className="auto-files-activity" aria-hidden="true">
        {activityGraph.map((column, index) => (
          <span
            key={index}
            className="auto-files-activity-column"
            data-band={column.band}
            data-lit={column.value >= 0.08 || undefined}
            style={activityColumnStyle(column)}
          >
            {Array.from({ length: ACTIVITY_GRAPH_ROWS }, (_, rowIndex) => {
              const isLit = isGraphDotLit(column, index, rowIndex);
              const isEdge = isLit && isGraphDotEdge(column, rowIndex);
              const isRough = isLit && isGraphDotRough(column, index, rowIndex);
              return (
                <span
                  key={rowIndex}
                  className="auto-files-activity-dot"
                  data-lit={isLit || undefined}
                  data-edge={isEdge || undefined}
                  data-rough={isRough || undefined}
                />
              );
            })}
          </span>
        ))}
      </div>
    </div>
  );
}
