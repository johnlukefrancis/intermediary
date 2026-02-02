// Path: app/src/lib/theme/texture_catalog.ts
// Description: Build-time texture catalog for theme substrate/dither selection

type TextureModuleMap = Record<string, string>;

export interface TextureOption {
  id: string;
  label: string;
  url: string;
}

export const DEFAULT_TEXTURE_ID = "buried_grain";

const textureModules = import.meta.glob<string>(
  "/assets/textures/*.png",
  {
    eager: true,
    import: "default",
  }
) as TextureModuleMap;

const textureOptions = Object.entries(textureModules)
  .map(([path, url]): TextureOption => {
    const filename = path.split("/").pop() ?? path;
    const id = filename.replace(/\.png$/i, "");
    return {
      id,
      label: toLabel(id),
      url,
    };
  })
  .sort((a, b) => a.label.localeCompare(b.label));

export function getTextureOptions(): TextureOption[] {
  return textureOptions;
}

export function getTextureById(textureId: string): TextureOption | undefined {
  return textureOptions.find((option) => option.id === textureId);
}

export function resolveTextureUrl(textureId: string | null | undefined): string | null {
  const resolved = textureId ? getTextureById(textureId) : undefined;
  if (resolved) return resolved.url;

  const fallback = getTextureById(DEFAULT_TEXTURE_ID) ?? textureOptions[0];
  return fallback?.url ?? null;
}

function toLabel(id: string): string {
  return id
    .split("_")
    .filter((segment) => segment.length > 0)
    .map((segment) => {
      const head = segment.slice(0, 1).toUpperCase();
      return `${head}${segment.slice(1)}`;
    })
    .join(" ");
}
