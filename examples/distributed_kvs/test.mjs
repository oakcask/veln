import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtemp, mkdir, readdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { createServer } from 'node:net';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { encode, decode, request, rawRequest } from './client.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const cwd = fileURLToPath(new URL('./', import.meta.url));
const pause = ms => new Promise(resolvePause => setTimeout(resolvePause, ms));

async function eventually(action, label, timeout = 15000) {
  const end = Date.now() + timeout;
  let last;
  while (Date.now() < end) {
    try { return await action(); } catch (error) { last = error; }
    await pause(100);
  }
  throw new Error(`${label}: ${last?.stack}`);
}

async function reservePorts() {
  for (let attempt = 0; attempt < 20; attempt++) {
    const servers = [];
    try {
      const first = createServer();
      servers.push(first);
      await new Promise((ok, bad) => { first.once('error', bad); first.listen(0, '127.0.0.1', ok); });
      const base = first.address().port - 1;
      if (base > 65532) continue;
      for (let id = 2; id <= 3; id++) {
        const server = createServer();
        servers.push(server);
        await new Promise((ok, bad) => { server.once('error', bad); server.listen(base + id, '127.0.0.1', ok); });
      }
      return base;
    } catch (error) { if (error.code !== 'EADDRINUSE') throw error; }
    finally { await Promise.all(servers.map(server => new Promise(ok => server.close(ok)))); }
  }
  throw new Error('Could not reserve three consecutive KVS ports');
}

async function cluster(t) {
  const directory = await mkdtemp(join(tmpdir(), 'veln-kvs-'));
  const base = await reservePorts();
  const binary = resolve(process.env.VELN_BIN ?? join(root, 'target/debug/veln'));
  const children = new Map();
  const output = new Map();
  const data = id => join(directory, `node${id}`);
  async function start(id) {
    await mkdir(data(id), { recursive: true });
    const child = spawn(binary, ['run', 'main', '.', '--', String(id), String(base), data(id)], {
      cwd, detached: true, stdio: ['ignore', 'pipe', 'pipe'],
      env: { ...process.env, TMPDIR: directory, VELN_CACHE_DIR: join(directory, `cache${id}`) },
    });
    let text = '';
    child.stdout.on('data', chunk => { text = (text + chunk).slice(-12000); output.set(id, text); });
    child.stderr.on('data', chunk => { text = (text + chunk).slice(-12000); output.set(id, text); });
    children.set(id, child);
    child.once('error', error => { output.set(id, error.stack); });
    await eventually(async () => {
      if (child.exitCode !== null || child.signalCode !== null) throw new Error(`Node ${id} exited: ${output.get(id)}`);
      assert.equal((await request(base + id, { kind: 7 }, { timeout: 800 })).kind, 8);
    }, `Node ${id} startup`, 45000);
  }
  async function stop(id) {
    const child = children.get(id);
    if (!child || child.exitCode !== null || child.signalCode !== null) return;
    const exited = once(child, 'close');
    process.kill(-child.pid, 'SIGKILL');
    await exited;
  }
  function signal(id, name) { process.kill(-children.get(id).pid, name); }
  async function leader(ids = [1, 2, 3]) {
    return eventually(async () => {
      const statuses = await Promise.all(ids.map(id => request(base + id, { kind: 7 }, { timeout: 1200 })));
      const candidates = statuses.filter(status => status.a === 2);
      assert.equal(candidates.length, 1, `Expected one leader: ${JSON.stringify(statuses)}`);
      assert.ok(candidates[0].c > 0, 'Leader must commit its term entry');
      return candidates[0].node;
    }, 'Leader election');
  }
  let sequence = 0;
  async function operation(id, op, key, value = '', identity) {
    const message = { op, key, value, client: 17, sequence: ++sequence, ...identity };
    return eventually(async () => {
      const reply = await request(base + id, message);
      assert.equal(reply.kind, 6);
      assert.ok(reply.a === 200 || reply.a === 404, `Operation unavailable: ${JSON.stringify(reply)}`);
      return reply;
    }, `Operation ${op} through node ${id}`);
  }
  t.after(async () => {
    await Promise.all([1, 2, 3].map(stop));
    await rm(directory, { recursive: true, force: true });
  });
  await Promise.all([1, 2, 3].map(start));
  return { base, data, start, stop, signal, leader, operation, output };
}

// Independent wire fixture catches schema/JavaScript layout disagreement.
test('binary envelope preserves UTF-8 including NUL and rejects extra bytes', () => {
  const message = { kind: 5, op: 1, client: 7, sequence: 1, key: '茶', value: '\0🍵' };
  const bytes = encode(message);
  assert.equal(bytes.toString('hex'), '4b560105000000000000000000000000000000000000000000000000000000000000000000000000000000000001000000070000000100030005e88cb600f09f8db5');
  assert.deepEqual({ ...decode(bytes), term: undefined, node: undefined, a: undefined, b: undefined, c: undefined, entry_term: undefined },
    { ...message, term: undefined, node: undefined, a: undefined, b: undefined, c: undefined, entry_term: undefined });
  assert.throws(() => decode(Buffer.concat([bytes, Buffer.from([0])])), /length/);
});

