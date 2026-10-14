import type { EditorJsonHandoff } from './manifest.types';
import type { EditorSession } from './editor-session.types';
import { emptyManifest, isEmptyManifest, tryParseManifestJson } from './manifest-parse';
import type { SkemaTransport } from './skema-transport';
import { showTextFailure } from './show-text-failure';

/** Open a handoff into an Editor session (glossary: loadManifest). */
export async function loadManifest(
  handoff: EditorJsonHandoff,
  transport: SkemaTransport,
): Promise<EditorSession> {
  const base: EditorSession = {
    manifestPath: handoff.manifestPath,
    svgPath: handoff.svgPath,
    manifest: null,
    manifestJson: handoff.manifestJson,
    lifecycleState: 'NoManifest',
    textFailure: null,
    diagramVisible: false,
  };

  const parsed = tryParseManifestJson(handoff.manifestJson);
  if (!parsed.ok) {
    return showTextFailure(
      { ...base, lifecycleState: 'Unparseable', diagramVisible: false },
      parsed.message,
    );
  }

  const manifest = parsed.manifest;
  const tempPath = await transport.writeTempManifest(handoff.manifestJson);
  const validation = await transport.validateManifestFile(tempPath);
  await transport.removeTempFile(tempPath);

  const diagramVisible = true;
  if (!validation.ok) {
    return showTextFailure(
      {
        ...base,
        manifest,
        lifecycleState: 'ParseableInvalid',
        diagramVisible,
      },
      validation.textFailure,
    );
  }

  if (isEmptyManifest(manifest)) {
    return {
      ...base,
      manifest,
      lifecycleState: 'ValidEmpty',
      diagramVisible,
    };
  }

  return {
    ...base,
    manifest,
    lifecycleState: 'ValidEditing',
    diagramVisible,
  };
}

/** Bootstrap empty Manifest when Human reviewer starts with no file (BEHAVIOR-empty-start). */
export function createEmptySession(handoff: Pick<EditorJsonHandoff, 'manifestPath' | 'svgPath'>): EditorSession {
  const manifest = emptyManifest();
  return {
    manifestPath: handoff.manifestPath,
    svgPath: handoff.svgPath,
    manifest,
    manifestJson: JSON.stringify(manifest, null, 2),
    lifecycleState: 'ValidEmpty',
    textFailure: null,
    diagramVisible: true,
  };
}
