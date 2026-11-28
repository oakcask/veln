import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdir } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
const cwd = fileURLToPath(new URL('./veln/', import.meta.url));
const base = Number(process.argv[2] ?? 19000);
const data = resolve(process.argv[3] ?? join(cwd, '../.data'));
if (!Number.isInteger(base) || base < 1024 || base > 65532) throw new Error('Usage: node examples/distributed_kvs/run.mjs [BASE_PORT] [DATA_DIRECTORY]');
const children = new Set();
let stopping = false;
function signalAll(signal) {
  for (const child of children) {
    try { process.kill(-child.pid, signal); } catch (error) { if (error.code !== 'ESRCH') throw error; }
  }
}
let timer;
function stop() {
  if (stopping) return;
  stopping = true;
  signalAll('SIGTERM');
  timer = setTimeout(() => signalAll('SIGKILL'), 2000);
  timer.unref();
}
process.once('SIGINT', stop);
process.once('SIGTERM', stop);
function launch(command, args, options) {
  const child = spawn(command, args, { detached: true, stdio: 'inherit', ...options });
  children.add(child);
  child.once('error', error => { console.error(error.message); children.delete(child); process.exitCode = 1; stop(); });
  child.once('exit', () => children.delete(child));
  return child;
}
try {
  if (!process.env.VELN_BIN) {
    const build = launch('cargo', ['build', '--locked', '-p', 'veln-cli'], { cwd: root });
    const [code] = await once(build, 'exit');
    if (!stopping && code !== 0) throw new Error('Build failed. Fix the Cargo error above and run the example again.');
  }
  if (!stopping) {
    const binary = resolve(process.env.VELN_BIN ?? join(root, 'target/debug/veln'));
    for (let id = 1; id <= 3; id++) {
      const directory = join(data, `node${id}`);
      await mkdir(directory, { recursive: true });
      if (stopping) break;
      const child = launch(binary, ['run', 'main', '.', '--', String(id), String(base), directory], { cwd });
      console.log(`Node ${id}: http://127.0.0.1:${base + id}, process group ${child.pid}`);
      child.once('exit', (code, signal) => {
        if (!stopping) console.error(`Node ${id} stopped (${signal ?? code}); the other nodes remain running.`);
      });
    }
    console.log('Wait for election; use client.mjs to issue operations. Ctrl-C stops the remaining nodes.');
    // Keep the launcher alive while each independent node runs.
    await Promise.all([...children].map(child => once(child, 'exit')));
  }
} finally {
  stop();
  if (children.size === 0) clearTimeout(timer);
}
