import { spawn } from 'node:child_process';
import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import http from 'node:http';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, '../..');
const defaultSkemaBin = path.join(repoRoot, 'target/release/skema');
const skemaBin = process.env.SKEMA_BIN ?? defaultSkemaBin;
const workspaceRoot = path.resolve(process.env.SKEMA_WORKSPACE ?? repoRoot);
const port = Number(process.env.SKEMA_BRIDGE_PORT ?? 3927);

function assertWorkspacePath(targetPath) {
  const resolved = path.resolve(targetPath);
  if (!resolved.startsWith(workspaceRoot + path.sep) && resolved !== workspaceRoot) {
    throw new Error(`path outside workspace: ${targetPath}`);
  }
  return resolved;
}

function runSkema(args) {
  return new Promise((resolve) => {
    const child = spawn(skemaBin, args, { cwd: workspaceRoot });
    let stderr = '';
    child.stderr.on('data', (chunk) => {
      stderr += chunk.toString();
    });
    child.on('close', (code) => {
      if (code === 0) {
        resolve({ ok: true });
      } else {
        resolve({ ok: false, textFailure: stderr.trim() || `skema exited ${code}` });
      }
    });
  });
}

async function readJson(req) {
  const chunks = [];
  for await (const chunk of req) {
    chunks.push(chunk);
  }
  return JSON.parse(Buffer.concat(chunks).toString('utf8'));
}

function sendJson(res, status, body) {
  res.writeHead(status, { 'Content-Type': 'application/json' });
  res.end(JSON.stringify(body));
}

const server = http.createServer(async (req, res) => {
  if (req.method !== 'POST' || !req.url?.startsWith('/api/skema/')) {
    sendJson(res, 404, { ok: false, textFailure: 'not found' });
    return;
  }

  try {
    const action = req.url.slice('/api/skema/'.length);
    const body = await readJson(req);

    if (action === 'validate') {
      const manifestPath = assertWorkspacePath(body.manifestPath);
      const result = await runSkema(['validate', '--manifest', manifestPath]);
      sendJson(res, 200, result);
      return;
    }

    if (action === 'render') {
      const manifestPath = assertWorkspacePath(body.manifestPath);
      const svgPath = assertWorkspacePath(body.svgPath);
      const result = await runSkema(['render', '--manifest', manifestPath, '--svg', svgPath]);
      sendJson(res, 200, result);
      return;
    }

    if (action === 'write-manifest') {
      const manifestPath = assertWorkspacePath(body.manifestPath);
      await fs.mkdir(path.dirname(manifestPath), { recursive: true });
      await fs.writeFile(manifestPath, body.manifestJson, 'utf8');
      sendJson(res, 200, { ok: true });
      return;
    }

    if (action === 'temp-manifest') {
      const tmpRoot = path.join(workspaceRoot, '.skema-editor-tmp');
      await fs.mkdir(tmpRoot, { recursive: true });
      const dir = await fs.mkdtemp(path.join(tmpRoot, 'session-'));
      const tempPath = assertWorkspacePath(path.join(dir, 'manifest.json'));
      await fs.writeFile(tempPath, body.manifestJson, 'utf8');
      sendJson(res, 200, { path: tempPath });
      return;
    }

    if (action === 'remove-temp') {
      const target = body.path;
      if (typeof target === 'string' && target.includes('.skema-editor-tmp')) {
        await fs.rm(target, { force: true });
      }
      sendJson(res, 200, { ok: true });
      return;
    }

    sendJson(res, 404, { ok: false, textFailure: 'unknown action' });
  } catch (err) {
    sendJson(res, 500, { ok: false, textFailure: err instanceof Error ? err.message : String(err) });
  }
});

server.listen(port, () => {
  console.log(`skema bridge listening on http://localhost:${port} (binary: ${skemaBin})`);
});