test('three real Veln nodes: linearizable operations, failover and process recovery', { timeout: 180000 }, async t => {
  const c = await cluster(t);
  await c.leader();
  await t.test('all entrances return the last completed write, with Unicode and empty strings', async () => {
    for (let id = 1; id <= 3; id++) {
      const value = `node${id}: 🍵\n\0\ufeff`;
      assert.equal((await c.operation(id, 1, '茶/鍵', value)).a, 200);
      for (let reader = 1; reader <= 3; reader++) assert.equal((await c.operation(reader, 2, '茶/鍵')).value, value);
    }
    await c.operation(2, 1, '', '');
    assert.equal((await c.operation(3, 2, '')).a, 200);
    await c.operation(1, 3, '');
    assert.equal((await c.operation(2, 2, '')).a, 404);
  });
  await t.test('retry identity prevents a duplicate write from reverting newer data', async () => {
    await c.operation(1, 1, 'retry', 'first', { client: 900, sequence: 1 });
    await c.operation(2, 1, 'retry', 'second', { client: 900, sequence: 2 });
    await c.operation(3, 1, 'retry', 'first', { client: 900, sequence: 1 });
    assert.equal((await c.operation(1, 2, 'retry')).value, 'second');
  });
  await t.test('fragmented padded binary DATA, boundary lengths and malformed packets', async () => {
    const bytes = encode({ op: 1, key: 'fragmented', value: '日本語', client: 30, sequence: 1 });
    const chunks = [...bytes].map(octet => Buffer.from([octet]));
    assert.equal((await rawRequest(c.base + 2, chunks, { padding: true })).a, 200);
    assert.equal((await c.operation(3, 2, 'fragmented')).value, '日本語');
    const key = 'k'.repeat(4096);
    const value = '🍵'.repeat(1024);
    await c.operation(1, 1, key, value);
    assert.equal((await c.operation(2, 2, key)).value, value);
    const invalid = encode({ kind: 5, op: 1, key: 'x', client: 30, sequence: 2 });
    invalid[58] = 0x80;
    for (const bad of [invalid, bytes.subarray(0, 20), Buffer.concat([bytes, Buffer.from([0])])]) {
      await assert.rejects(rawRequest(c.base + 3, [bad], { timeout: 4500 }));
    }
    assert.equal((await c.operation(3, 2, 'fragmented')).value, '日本語');
  });
  await t.test('a paused leader cannot serve stale reads after the other two commit', async () => {
    const old = await c.leader();
    const survivors = [1, 2, 3].filter(id => id !== old);
    c.signal(old, 'SIGSTOP');
    try {
      await c.leader(survivors);
      await c.operation(survivors[0], 1, 'failover', 'new-term');
      assert.equal((await c.operation(survivors[1], 2, 'failover')).value, 'new-term');
    } finally { c.signal(old, 'SIGCONT'); }
    const reply = await request(c.base + old, { op: 2, key: 'failover', client: 40, sequence: 1 });
    assert.ok(reply.a === 503 || (reply.a === 200 && reply.value === 'new-term'), `Old leader returned stale data: ${JSON.stringify(reply)}`);
    assert.equal((await c.operation(old, 2, 'failover')).value, 'new-term');
  });
  await t.test('kill the leader, continue on two nodes, then recover its durable state', async () => {
    const old = await c.leader();
    const survivors = [1, 2, 3].filter(id => id !== old);
    await c.stop(old);
    await c.leader(survivors);
    await c.operation(survivors[0], 1, 'after-crash', 'survives');
    assert.equal((await c.operation(survivors[1], 2, 'after-crash')).value, 'survives');
    // Model a partial, unacknowledged final filesystem write.
    const records = await readdir(c.data(old));
    const serial = Math.max(...records.filter(name => /^record-\d+\.hex$/.test(name)).map(name => Number(name.match(/\d+/)[0]))) + 1;
    await writeFile(join(c.data(old), `record-${serial}.hex`), '4b5601');
    await c.start(old);
    assert.equal((await c.operation(old, 2, 'after-crash')).value, 'survives');
    await eventually(async () => {
      const [current, restored] = await Promise.all([request(c.base + survivors[0], { kind: 7 }), request(c.base + old, { kind: 7 })]);
      assert.ok(restored.c >= current.c, 'Recovered follower has not applied the prefix');
    }, 'Restarted follower catch-up');
  });
  await t.test('concurrent completed operations admit a linearizable history', async () => {
    await c.operation(1, 1, 'history', 'initial');
    let clock = 0;
    const history = [];
    async function perform(id, op, value = '') {
      const item = { op, value, start: ++clock };
      const reply = await c.operation(id, op, 'history', value);
      item.end = ++clock;
      item.result = reply.value;
      history.push(item);
    }
    await Promise.all([perform(1, 1, 'a'), perform(2, 2), perform(3, 1, 'b'), perform(1, 2)]);
    await perform(2, 2);
    function accepts(remaining, value) {
      if (!remaining.length) return true;
      return remaining.some(item => {
        if (remaining.some(other => other.end < item.start)) return false;
        if (item.op === 2 && item.result !== value) return false;
        return accepts(remaining.filter(other => other !== item), item.op === 1 ? item.value : value);
      });
    }
    assert.ok(accepts(history, 'initial'), `Nonlinearizable history: ${JSON.stringify(history)}`);
  });
  await t.test('all processes can restart from their journals without losing completed operations', async () => {
    await Promise.all([1, 2, 3].map(c.stop));
    await Promise.all([1, 2, 3].map(c.start));
    await c.leader();
    for (const id of [1, 2, 3]) assert.equal((await c.operation(id, 2, 'after-crash')).value, 'survives');
    await c.operation(3, 1, 'retry', 'first', { client: 900, sequence: 1 });
    assert.equal((await c.operation(2, 2, 'retry')).value, 'second');
  });
  await t.test('loss of the majority rejects both reads and writes', async () => {
    const leader = await c.leader();
    for (const id of [1, 2, 3].filter(id => id !== leader)) await c.stop(id);
    for (const op of [1, 2]) {
      const reply = await request(c.base + leader, { op, key: 'after-crash', value: 'uncommitted', client: 50, sequence: op });
      assert.equal(reply.a, 503);
    }
  });
});
