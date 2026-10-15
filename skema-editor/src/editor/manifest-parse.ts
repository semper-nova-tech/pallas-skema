import type { ManifestDocument } from './manifest.types';

export function tryParseManifestJson(
  json: string,
): { ok: true; manifest: ManifestDocument } | { ok: false; message: string } {
  try {
    const value = JSON.parse(json) as unknown;
    if (!value || typeof value !== 'object') {
      return { ok: false, message: 'malformed JSON: root must be an object' };
    }
    const record = value as Record<string, unknown>;
    if (!Array.isArray(record['nodes']) || !Array.isArray(record['edges'])) {
      return { ok: false, message: 'malformed JSON: Manifest requires nodes and edges arrays' };
    }
    return { ok: true, manifest: value as ManifestDocument };
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return { ok: false, message: `malformed JSON: ${message}` };
  }
}

export function serializeManifest(manifest: ManifestDocument): string {
  return `${JSON.stringify(manifest, null, 2)}\n`;
}

export function emptyManifest(): ManifestDocument {
  return { nodes: [], edges: [] };
}

export function isEmptyManifest(manifest: ManifestDocument): boolean {
  return manifest.nodes.length === 0 && manifest.edges.length === 0;
}
