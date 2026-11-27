import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtemp, open, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { parseArgs } from 'node:util';
import { StringDecoder } from 'node:string_decoder';
import { fileURLToPath } from 'node:url';
import { createWebServer } from './server.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));

function childProcess(command, args, options, signal) {
  const child = spawn(command, args, { ...options, detached: true });
  const completion = new Promise((resolveExit, reject) => {
    child.once('error', error => reject(new Error(`Could not start ${command}: ${error.message}`)));
    child.once('close', (code, killedBy) => resolveExit({ code, signal: killedBy }));
  });
  // Observe failures immediately, including during startup and teardown.
  completion.catch(() => {});
  const terminate = () => {
    if (!child.pid) return;
    try { process.kill(-child.pid, 'SIGTERM'); } catch (error) { if (error.code !== 'ESRCH') throw error; }
  };
  signal.addEventListener('abort', terminate, { once: true });
  if (signal.aborted) terminate();
  return {
    completion,
    async stop() {
      signal.removeEventListener('abort', terminate);
      terminate();
      const timer = setTimeout(() => {
        if (!child.pid) return;
        try { process.kill(-child.pid, 'SIGKILL'); } catch (error) { if (error.code !== 'ESRCH') throw error; }
      }, 2000);
      try { await completion.catch(() => {}); } finally {
        clearTimeout(timer);
        if (child.pid) {
          try { process.kill(-child.pid, 'SIGKILL'); } catch (error) { if (error.code !== 'ESRCH') throw error; }
        }
      }
    },
  };
}

async function pause(signal, milliseconds) {
  if (signal.aborted) return;
  await new Promise(resolvePause => {
    const done = () => { clearTimeout(timer); signal.removeEventListener('abort', done); resolvePause(); };
    const timer = setTimeout(done, milliseconds);
    signal.addEventListener('abort', done, { once: true });
  });
}

// Stream only this run's file. File export avoids veln run's captured stderr.
async function followLogs(path, signal, ready) {
  let file;
  let offset = 0;
  let pending = '';
  const decoder = new StringDecoder('utf8');
  try {
    while (true) {
      if (!file) {
        try { file = await open(path, 'r'); } catch (error) { if (error.code !== 'ENOENT') throw error; }
      }
      if (file) {
        const buffer = Buffer.alloc(16384);
        const { bytesRead } = await file.read(buffer, 0, buffer.length, offset);
        if (bytesRead > 0) {
          offset += bytesRead;
          pending += decoder.write(buffer.subarray(0, bytesRead));
          let newline;
          while ((newline = pending.indexOf('\n')) !== -1) {
            const line = pending.slice(0, newline);
            pending = pending.slice(newline + 1);
            process.stderr.write(`${line}\n`);
            const record = JSON.parse(line);
            if (record.message === 'server.listening') {
              const address = record.attributes.find(attribute => attribute.name === 'address')?.value;
              if (typeof address === 'string' && /^127\.0\.0\.1:\d+$/.test(address)) ready(`http://${address}`);
            }
          }
          continue;
        }
      }
      if (signal.aborted) break;
      await pause(signal, 50);
    }
  } finally {
    await file?.close();
  }
}

export async function run({ webPort = 8080, backendPort = 8082, skipBuild = false, signal }) {
  const controller = new AbortController();
  const abort = () => controller.abort();
  signal.addEventListener('abort', abort, { once: true });
  if (signal.aborted) abort();
  const children = [];
  let temporary;
  let following;
  let server;
  try {
    if (!skipBuild && !controller.signal.aborted) {
      console.log('Building Veln CLI…');
      const build = childProcess('cargo', ['build', '--locked', '-p', 'veln-cli'], { cwd: root, stdio: 'inherit' }, controller.signal);
      children.push(build);
      const result = await build.completion;
      await build.stop();
      children.pop();
      if (!controller.signal.aborted && result.code !== 0) throw new Error('CLI build failed. Fix the Cargo error above, then rerun pnpm example:csv.');
    }
    if (controller.signal.aborted) return;
    temporary = await mkdtemp(join(tmpdir(), 'veln-csv-dev-'));
    const logPath = join(temporary, 'backend.jsonl');
    let ready;
    const listening = new Promise(resolveReady => { ready = resolveReady; });
    following = followLogs(logPath, controller.signal, ready);
    following.catch(() => {});
    const backend = childProcess(join(root, 'target/debug/veln'), ['run', 'main_with_log_file', '.', '--', String(backendPort), logPath], {
      cwd: new URL('backend/', import.meta.url), stdio: 'inherit',
      env: { ...process.env, TMPDIR: temporary, VELN_CACHE_DIR: temporary },
    }, controller.signal);
    children.push(backend);
    const backendExited = backend.completion.then(result => {
      if (!controller.signal.aborted) throw new Error(`Veln backend exited (${result.code ?? result.signal}). Inspect the logs above and check the backend port.`);
    });
    backendExited.catch(() => {});
    const interrupted = new Promise(resolveAbort => controller.signal.addEventListener('abort', () => resolveAbort(null), { once: true }));
    const timer = setTimeout(() => controller.abort(new Error('Backend startup timed out')), 45000);
    let endpoint;
    try {
      endpoint = await Promise.race([listening, backendExited, following, interrupted]);
    } finally {
      clearTimeout(timer);
    }
    if (controller.signal.aborted) {
      if (!signal.aborted) throw new Error('Backend did not become ready within 45 seconds. Inspect the logs and rerun pnpm example:csv.');
      return;
    }
    if (!endpoint) return;
    server = createWebServer({ backend: endpoint });
    server.requestTimeout = 10000;
    const webFailed = new Promise((_, reject) => server.on('error', reject));
    webFailed.catch(() => {});
    server.listen(webPort, '127.0.0.1');
    await once(server, 'listening');
    console.log(`CSV analyzer: http://127.0.0.1:${server.address().port}`);
    console.log('Veln JSON logs stream below. Press Ctrl-C to stop both servers.');
    await Promise.race([backendExited, following, interrupted, webFailed]);
  } finally {
    controller.abort();
    signal.removeEventListener('abort', abort);
    server?.closeAllConnections();
    if (server?.listening) await new Promise(resolveClose => server.close(resolveClose));
    await Promise.all(children.map(child => child.stop()));
    await following?.catch(() => {});
    if (temporary) await rm(temporary, { recursive: true, force: true });
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const controller = new AbortController();
  const interrupt = () => { process.exitCode = 130; controller.abort(); };
  const terminate = () => { process.exitCode = 143; controller.abort(); };
  process.on('SIGINT', interrupt);
  process.on('SIGTERM', terminate);
  try {
    const { values } = parseArgs({ options: {
      'web-port': { type: 'string', default: '8080' },
      'backend-port': { type: 'string', default: '8082' },
      'skip-build': { type: 'boolean', default: false },
    } });
    const webPort = Number(values['web-port']);
    const backendPort = Number(values['backend-port']);
    if (![webPort, backendPort].every(port => Number.isInteger(port) && port >= 0 && port <= 65535)) {
      throw new Error('Ports must be integers from 0 to 65535. Use pnpm example:csv --web-port 8090 --backend-port 8092.');
    }
    await run({ webPort, backendPort, skipBuild: values['skip-build'], signal: controller.signal });
  } catch (error) {
    if (!controller.signal.aborted) { console.error(error.message); process.exitCode = 1; }
  } finally {
    process.removeListener('SIGINT', interrupt);
    process.removeListener('SIGTERM', terminate);
  }
}
