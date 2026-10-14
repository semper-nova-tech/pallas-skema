import type { EditorSession } from './editor-session.types';
import { showTextFailure } from './show-text-failure';

/** Refuse an edit that would fail Validation; Diagram stays unchanged (glossary: refuseInvalidEdit). */
export function refuseInvalidEdit(
  session: EditorSession,
  previousManifest: EditorSession['manifest'],
  previousJson: string,
  reason: string,
): EditorSession {
  return showTextFailure(
    {
      ...session,
      manifest: previousManifest,
      manifestJson: previousJson,
      lifecycleState:
        session.lifecycleState === 'ParseableInvalid' ? 'ParseableInvalid' : 'ValidEditing',
    },
    reason,
  );
}
