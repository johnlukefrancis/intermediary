// Path: app/src/components/options/excludes/excludes_normalizers.ts
// Description: Normalization helpers for global excludes inputs

export function normalizeExtension(value: string): string {
  const trimmed = value.trim().toLowerCase();
  if (trimmed.length === 0) return "";
  if (trimmed === "~") return "~";
  return trimmed.startsWith(".") ? trimmed : `.${trimmed}`;
}

export function normalizePattern(value: string): string {
  const trimmed = value.trim().replace(/^\/+|\/+$/g, "").toLowerCase();
  return trimmed;
}

export function normalizeName(value: string): string {
  return value.trim().toLowerCase();
}
