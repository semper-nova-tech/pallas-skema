import {
  applyNodeGridMove,
  createEmptySession,
  loadManifest,
  refuseInvalidEdit,
  saveManifest,
  showTextFailure,
  type EditorJsonHandoff,
  type SkemaTransport,
} from './index';

function mockTransport(validateResults: Array<{ ok: boolean; textFailure?: string }>): SkemaTransport {
  let call = 0;
  return {
    async validateManifestFile() {
      const result = validateResults[call] ?? { ok: true };
      call += 1;
      return result.ok
        ? { ok: true }
        : { ok: false, textFailure: result.textFailure ?? 'validation failed' };
    },
    async renderManifest() {
      return { ok: true };
    },
    async writeManifestFile() {},
    async writeTempManifest(json: string) {
      return `/tmp/mock-${json.length}.json`;
    },
    async removeTempFile() {},
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

  it('blocks save when Validation fails and does not render', async () => {
    const writes: string[] = [];
    const renders: string[] = [];
    const transport: SkemaTransport = {
      async validateManifestFile() {
        return { ok: false, textFailure: 'blocked' };
      },
      async renderManifest(manifestPath, svgPath) {
        renders.push(`${manifestPath}:${svgPath}`);
        return { ok: true };
      },
      async writeManifestFile(_path, json) {
        writes.push(json);
      },
      async writeTempManifest(json) {
        return `/tmp/t-${json.length}.json`;
      },
      async removeTempFile() {},
    };
    const session = createEmptySession({ manifestPath: '/w/m.json', svgPath: '/w/m.svg' });
    const result = await saveManifest(session, transport);
    expect(result.textFailure).toBe('blocked');
    expect(writes.length).toBe(0);
    expect(renders.length).toBe(0);
  });

  it('save triggers render after Validation passes (BEHAVIOR-editor-save-render)', async () => {
    let rendered = false;
    let written = false;
    const transport: SkemaTransport = {
      async validateManifestFile() {
        return { ok: true };
      },
      async renderManifest() {
        rendered = true;
        return { ok: true };
      },
      async writeManifestFile() {
        written = true;
      },
      async writeTempManifest(json) {
        return `/tmp/t-${json.length}.json`;
      },
      async removeTempFile() {},
    };
    const session = createEmptySession({ manifestPath: '/w/m.json', svgPath: '/w/m.svg' });
    const result = await saveManifest(session, transport);
    expect(result.lifecycleState).toBe('ValidSaved');
    expect(written).toBeTrue();
    expect(rendered).toBeTrue();
  });

  it('exports glossary helpers showTextFailure and refuseInvalidEdit', () => {
    const session = createEmptySession({ manifestPath: '/w/m.json', svgPath: '/w/m.svg' });
    const withFailure = showTextFailure(session, 'msg');
    expect(withFailure.textFailure).toBe('msg');
    const refused = refuseInvalidEdit(session, session.manifest, session.manifestJson, 'no');
    expect(refused.textFailure).toBe('no');
  });
});
