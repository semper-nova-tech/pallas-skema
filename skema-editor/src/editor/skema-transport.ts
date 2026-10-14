/** Calls the `skema` binary for validateManifest and renderManifest (PT-002). */

export interface SkemaTransport {
  validateManifestFile(manifestPath: string): Promise<{ ok: true } | { ok: false; textFailure: string }>;
  renderManifest(manifestPath: string, svgPath: string): Promise<{ ok: true } | { ok: false; textFailure: string }>;
  writeManifestFile(manifestPath: string, manifestJson: string): Promise<void>;
  writeTempManifest(manifestJson: string): Promise<string>;
  removeTempFile(path: string): Promise<void>;
}
