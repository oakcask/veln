import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { connect, createServer as createHttp2Server } from 'node:http2';
import { createServer, connect as connectTcp } from 'node:net';
import { stripTypeScriptTypes } from 'node:module';
import { Script, createContext } from 'node:vm';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { analyze, createWebServer } from './server.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const sample = 'sku,quantity,unit_cents\ntea,2,250\ncake,1,400\n';
const expected = { rows: 2, accepted: 2, quantity: 3, total_cents: 900, errors: [] };

async function startBackend(t) {
  const cache = await mkdtemp(join(tmpdir(), 'veln-csv-'));
  const binary = resolve(process.env.VELN_BIN ?? join(root, 'target/debug/veln'));
  const reservation = createServer();
  await new Promise(resolveListen => reservation.listen(0, '127.0.0.1', resolveListen));
  const port = reservation.address().port;
  await new Promise(resolveClose => reservation.close(resolveClose));
  const logPath = join(cache, 'backend.jsonl');
  const child = spawn(binary, ['run', 'main_with_log_file', '.', '--', String(port), logPath], {
    cwd: new URL('backend/', import.meta.url), detached: true,
    env: { ...process.env, VELN_CACHE_DIR: cache, TMPDIR: cache }, stdio: ['ignore', 'pipe', 'pipe'],
  });
  let output = '';
  let spawnError;
  child.once('error', error => { spawnError = error; });
  child.stdout.on('data', chunk => { output += chunk; });
  child.stderr.on('data', chunk => { output += chunk; });
  child.on('exit', code => { if (code) console.error(output.slice(-6000)); });
  t.after(async () => {
    const exited = child.exitCode === null && !spawnError ? once(child, 'exit') : null;
    try { process.kill(-child.pid, 'SIGKILL'); } catch (error) { if (error.code !== 'ESRCH') throw error; }
    if (exited) await exited;
    await rm(cache, { recursive: true, force: true });
  });
  // veln run captures stdout until exit, so readiness uses the actual API.
  const backend = `http://127.0.0.1:${port}`;
  const started = Date.now();
  let lastError;
  while (Date.now() - started < 45000) {
    if (spawnError) throw spawnError;
    if (child.exitCode !== null) throw new Error(`Veln backend exited (${child.exitCode}). Fix the example against the current toolchain:\n${output}`);
    try {
      const reply = await analyze(Buffer.from(sample), backend);
      assert.equal(reply.status, 200);
      return { backend, logPath };
    } catch (error) {
      lastError = error;
      await new Promise(resolveRetry => setTimeout(resolveRetry, 100));
    }
  }
  throw new Error(`Veln backend did not serve a valid response (${lastError?.message}). Inspect compiler/runtime output:\n${output}`);
}

function direct(backend, { method = 'POST', path = '/api/analyze', chunks = [Buffer.from(sample)], pad = false } = {}) {
  return new Promise((resolveReply, reject) => {
    const session = connect(backend, pad ? { paddingStrategy: 2 } : {});
    const timer = setTimeout(() => finish(new Error('Direct HTTP/2 request timed out')), 8000);
    let request;
    let settled = false;
    const finish = (error, reply) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      request?.destroy();
      session.destroy();
      error ? reject(error) : resolveReply(reply);
    };
    session.on('error', finish);
    request = session.request({ ':method': method, ':path': path }, { endStream: false });
    let status;
    let body = '';
    request.on('response', headers => { status = headers[':status']; });
    request.setEncoding('utf8');
    request.on('data', chunk => { body += chunk; });
    request.on('error', finish);
    request.on('end', () => {
      try { finish(null, { status, body: JSON.parse(body) }); }
      catch (error) { finish(new Error(`Invalid HTTP/2 JSON response (status ${status}): ${error.message}`)); }
    });
    void (async () => {
      for (const chunk of chunks) {
        request.write(chunk);
        await new Promise(resolveTick => setTimeout(resolveTick, 5));
      }
      request.end();
    })();
  });
}

