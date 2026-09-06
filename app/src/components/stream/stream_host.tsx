// Path: app/src/components/stream/stream_host.tsx
// Description: Renders nothing; keeps the Stream store registry fed for the life of the app

import type React from "react";
import { useStreamHost } from "../../hooks/stream/use_stream_host.js";

export function StreamHost({ documentHidden }: { documentHidden: boolean }): React.JSX.Element | null {
  useStreamHost(documentHidden);
  return null;
}
