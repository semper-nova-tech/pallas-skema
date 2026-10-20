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
  const result = await transport.saveManifest(session.manifestPath, session.svgPath, manifestJson);
  if (!result.ok) {
    return showTextFailure(session, result.textFailure);
  }

  return {
    ...session,
    manifestJson,
    textFailure: null,
    lifecycleState: 'ValidSaved',
  };
}