test('real Veln HTTP/2 backend and browser-facing server', { timeout: 100000 }, async t => {
  const { backend, logPath } = await startBackend(t);
  await t.test('valid CSV and repeated independent connections', async () => {
    for (let i = 0; i < 3; i++) {
      const reply = await analyze(Buffer.from(sample), backend);
      assert.equal(reply.status, 200);
      assert.deepEqual(JSON.parse(reply.body), expected);
    }
  });
  await t.test('fragmented body, CRLF, padded DATA and final row without newline', async () => {
    const csv = sample.replaceAll('\n', '\r\n').trimEnd();
    const chunks = [...Buffer.from(csv)].map(byte => Buffer.from([byte]));
    const reply = await direct(backend, { chunks, pad: true });
    assert.equal(reply.status, 200);
    assert.deepEqual(reply.body, expected);
  });
  await t.test('all row failures are collected while valid rows contribute totals', async () => {
    const csv = 'sku,quantity,unit_cents\ntea,2,250\nx,0,1\n,2,3\nx,2,-1\nbroken\n"x",1,2\nx,1,2,3\nx,no,2\n';
    const reply = await analyze(Buffer.from(csv), backend);
    assert.equal(reply.status, 200);
    assert.deepEqual(JSON.parse(reply.body), {
      rows: 8, accepted: 1, quantity: 2, total_cents: 500,
      errors: [
        { line: 3, code: 'invalid_quantity' }, { line: 4, code: 'missing_sku' },
        { line: 5, code: 'invalid_unit_cents' }, { line: 6, code: 'expected_three_columns' },
        { line: 7, code: 'quoted_fields_unsupported' }, { line: 8, code: 'expected_three_columns' },
        { line: 9, code: 'invalid_quantity' },
      ],
    });
  });
  await t.test('request errors and bounds do not kill the listener', async () => {
    const cases = [
      ['', 400, 'expected_header'], ['wrong\n', 400, 'expected_header'],
      ['sku,quantity,unit_cents\nx,1,2\r', 400, 'invalid_line_ending'],
      ['sku,quantity,unit_cents\n茶,1,2', 400, 'expected_ascii_csv'],
      [`sku,quantity,unit_cents\n${'x,1,2\n'.repeat(101)}`, 400, 'too_many_rows'],
      [`sku,quantity,unit_cents\n${'x'.repeat(8192)}`, 413, 'csv_too_large'],
    ];
    for (const [csv, status, error] of cases) {
      const reply = await direct(backend, { chunks: [Buffer.from(csv)] });
      assert.deepEqual(reply, { status, body: { error } });
    }
    assert.equal((await direct(backend, { method: 'GET', chunks: [] })).status, 405);
    assert.equal((await direct(backend, { path: '/missing', chunks: [] })).status, 404);
    const maximum = `sku,quantity,unit_cents\n${'x,1000000,1000000\n'.repeat(100)}`;
    const reply = await direct(backend, { chunks: [Buffer.from(maximum)] });
    assert.deepEqual(reply.body, { rows: 100, accepted: 100, quantity: 100000000, total_cents: 100000000000000, errors: [] });
    const exactly = `sku,quantity,unit_cents\n${'x'.repeat(8192 - Buffer.byteLength('sku,quantity,unit_cents\n') - 4)},1,2`;
    assert.equal(Buffer.byteLength(exactly), 8192);
    assert.equal((await direct(backend, { chunks: [Buffer.from(exactly)] })).status, 200);
    assert.deepEqual((await direct(backend)).body, expected);
  });
  await t.test('malformed and idle connections are closed without losing the listener', async () => {
    const endpoint = new URL(backend);
    const malformed = connectTcp(Number(endpoint.port), endpoint.hostname);
    malformed.on('error', () => {});
    malformed.resume();
    t.after(() => malformed.destroy());
    await once(malformed, 'connect');
    const closed = once(malformed, 'close');
    malformed.end(Buffer.alloc(24, 120));
    await closed;
    const idle = connectTcp(Number(endpoint.port), endpoint.hostname);
    idle.on('error', () => {});
    idle.resume();
    t.after(() => idle.destroy());
    await once(idle, 'connect');
    await once(idle, 'close');
    assert.deepEqual((await direct(backend)).body, expected);
  });
  await t.test('web assets, POST proxy, size rejection and unavailable backend', async () => {
    const server = createWebServer({ backend });
    await new Promise(resolveListen => server.listen(0, '127.0.0.1', resolveListen));
    t.after(() => { server.closeAllConnections(); server.close(); });
    const url = `http://127.0.0.1:${server.address().port}`;
    const html = await (await fetch(url)).text();
    assert.match(html, /id="csv"/);
    const js = await (await fetch(`${url}/app.js`)).text();
    new Script(js);
    assert.match(await (await fetch(`${url}/style.css`)).text(), /focus-visible/);
    assert.match(await (await fetch(`${url}/sample.csv`)).text(), /sku,quantity,unit_cents/);
    const reply = await fetch(`${url}/api/analyze`, { method: 'POST', body: sample });
    assert.equal(reply.status, 200);
    assert.deepEqual(await reply.json(), expected);
    assert.equal((await fetch(`${url}/api/analyze`, { method: 'POST', body: 'x'.repeat(8193) })).status, 413);
    assert.equal((await fetch(`${url}/api/analyze`)).status, 405);
    assert.equal((await fetch(`${url}/server.mjs`)).status, 404);
    const broken = createWebServer({ backend: 'http://127.0.0.1:1' });
    await new Promise(resolveListen => broken.listen(0, '127.0.0.1', resolveListen));
    t.after(() => { broken.closeAllConnections(); broken.close(); });
    const failure = await fetch(`http://127.0.0.1:${broken.address().port}/api/analyze`, { method: 'POST', body: sample });
    assert.equal(failure.status, 502);
    assert.deepEqual(await failure.json(), { error: 'backend_unavailable' });
  });
  await t.test('log/observe emits live structured records without CSV contents', async () => {
    const previous = (await readFile(logPath, 'utf8')).trim().split('\n').length;
    const marker = 'private-csv-sku';
    const reply = await analyze(Buffer.from(`sku,quantity,unit_cents\n${marker},1,123\n`), backend);
    assert.equal(reply.status, 200);
    let text;
    let records;
    for (let attempt = 0; attempt < 30; attempt++) {
      text = await readFile(logPath, 'utf8');
      records = text.trim().split('\n').map(line => JSON.parse(line));
      if (records.length > previous) break;
      await new Promise(resolveTick => setTimeout(resolveTick, 10));
    }
    assert.ok(records.length > previous, 'The completed request must be exported while the backend is running');
    const attributes = record => Object.fromEntries(record.attributes.map(attribute => [attribute.name, attribute.value]));
    assert.equal(records[0].message, 'server.listening');
    assert.equal(records[0].severity, 'info');
    assert.match(attributes(records[0]).address, /^127\.0\.0\.1:\d+$/);
    for (const [index, record] of records.entries()) {
      assert.equal(record.kind, 'log');
      assert.equal(record.sequence, index + 1);
      assert.equal(record.resource.name, 'csv-analyzer');
      assert.equal(record.instrumentation_scope.name, 'csv-analyzer.backend');
      assert.equal(record.exporter_health.failed_exports, 0);
      assert.equal(record.source.file, 'server.veln');
      assert.ok(record.source.start_line > 0);
      assert.deepEqual(record.attribute_facts, { dropped: 0, truncated: 0, redacted: 0 });
    }
    const completed = records.filter(record => record.message === 'request.completed');
    for (const status of [200, 400, 404, 405, 413]) {
      const record = completed.find(candidate => attributes(candidate).status === status);
      assert.ok(record, `Missing request.completed log for HTTP ${status}`);
      assert.equal(record.severity, status >= 400 ? 'warning' : 'info');
      assert.equal(attributes(record).stream_id, 1);
      assert.ok(attributes(record).response_bytes > 0);
      assert.ok(record.attributes.every(attribute => attribute.type === 'int'));
    }
    const rejected = records.filter(record => record.message === 'connection.rejected');
    assert.ok(rejected.length >= 2, 'Malformed and idle connections must emit warnings');
    assert.ok(rejected.every(record => record.severity === 'warning' && attributes(record).reason.length > 0));
    assert.equal(text.includes(marker), false, 'CSV contents must stay out of logs');
  });
});

