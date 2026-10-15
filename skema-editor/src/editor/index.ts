/** Glossary module: editor — public surfaces only. */

export type { EditorSession, EditorManifestLifecycleState } from './editor-session.types';

export { loadManifest } from './load-manifest';
export { saveManifest } from './save-manifest';
export { refuseInvalidEdit } from './refuse-invalid-edit';
export { showTextFailure } from './show-text-failure';
