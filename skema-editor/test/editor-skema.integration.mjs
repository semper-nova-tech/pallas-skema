import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, '../..');
const skemaBin = process.env.SKEMA_BIN ?? path.join(repoRoot, 'target/release/skema');

function runSkema(args) {
  return new Promise((resolve, reject) => {
    const child = spawn(skemaBin, args, { cwd: repoRoot });
    let stderr = '';
    child.stderr.on('data', (c) => {
      stderr += c.toString();
    });
    child.on('error', reject);
    child.on('close', (code) => resolve({ code, stderr: stderr.trim() }));
  });
}

test('CLI: empty Manifest validates and renders', async () => {
  const dir = await fs.mkdtemp(path.join(os.tmpdir(), 'skema-cli-empty-'));
  const manifestPath = path.join(dir, 'manifest.json');
  const svgPath = path.join(dir, 'diagram.svg');
  await fs.writeFile(manifestPath, '{"nodes":[],"edges":[]}\n', 'utf8');
  const validate = await runSkema(['validate', '--manifest', manifestPath]);
  assert.equal(validate.code, 0);
  const render = await runSkema(['render', '--manifest', manifestPath, '--svg', svgPath]);
  assert.equal(render.code, 0);
  const svg = await fs.readFile(svgPath, 'utf8');
  assert.match(svg, /<svg/);
});

test('CLI: overlapping Nodes fail Validation', async () => {
  const dir = await fs.mkdtemp(path.join(os.tmpdir(), 'skema-cli-refuse-'));
  const manifestPath = path.join(dir, 'manifest.json');
  const body = {
    nodes: [
      { id: 'a', label: 'a', type: 'process', size: 'medium', gridPosition: { x: 0, y: 0 } },
      { id: 'b', label: 'b', type: 'process', size: 'medium', gridPosition: { x: 0, y: 0 } },
    ],
    edges: [],
  };
  await fs.writeFile(manifestPath, `${JSON.stringify(body, null, 2)}\n`, 'utf8');
  const validate = await runSkema(['validate', '--manifest', manifestPath]);
  assert.notEqual(validate.code, 0);
  assert.match(validate.stderr, /overlap/i);
});
