// Path: app/src/components/source_control/source_control_warnings.tsx
// Description: Warning rows for omitted paths and truncated status in the Source Control column

import type React from "react";
import type { SourceControlStatus } from "../../shared/protocol.js";

interface SourceControlWarningsProps {
  status: SourceControlStatus;
}

interface WarningRow {
  key: string;
  severity: "warning" | "error";
  text: string;
  title: string;
}

function buildWarningRows(status: SourceControlStatus): WarningRow[] {
  const rows: WarningRow[] = [];
  const { stagedOutsideRoot, unrepresentablePath } = status.omitted;
  if (stagedOutsideRoot > 0) {
    rows.push({
      key: "outside-root",
      severity: "warning",
      text: `${stagedOutsideRoot} STAGED OUTSIDE THIS FOLDER WILL ALSO BE COMMITTED`,
      title: "Commit always commits the whole index; paths above the configured root are not listed here",
    });
  }
  if (unrepresentablePath > 0) {
    rows.push({
      key: "unrepresentable",
      severity: "warning",
      text: `${unrepresentablePath} PATHS NOT REPRESENTABLE`,
      title: "Paths that are not valid UTF-8 are hidden; STAGE ALL still includes them",
    });
  }
  if (status.truncated) {
    rows.push({
      key: "truncated",
      severity: "error",
      text: "STATUS TRUNCATED — LISTS INCOMPLETE",
      title: "Git status overran its budget; STAGE ALL and COMMIT are disabled until a full read",
    });
  }
  return rows;
}

export function SourceControlWarnings({
  status,
}: SourceControlWarningsProps): React.JSX.Element | null {
  const rows = buildWarningRows(status);
  if (rows.length === 0) return null;

  return (
    <ul className="source-control-warnings">
      {rows.map((row) => (
        <li
          key={row.key}
          className="source-control-warning"
          data-severity={row.severity}
          title={row.title}
        >
          {row.text}
        </li>
      ))}
    </ul>
  );
}
