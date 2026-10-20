import { applyConnectEdge, applyNodeGridMove } from './apply-edit';
import type { EditorSession } from './editor-session.types';
import { createEmptySession, loadManifest } from './load-manifest';
import { refuseInvalidEdit, showTextFailure, saveManifest } from './index';
import type { EditorJsonHandoff } from './manifest.types';
import type { SkemaTransport } from './skema-transport';

function mockTransport(validateResults: Array<{ ok: boolean; textFailure?: string }>): SkemaTransport {
  let call = 0;
  return {
    async validateManifestJson() {
      const result = validateResults[call] ?? { ok: true };
      call += 1;
      return result.ok
        ? { ok: true }
        : { ok: false, textFailure: result.textFailure ?? 'validation failed' };
    },
    async saveManifest() {
      return { ok: true };
    },
  };
}

describe('Editor session (STATE-editor-manifest-lifecycle)', () => {
  it('shows Text failure for unparseable JSON (BEHAVIOR-repair-invalid)', async () => {
    const handoff: EditorJsonHandoff = {
      manifestJson: '{not json',
      manifestPath: '/w/m.json',
      svgPath: '/w/m.svg',
    };
    const session = await loadManifest(handoff, mockTransport([]));
    expect(session.lifecycleState).toBe('Unparseable');
    expect(session.diagramVisible).toBeFalse();
    expect(session.textFailure).toContain('malformed JSON');
  });

  it('opens parseable invalid Manifest for repair', async () => {
    const invalid = JSON.stringify({
      nodes: [
        { id: 'a', label: 'a', type: 'process', size: 'medium', gridPosition: { x: 0, y: 0 } },
        { id: 'b', label: 'b', type: 'process', size: 'medium', gridPosition: { x: 0, y: 0 } },
      ],
      edges: [],
    });
    const handoff: EditorJsonHandoff = {
      manifestJson: invalid,
      manifestPath: '/w/m.json',
      svgPath: '/w/m.svg',
    };
    const session = await loadManifest(handoff, mockTransport([{ ok: false, textFailure: 'overlap' }]));
    expect(session.lifecycleState).toBe('ParseableInvalid');
    expect(session.diagramVisible).toBeTrue();
    expect(session.textFailure).toBe('overlap');
  });

  it('starts empty (BEHAVIOR-empty-start)', () => {
    const session = createEmptySession({ manifestPath: '/w/m.json', svgPath: '/w/m.svg' });
    expect(session.manifest?.nodes.length).toBe(0);
    expect(session.manifest?.edges.length).toBe(0);
    expect(session.lifecycleState).toBe('ValidEmpty');
  });

  it('refuses invalid grid move (BEHAVIOR-refuse-invalid-edit)', async () => {
    const manifest = {
      nodes: [
        { id: 'a', label: 'a', type: 'process' as const, size: 'medium' as const, gridPosition: { x: 0, y: 0 } },
        { id: 'b', label: 'b', type: 'process' as const, size: 'medium' as const, gridPosition: { x: 1, y: 0 } },
      ],
      edges: [],
    };
    const base = {
      manifestPath: '/w/m.json',
      svgPath: '/w/m.svg',
      manifest,
      manifestJson: JSON.stringify(manifest),
      lifecycleState: 'ValidEditing' as const,
      textFailure: null,
      diagramVisible: true,
    };
    const refused = await applyNodeGridMove(base, mockTransport([{ ok: false, textFailure: 'overlap' }]), 'b', 0, 0);
    expect(refused.textFailure).toBe('overlap');
    expect(refused.manifest?.nodes.find((n) => n.id === 'b')?.gridPosition).toEqual({ x: 1, y: 0 });
  });

  it('persists Edge on save (BEHAVIOR-editor-roundtrip)', async () => {
    let written = '';
    const manifest = {
      nodes: [
        { id: 'a', label: 'a', type: 'process' as const, size: 'medium' as const, gridPosition: { x: 0, y: 0 } },
        { id: 'b', label: 'b', type: 'process' as const, size: 'medium' as const, gridPosition: { x: 2, y: 0 } },
      ],
      edges: [] as { from: string; to: string }[],
    };
    let session: EditorSession = {
      manifestPath: '/w/m.json',
      svgPath: '/w/m.svg',
      manifest,
      manifestJson: JSON.stringify(manifest),
      lifecycleState: 'ValidEditing',
      textFailure: null,
      diagramVisible: true,
    };
    session = await applyConnectEdge(session, mockTransport([{ ok: true }]), 'a', 'b');
    expect(session.manifest?.edges).toEqual([{ from: 'a', to: 'b' }]);

    const transport: SkemaTransport = {
      async validateManifestJson() {
        return { ok: true };
      },
      async saveManifest(_manifestPath, _svgPath, json) {
        written = json;
        return { ok: true };
      },
    };
    const saved = await saveManifest(session, transport);
    expect(saved.lifecycleState).toBe('ValidSaved');
    expect(written).toContain('"from": "a"');
    expect(written).toContain('"to": "b"');
  });

  it('surfaces a blocked save as Text failure and keeps the prior state', async () => {
    const transport: SkemaTransport = {
      async validateManifestJson() {
        return { ok: true };
      },
      async saveManifest() {
        return { ok: false, textFailure: 'blocked' };
      },
    };
    const session = createEmptySession({ manifestPath: '/w/m.json', svgPath: '/w/m.svg' });
    const result = await saveManifest(session, transport);
    expect(result.textFailure).toBe('blocked');
    expect(result.lifecycleState).toBe('ValidEmpty');
  });

  it('save passes the launch paths and serialized Manifest (BEHAVIOR-editor-save-render)', async () => {
    const saves: Array<{ manifestPath: string; svgPath: string; json: string }> = [];
    const transport: SkemaTransport = {
      async validateManifestJson() {
        return { ok: true };
      },
      async saveManifest(manifestPath, svgPath, json) {
        saves.push({ manifestPath, svgPath, json });
        return { ok: true };
      },
    };
    const session = createEmptySession({ manifestPath: '/w/m.json', svgPath: '/w/m.svg' });
    const result = await saveManifest(session, transport);
    expect(result.lifecycleState).toBe('ValidSaved');
    expect(saves).toEqual([
      { manifestPath: '/w/m.json', svgPath: '/w/m.svg', json: '{\n  "nodes": [],\n  "edges": []\n}\n' },
    ]);
  });

  it('exports glossary helpers showTextFailure and refuseInvalidEdit', () => {
    const session = createEmptySession({ manifestPath: '/w/m.json', svgPath: '/w/m.svg' });
    const withFailure = showTextFailure(session, 'msg');
    expect(withFailure.textFailure).toBe('msg');
    const refused = refuseInvalidEdit(session, session.manifest, session.manifestJson, 'no');
    expect(refused.textFailure).toBe('no');
  });
});
