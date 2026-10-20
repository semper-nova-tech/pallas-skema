/** Calls the skema CLI's Editor server for Validation and save (PT-002). */

export type SkemaOutcome = { ok: true } | { ok: false; textFailure: string };

export interface SkemaTransport {
  /** Validate Manifest JSON in memory; writes nothing. */
  validateManifestJson(manifestJson: string): Promise<SkemaOutcome>;
  /** Validate, write the Manifest, then Render the SVG. Validation failure writes neither. */
  saveManifest(manifestPath: string, svgPath: string, manifestJson: string): Promise<SkemaOutcome>;
}
