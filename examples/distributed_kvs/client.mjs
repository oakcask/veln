import { connect } from 'node:http2';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';

export function encode(message) {
  const key = Buffer.from(message.key ?? '', 'utf8');
  const value = Buffer.from(message.value ?? '', 'utf8');
  if (key.length > 4096 || value.length > 4096) throw new Error('Strings must fit in 4096 UTF-8 bytes each');
  const bytes = Buffer.alloc(58 + key.length + value.length);
  bytes.writeUInt16BE(19286, 0);
  bytes[2] = 1;
  bytes[3] = message.kind ?? 5;
  bytes.writeBigUInt64BE(BigInt(message.term ?? 0), 4);
  bytes[12] = message.node ?? 0;
  for (const [name, offset] of [['a', 13], ['b', 21], ['c', 29], ['entry_term', 37]]) {
    bytes.writeBigUInt64BE(BigInt(message[name] ?? 0), offset);
  }
  bytes[45] = message.op ?? 0;
  bytes.writeUInt32BE(message.client ?? 0, 46);
  bytes.writeUInt32BE(message.sequence ?? 0, 50);
  bytes.writeUInt16BE(key.length, 54);
  bytes.writeUInt16BE(value.length, 56);
  key.copy(bytes, 58);
  value.copy(bytes, 58 + key.length);
  return bytes;
}

export function decode(bytes) {
  if (bytes.length < 58 || bytes.readUInt16BE(0) !== 19286 || bytes[2] !== 1) throw new Error('Invalid packet header');
  const keySize = bytes.readUInt16BE(54);
  const valueSize = bytes.readUInt16BE(56);
  if (bytes.length !== 58 + keySize + valueSize || keySize > 4096 || valueSize > 4096) throw new Error('Invalid packet length');
  const text = new TextDecoder('utf-8', { fatal: true, ignoreBOM: true });
  return {
    kind: bytes[3], term: Number(bytes.readBigUInt64BE(4)), node: bytes[12],
    a: Number(bytes.readBigUInt64BE(13)), b: Number(bytes.readBigUInt64BE(21)),
    c: Number(bytes.readBigUInt64BE(29)), entry_term: Number(bytes.readBigUInt64BE(37)),
    op: bytes[45], client: bytes.readUInt32BE(46), sequence: bytes.readUInt32BE(50),
    key: text.decode(bytes.subarray(58, 58 + keySize)), value: text.decode(bytes.subarray(58 + keySize)),
  };
}

// One bounded request per connection matches the Veln transport's teaching scope.
export function request(port, message, { timeout = 4500, chunks, padding = false } = {}) {
  const bytes = encode(message);
  return rawRequest(port, chunks ?? [bytes], { timeout, padding });
}

export function rawRequest(port, chunks, { timeout = 4500, padding = false } = {}) {
  return new Promise((resolveReply, reject) => {
    const session = connect(`http://127.0.0.1:${port}`, padding ? { paddingStrategy: 2 } : {});
    let stream;
    let settled = false;
    const timer = setTimeout(() => finish(new Error('KVS request timed out')), timeout);
    const finish = (error, reply) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      stream?.destroy();
      session.destroy();
      error ? reject(error) : resolveReply(reply);
    };
    session.on('error', finish);
    session.on('close', () => finish(new Error('KVS connection closed before reply')));
    session.once('connect', () => {
      if (settled) return;
      stream = session.request({ ':method': 'POST', ':path': '/rpc', 'content-type': 'application/octet-stream' });
      const received = [];
      let size = 0;
      let status;
      stream.on('response', headers => { status = headers[':status']; });
      stream.on('error', finish);
      stream.on('data', chunk => {
        size += chunk.length;
        if (size > 8250) finish(new Error('KVS response exceeded limit'));
        else received.push(chunk);
      });
      stream.on('end', () => {
        try {
          if (status !== 200) throw new Error(`Unexpected HTTP status ${status}`);
          finish(null, decode(Buffer.concat(received)));
        } catch (error) { finish(error); }
      });
      stream.on('close', () => finish(new Error('KVS stream closed before reply')));
      for (const chunk of chunks) stream.write(chunk);
      stream.end();
    });
  });
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const [portText, operation, key = '', ...args] = process.argv.slice(2);
  const [value, clientText, sequenceText] = operation === 'put' ? args : ['', ...args];
  const port = Number(portText);
  const op = { put: 1, get: 2, delete: 3 }[operation];
  if (!Number.isInteger(port) || port < 1 || port > 65535 || (!op && operation !== 'status') ||
      (op && (!clientText || !sequenceText))) {
    throw new Error('Usage: node examples/distributed_kvs/client.mjs PORT status | PORT put KEY VALUE CLIENT_ID SEQUENCE | PORT get|delete KEY CLIENT_ID SEQUENCE');
  }
  const reply = await request(port, operation === 'status' ? { kind: 7 } : {
    op, key, value, client: Number(clientText), sequence: Number(sequenceText),
  });
  console.log(JSON.stringify(reply, null, 2));
  if (reply.kind === 6 && reply.a >= 500) process.exitCode = 1;
}
