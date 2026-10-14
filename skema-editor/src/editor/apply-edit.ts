import type { ManifestDocument, ManifestNode } from './manifest.types';
import type { EditorSession } from './editor-session.types';
import { refuseInvalidEdit } from './refuse-invalid-edit';
import { serializeManifest } from './manifest-parse';
import type { SkemaTransport } from './skema-transport';

function cloneManifest(manifest: ManifestDocument): ManifestDocument {
  return structuredClone(manifest);
}

/** Move a Node to a Grid cell; refuses overlap and invalid coordinates via Rust Validation. */
export async function applyNodeGridMove(
  session: EditorSession,
  transport: SkemaTransport,
  nodeId: string,
  x: number,
  y: number,
): Promise<EditorSession> {
  if (!session.manifest || !session.diagramVisible) {
    return session;
  }

  const previousManifest = session.manifest;
  const previousJson = session.manifestJson;

  const next = cloneManifest(session.manifest);
  const node = next.nodes.find((n) => n.id === nodeId);
  if (!node) {
    return session;
  }
  node.gridPosition = { x, y };

  const candidateJson = serializeManifest(next);
  const tempPath = await transport.writeTempManifest(candidateJson);
  const validation = await transport.validateManifestFile(tempPath);
  await transport.removeTempFile(tempPath);

  if (!validation.ok) {
    return refuseInvalidEdit(session, previousManifest, previousJson, validation.textFailure);
  }

  const lifecycleState =
    session.lifecycleState === 'ValidEmpty' || session.lifecycleState === 'ValidSaved'
      ? 'ValidEditing'
      : session.lifecycleState === 'ParseableInvalid'
        ? 'ParseableInvalid'
        : 'ValidEditing';

  return {
    ...session,
    manifest: next,
    manifestJson: candidateJson,
    lifecycleState,
    textFailure: null,
  };
}

export function applyNodeLabelEdit(
  session: EditorSession,
  nodeId: string,
  label: string,
): EditorSession {
  if (!session.manifest || !session.diagramVisible) {
    return session;
  }
  const next = cloneManifest(session.manifest);
  const node = next.nodes.find((n) => n.id === nodeId);
  if (!node) {
    return session;
  }
  node.label = label;
  return {
    ...session,
    manifest: next,
    manifestJson: serializeManifest(next),
    lifecycleState: session.lifecycleState === 'ValidEmpty' ? 'ValidEditing' : session.lifecycleState,
    textFailure: null,
  };
}

/** Connect an Edge between two Nodes; Validation runs via Rust before the edit sticks. */
export async function applyConnectEdge(
  session: EditorSession,
  transport: SkemaTransport,
  fromId: string,
  toId: string,
): Promise<EditorSession> {
  if (!session.manifest || !session.diagramVisible) {
    return session;
  }

  const previousManifest = session.manifest;
  const previousJson = session.manifestJson;

  const next = cloneManifest(session.manifest);
  next.edges.push({ from: fromId, to: toId });

  const candidateJson = serializeManifest(next);
  const tempPath = await transport.writeTempManifest(candidateJson);
  const validation = await transport.validateManifestFile(tempPath);
  await transport.removeTempFile(tempPath);

  if (!validation.ok) {
    return refuseInvalidEdit(session, previousManifest, previousJson, validation.textFailure);
  }

  return {
    ...session,
    manifest: next,
    manifestJson: candidateJson,
    lifecycleState: 'ValidEditing',
    textFailure: null,
  };
}

export function addNodeAt(
  session: EditorSession,
  node: ManifestNode,
): EditorSession {
  if (!session.manifest || !session.diagramVisible) {
    return session;
  }
  const next = cloneManifest(session.manifest);
  next.nodes.push(node);
  return {
    ...session,
    manifest: next,
    manifestJson: serializeManifest(next),
    lifecycleState: 'ValidEditing',
    textFailure: null,
  };
}