// Execute the served TypeScript as browser JavaScript with a small DOM boundary.
// This checks user actions and rendering without a second application framework.
function browser(source, fetchImpl) {
  const nodes = new Map();
  function element() {
    return { textContent: '', value: sample, hidden: true, disabled: false, files: [], children: [], handlers: {},
      addEventListener(name, handler) { this.handlers[name] = handler; },
      replaceChildren() { this.children = []; }, append(child) { this.children.push(child); },
    };
  }
  for (const id of ['analyze', 'csv', 'file', 'submit', 'status', 'results', 'errors', 'sample', 'rows', 'accepted', 'quantity', 'total']) nodes.set(id, element());
  const document = { querySelector: selector => nodes.get(selector.slice(1)), getElementById: id => nodes.get(id), createElement: element };
  const context = createContext({ document, fetch: fetchImpl, TextEncoder, AbortSignal });
  new Script(stripTypeScriptTypes(source)).runInContext(context);
  return nodes;
}

test('TypeScript UI submits CSV, renders totals and row issues, and recovers from failures', async () => {
  const source = await readFile(new URL('web/app.ts', import.meta.url), 'utf8');
  let reply = { ok: true, json: async () => ({ ...expected, errors: [{ line: 4, code: 'missing_sku' }] }) };
  const nodes = browser(source, async (url, options) => {
    assert.equal(url, '/api/analyze'); assert.equal(options.method, 'POST'); assert.equal(options.body, sample);
    return reply;
  });
  const submit = () => nodes.get('analyze').handlers.submit({ preventDefault() {} });
  await submit();
  assert.equal(nodes.get('total').textContent, '9.00');
  assert.equal(nodes.get('accepted').textContent, '2');
  assert.equal(nodes.get('results').hidden, false);
  assert.equal(nodes.get('errors').children[0].children[1].textContent, 'SKU is required.');
  reply = { ok: false, status: 502, json: async () => ({ error: 'backend_unavailable' }) };
  await submit();
  assert.equal(nodes.get('results').hidden, true);
  assert.match(nodes.get('status').textContent, /Check that it is running/);
  assert.equal(nodes.get('submit').disabled, false);
  nodes.get('csv').value = 'x'.repeat(8193);
  await submit();
  assert.match(nodes.get('status').textContent, /8 KiB/);
  nodes.get('file').files = [{ size: 30, text: async () => 'new CSV' }];
  await nodes.get('file').handlers.change();
  assert.equal(nodes.get('csv').value, 'new CSV');
});

