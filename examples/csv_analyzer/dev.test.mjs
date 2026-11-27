import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { createServer, connect } from 'node:net';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));

function launch(t, args = [], { pnpm = false, env = process.env } = {}) {
  const child = spawn(pnpm ? 'pnpm' : process.execPath, [
    ...(pnpm ? ['example:csv'] : ['examples/csv_analyzer/dev.mjs']), ...args,
  ], { cwd: root, detached: true, env: { ...env, PNPM_MAX_WORKERS: '1' }, stdio: ['ignore', 'pipe', 'pipe'] });
  let output = '';
  child.stdout.on('data', chunk => { output += chunk; });
  child.stderr.on('data', chunk => { output += chunk; });
  const completion = once(child, 'close').then(([code, signal]) => ({ code, signal }));
  completion.catch(() => {});
  t.after(async () => {
    try { process.kill(-child.pid, 'SIGTERM'); } catch (error) { if (error.code !== 'ESRCH') throw error; }
    const timer = setTimeout(() => {
      try { process.kill(-child.pid, 'SIGKILL'); } catch (error) { if (error.code !== 'ESRCH') throw error; }
    }, 3000);
    try { await completion; } finally { clearTimeout(timer); }
  });
  return {
    child, completion, output: () => output,
    async ready() {
      for (let attempt = 0; attempt < 600; attempt++) {
        const match = output.match(/CSV analyzer: (http:\/\/127\.0\.0\.1:\d+)/);
        if (match) return match[1];
        if (child.exitCode !== null || child.signalCode !== null) throw new Error(`Launcher exited before readiness. Fix startup:\n${output}`);
        await new Promise(resolveTick => setTimeout(resolveTick, 50));
      }
      throw new Error(`Launcher did not become ready. Inspect startup output:\n${output}`);
    },
  };
}

function backendAddress(output) {
  const record = output.split('\n').filter(line => line.startsWith('{')).map(line => JSON.parse(line)).find(record => record.message === 'server.listening');
  assert.ok(record, `Missing backend listening log:\n${output}`);
  return record.attributes.find(attribute => attribute.name === 'address').value;
}

async function closed(address) {
  const [host, port] = address.split(':');
  for (let attempt = 0; attempt < 60; attempt++) {
    const refused = await new Promise(resolveConnect => {
      const socket = connect(Number(port), host);
      socket.once('error', () => { socket.destroy(); resolveConnect(true); });
      socket.once('connect', () => { socket.destroy(); resolveConnect(false); });
    });
    if (refused) return;
    await new Promise(resolveTick => setTimeout(resolveTick, 50));
  }
  assert.fail(`Server ${address} remained reachable after launcher shutdown`);
}

async function occupied(t) {
  const server = createServer();
  await new Promise(resolveListen => server.listen(0, '127.0.0.1', resolveListen));
  t.after(() => server.close());
  return server.address().port;
}

test('pnpm command builds, serves the application, streams logs, and stops both servers on Ctrl-C', { timeout: 60000 }, async t => {
  const app = launch(t, ['--web-port', '0', '--backend-port', '0'], { pnpm: true });
  const url = await app.ready();
  assert.match(app.output(), /Building Veln CLI/);
  assert.match(await (await fetch(url)).text(), /Order desk/);
  const reply = await fetch(`${url}/api/analyze`, { method: 'POST', body: 'sku,quantity,unit_cents\ntea,2,250\n' });
  assert.equal(reply.status, 200);
  assert.equal((await reply.json()).total_cents, 500);
  const backend = backendAddress(app.output());
  // Signal the foreground group as the terminal does; the JVM has its own group.
  process.kill(-app.child.pid, 'SIGINT');
  await app.completion;
  await closed(new URL(url).host);
  await closed(backend);
  assert.match(app.output(), /request.completed/);
});

test('frontend bind failure cleans up the already-started backend', { timeout: 15000 }, async t => {
  const port = await occupied(t);
  const app = launch(t, ['--skip-build', '--web-port', String(port), '--backend-port', '0']);
  const result = await app.completion;
  assert.equal(result.code, 1, app.output());
  assert.match(app.output(), /EADDRINUSE/);
  await closed(backendAddress(app.output()));
});

test('backend bind failure exits without starting the frontend', { timeout: 15000 }, async t => {
  const port = await occupied(t);
  const app = launch(t, ['--skip-build', '--web-port', '0', '--backend-port', String(port)]);
  const result = await app.completion;
  assert.equal(result.code, 1, app.output());
  assert.match(app.output(), /Veln backend exited/);
  assert.doesNotMatch(app.output(), /CSV analyzer: http/);
});

test('invalid ports fail before build or server startup', { timeout: 5000 }, async t => {
  const app = launch(t, ['--web-port', '70000']);
  assert.equal((await app.completion).code, 1);
  assert.match(app.output(), /Ports must be integers/);
  assert.doesNotMatch(app.output(), /Building Veln CLI/);
});

test('missing Cargo reports how startup failed', { timeout: 5000 }, async t => {
  const app = launch(t, [], { env: { ...process.env, PATH: '' } });
  assert.equal((await app.completion).code, 1);
  assert.match(app.output(), /Could not start cargo/);
});

test('SIGTERM shuts down both servers when the CLI is already built', { timeout: 15000 }, async t => {
  const app = launch(t, ['--skip-build', '--web-port', '0', '--backend-port', '0']);
  const url = await app.ready();
  const backend = backendAddress(app.output());
  app.child.kill('SIGTERM');
  assert.equal((await app.completion).code, 143);
  await closed(new URL(url).host);
  await closed(backend);
});
