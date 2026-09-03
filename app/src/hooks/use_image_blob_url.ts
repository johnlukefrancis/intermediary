// Path: app/src/hooks/use_image_blob_url.ts
// Description: Base64 image payload to a revocable Blob URL for workspace image and image-diff panes

import { useEffect, useState } from "react";

export type ImageBlobUrlState =
  | { status: "idle" }
  | { status: "ready"; url: string }
  | { status: "error"; message: string };

function base64ToBlob(dataBase64: string, mimeType: string): Blob {
  const binary = globalThis.atob(dataBase64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    bytes[i] = binary.charCodeAt(i);
  }
  return new Blob([bytes], { type: mimeType });
}

/** The URL lives exactly as long as the payload: every change revokes the previous object URL. */
export function useImageBlobUrl(
  dataBase64: string | undefined,
  mimeType: string | undefined
): ImageBlobUrlState {
  const [state, setState] = useState<ImageBlobUrlState>({ status: "idle" });

  useEffect(() => {
    if (!dataBase64 || !mimeType) {
      setState({ status: "idle" });
      return undefined;
    }

    let url: string;
    try {
      url = URL.createObjectURL(base64ToBlob(dataBase64, mimeType));
    } catch (err) {
      const message = err instanceof Error ? err.message : "Unable to decode image preview";
      setState({ status: "error", message });
      return undefined;
    }

    setState({ status: "ready", url });
    return () => {
      URL.revokeObjectURL(url);
    };
  }, [dataBase64, mimeType]);

  return state;
}