test('HTTP/2 proxy tears down timed-out and cancelled sessions', async t => {
  const server = createHttp2Server();
  const sessions = new Set();
  server.on('session', session => {
    sessions.add(session);
    session.on('close', () => sessions.delete(session));
  });
  server.on('stream', stream => { stream.on('error', () => {}); });
  await new Promise(resolveListen => server.listen(0, '127.0.0.1', resolveListen));
  t.after(() => { for (const session of sessions) session.destroy(); server.close(); });
  const endpoint = `http://127.0.0.1:${server.address().port}`;
  await assert.rejects(analyze(Buffer.from(sample), endpoint, { timeout: 50 }), /timed out/);
  const controller = new AbortController();
  const cancelled = analyze(Buffer.from(sample), endpoint, { signal: controller.signal });
  controller.abort();
  await assert.rejects(cancelled, /cancelled/);
  await assert.rejects(analyze(Buffer.from(sample), endpoint, { signal: controller.signal }), /cancelled/);
  for (let attempt = 0; sessions.size > 0 && attempt < 20; attempt++) await new Promise(resolveTick => setTimeout(resolveTick, 10));
  assert.equal(sessions.size, 0, 'timed-out and cancelled sessions must release the server connection');
});

test('editing input while analysis is pending discards the old result', async () => {
  const source = await readFile(new URL('web/app.ts', import.meta.url), 'utf8');
  let resolveRequest;
  const pending = new Promise(resolvePending => { resolveRequest = resolvePending; });
  const nodes = browser(source, () => pending);
  const submitted = nodes.get('analyze').handlers.submit({ preventDefault() {} });
  nodes.get('csv').value = 'different input';
  nodes.get('csv').handlers.input();
  resolveRequest({ ok: true, json: async () => expected });
  await submitted;
  assert.equal(nodes.get('results').hidden, true);
  assert.equal(nodes.get('status').textContent, '');
  assert.equal(nodes.get('submit').disabled, false);
});

test('default entry exports startup failures as structured stderr logs', { timeout: 45000 }, async t => {
  const cache = await mkdtemp(join(tmpdir(), 'veln-csv-log-'));
  const binary = resolve(process.env.VELN_BIN ?? join(root, 'target/debug/veln'));
  const child = spawn(binary, ['run', 'main', '.', '--', '-1'], {
    cwd: new URL('backend/', import.meta.url), detached: true,
    env: { ...process.env, VELN_CACHE_DIR: cache, TMPDIR: cache }, stdio: ['ignore', 'pipe', 'pipe'],
  });
  t.after(async () => {
    if (child.exitCode === null) {
      const closed = once(child, 'close');
      try { process.kill(-child.pid, 'SIGKILL'); } catch (error) { if (error.code !== 'ESRCH') throw error; }
      await closed;
    }
    await rm(cache, { recursive: true, force: true });
  });
  let stderr = '';
  child.stdout.resume();
  child.stderr.on('data', chunk => { stderr += chunk; });
  const [code] = await once(child, 'close');
  assert.equal(code, 1);
  const records = stderr.split('\n').filter(line => line.startsWith('{')).map(line => JSON.parse(line));
  const failure = records.find(record => record.kind === 'log' && record.message === 'server.failed');
  assert.ok(failure, `Missing structured startup failure in stderr: ${stderr}`);
  assert.equal(failure.severity, 'error');
  assert.equal(failure.resource.name, 'csv-analyzer');
  assert.ok(failure.attributes.some(attribute => attribute.name === 'reason' && attribute.value.length > 0));
});
