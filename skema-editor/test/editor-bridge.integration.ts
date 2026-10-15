import assert from 'node:assert/strict';
import { spawn, type ChildProcess } from 'node:child_process';
import fs from 'node:fs/promises';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

import { applyConnectEdge, applyNodeGridMove } from '../src/editor/apply-edit.ts';
import { loadManifest } from '../src/editor/load-manifest.ts';
import { saveManifest } from '../src/editor/save-manifest.ts';
import type { SkemaTransport } from '../src/editor/skema-transport.ts';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const editorRoot = path.resolve(__dirname, '..');
const repoRoot = path.resolve(editorRoot, '..');

function createHttpTransport(baseUrl: string): SkemaTransport {
  async function post(action: string, body: Record<string, string>) {
    const res = await fetch(`${baseUrl}/api/skema/${action}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    return (await res.json()) as { ok: boolean; textFailure?: string; path?: string };
  }

  return {
    validateManifestFile: async (manifestPath) => {
      const result = await post('validate', { manifestPath });
      return result.ok ? { ok: true } : { ok: false, textFailure: result.textFailure ?? 'validate failed' };
    },
    renderManifest: async (manifestPath, svgPath) => {
      const result = await post('render', { manifestPath, svgPath });
      return result.ok ? { ok: true } : { ok: false, textFailure: result.textFailure ?? 'render failed' };
    },
    writeManifestFile: async (manifestPath, manifestJson) => {
      await post('write-manifest', { manifestPath, manifestJson });
    },
    writeTempManifest: async (manifestJson) => {
      const result = await post('temp-manifest', { manifestJson });
      assert.ok(result.path);
      return result.path;
    },
    removeTempFile: async (tempPath) => {
      await post('remove-temp', { path: tempPath });
    },
  };
}

async function waitForBridge(baseUrl: string, attempts = 40): Promise<void> {
  for (let i = 0; i < attempts; i += 1) {
    try {
      const res = await fetch(`${baseUrl}/api/skema/validate`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ manifestPath: `${repoRoot}/package.json` }),
      });
      if (res.ok) {
        return;
      }
    } catch {
      // retry
    }
    await new Promise((r) => setTimeout(r, 50));
  }
  throw new Error('skema bridge did not become ready');
}

function startBridge(port: number, workspace: string): ChildProcess {
  return spawn(process.execPath, ['server/skema-bridge.mjs'], {
    cwd: editorRoot,
    env: {
      ...process.env,
      SKEMA_BRIDGE_PORT: String(port),
      SKEMA_WORKSPACE: workspace,
      SKEMA_BIN: process.env.SKEMA_BIN ?? path.join(repoRoot, 'target/release/skema'),
    },
    stdio: 'ignore',
  });
}

test('Editor: grid move, Edge connect, and saveManifest through bridge write Manifest and SVG', async () => {
  const port = 3900 + Math.floor(Math.random() * 200);
  const workspace = path.join(repoRoot, '.skema-editor-int', `run-${Date.now()}`);
  await fs.mkdir(workspace, { recursive: true });

  const manifestPath = path.join(workspace, 'manifest.json');
  const svgPath = path.join(workspace, 'diagram.svg');
  const initial = {
    nodes: [
      { id: 'start', label: 'Start', type: 'process', size: 'medium', gridPosition: { x: 0, y: 0 } },
      { id: 'next', label: 'Next', type: 'process', size: 'medium', gridPosition: { x: 2, y: 0 } },
    ],
    edges: [],
  };
  const manifestJson = `${JSON.stringify(initial, null, 2)}\n`;

  const bridge = startBridge(port, repoRoot);
  const baseUrl = `http://127.0.0.1:${port}`;
  try {
    await waitForBridge(baseUrl);
    const transport = createHttpTransport(baseUrl);

    let session = await loadManifest(
      { manifestJson, manifestPath, svgPath },
      transport,
    );
    assert.equal(session.lifecycleState, 'ValidEditing');

    session = await applyNodeGridMove(session, transport, 'next', 3, 0);
    assert.equal(session.textFailure, null);
    assert.equal(session.manifest?.nodes.find((n) => n.id === 'next')?.gridPosition.x, 3);

    session = await applyConnectEdge(session, transport, 'start', 'next');
    assert.equal(session.textFailure, null);
    assert.deepEqual(session.manifest?.edges, [{ from: 'start', to: 'next' }]);

    session = await saveManifest(session, transport);
    assert.equal(session.lifecycleState, 'ValidSaved');
    assert.equal(session.textFailure, null);

    const onDisk = JSON.parse(await fs.readFile(manifestPath, 'utf8')) as {
      nodes: { id: string; gridPosition: { x: number } }[];
      edges: { from: string; to: string }[];
    };
    assert.equal(onDisk.nodes.find((n) => n.id === 'next')?.gridPosition.x, 3);
    assert.deepEqual(onDisk.edges, [{ from: 'start', to: 'next' }]);

    const svg = await fs.readFile(svgPath, 'utf8');
    assert.match(svg, /<svg/);
  } finally {
    bridge.kill();
    await fs.rm(workspace, { recursive: true, force: true });
  }
});
