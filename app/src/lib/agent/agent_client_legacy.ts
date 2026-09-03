// Path: app/src/lib/agent/agent_client_legacy.ts
// Description: Legacy hostPath/windowsPath envelope normalization for older agent payloads

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function withHostPathFallback(record: Record<string, unknown>): Record<string, unknown> {
  if (typeof record.hostPath === "string") {
    return record;
  }
  if (typeof record.windowsPath !== "string") {
    return record;
  }
  return {
    ...record,
    hostPath: record.windowsPath,
  };
}

function withAliasHostPathFallback(record: Record<string, unknown>): Record<string, unknown> {
  if (typeof record.aliasHostPath === "string") {
    return record;
  }
  if (typeof record.aliasWindowsPath !== "string") {
    return record;
  }
  return {
    ...record,
    aliasHostPath: record.aliasWindowsPath,
  };
}

function normalizeLegacyPayload(payload: Record<string, unknown>): Record<string, unknown> {
  const type = payload.type;
  if (typeof type !== "string") {
    return payload;
  }

  if (type === "stageFileResult") {
    return withHostPathFallback(payload);
  }

  if (type === "buildBundleResult" || type === "bundleBuilt") {
    return withAliasHostPathFallback(withHostPathFallback(payload));
  }

  if (type === "listBundlesResult") {
    const bundles = payload.bundles;
    if (!Array.isArray(bundles)) {
      return payload;
    }
    const bundleList = bundles as unknown[];
    return {
      ...payload,
      bundles: bundleList.map((bundle: unknown): unknown => {
        return isRecord(bundle) ? withHostPathFallback(bundle) : bundle;
      }),
    };
  }

  if (type === "fileChanged") {
    const staged = payload.staged;
    if (!isRecord(staged)) {
      return payload;
    }
    return {
      ...payload,
      staged: withHostPathFallback(staged),
    };
  }

  return payload;
}

export function normalizeLegacyEnvelope(envelope: unknown): unknown {
  if (!isRecord(envelope)) {
    return envelope;
  }

  const kind = envelope.kind;
  if (kind === "event" && isRecord(envelope.payload)) {
    return {
      ...envelope,
      payload: normalizeLegacyPayload(envelope.payload),
    };
  }

  if (
    kind === "response" &&
    envelope.status === "ok" &&
    isRecord(envelope.payload)
  ) {
    return {
      ...envelope,
      payload: normalizeLegacyPayload(envelope.payload),
    };
  }

  return envelope;
}
