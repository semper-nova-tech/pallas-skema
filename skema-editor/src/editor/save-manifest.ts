import type { EditorSession } from './editor-session.types';
import { serializeManifest } from './manifest-parse';
import type { SkemaTransport } from './skema-transport';
import { showTextFailure } from './show-text-failure';

/** Persist Manifest after Validation and invoke Render (glossary: saveManifest). */
export async function saveManifest(
  session: EditorSession,
  transport: SkemaTransport,
): Promise<EditorSession> {
  if (!session.manifest || !session.diagramVisible) {
    return showTextFailure(session, 'Cannot save: no editable Manifest');
  }

  const manifestJson = serializeManifest(session.manifest);
  const tempPath = await transport.writeTempManifest(manifestJson);
  const validation = await transport.validateManifestFile(tempPath);
  await transport.removeTempFile(tempPath);

  if (!validation.ok) {
    const lifecycleState =
      session.lifecycleState === 'ParseableInvalid' ? 'ParseableInvalid' : session.lifecycleState;
    return showTextFailure({ ...session, lifecycleState }, validation.textFailure);
  }

  await transport.writeManifestFile(session.manifestPath, manifestJson);
  const render = await transport.renderManifest(session.manifestPath, session.svgPath);
  if (!render.ok) {
    return showTextFailure(session, render.textFailure);
  }

  return {
    ...session,
    manifestJson,
    textFailure: null,
    lifecycleState: 'ValidSaved',
  };
}
