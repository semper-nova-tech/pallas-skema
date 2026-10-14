import type { EditorSession } from './editor-session.types';

/** Surface a Text failure on the Editor session (glossary: showTextFailure). */
export function showTextFailure(session: EditorSession, message: string): EditorSession {
  return {
    ...session,
    textFailure: message,
  };
}
