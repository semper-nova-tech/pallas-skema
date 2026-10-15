import type { ManifestDocument } from './manifest.types';

/** Editor Manifest lifecycle states (STATE-editor-manifest-lifecycle). */
export type EditorManifestLifecycleState =
  | 'NoManifest'
  | 'ValidEmpty'
  | 'ValidEditing'
  | 'ValidSaved'
  | 'ParseableInvalid'
  | 'Unparseable';

/** Glossary wire model: editorSession. */
export interface EditorSession {
  manifestPath: string;
  svgPath: string;
  manifest: ManifestDocument | null;
  manifestJson: string;
  lifecycleState: EditorManifestLifecycleState;
  textFailure: string | null;
  diagramVisible: boolean;
}
