/** Glossary module: editor */

export type { EditorSession, EditorManifestLifecycleState } from './editor-session.types';
export type { EditorJsonHandoff, ManifestDocument, ManifestNode, ManifestEdge } from './manifest.types';
export type { SkemaTransport } from './skema-transport';

export { loadManifest, createEmptySession } from './load-manifest';
export { saveManifest } from './save-manifest';
export { refuseInvalidEdit } from './refuse-invalid-edit';
export { showTextFailure } from './show-text-failure';
export { applyNodeGridMove, applyNodeLabelEdit, addNodeAt } from './apply-edit';
