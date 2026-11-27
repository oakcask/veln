import { createServer } from 'node:http';
import { connect } from 'node:http2';
import { readFile } from 'node:fs/promises';
import { stripTypeScriptTypes } from 'node:module';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';

export const MAX_BYTES = 8192;

// A fresh HTTP/2 session matches the example backend's one-request boundary.
export function analyze(body, backend, { timeout = 7000, signal } = {}) {
  return new Promise((resolveReply, reject) => {
    const session = connect(backend);
    let settled = false;
    let request;
    const finish = (error, value) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      signal?.removeEventListener('abort', abort);
      request?.destroy();
      session.destroy();
      if (error) reject(error);
      else resolveReply(value);
    };
    const abort = () => finish(new Error('Request cancelled'));
    const timer = setTimeout(() => finish(new Error('Backend timed out')), timeout);
    signal?.addEventListener('abort', abort, { once: true });
    session.on('error', error => finish(error));
    session.on('close', () => finish(new Error('Backend closed before responding')));
    if (signal?.aborted) return abort();
    session.once('connect', () => {
      if (settled) return;
      request = session.request({
        ':method': 'POST', ':path': '/api/analyze',
        'content-type': 'text/csv', 'content-length': String(body.length),
      });
      let status = 502;
      let size = 0;
      const chunks = [];
      request.on('response', headers => { status = Number(headers[':status']); });
      request.on('data', chunk => {
        size += chunk.length;
        if (size > 16384) finish(new Error('Backend response exceeded limit'));
        else chunks.push(chunk);
      });
      request.on('error', error => finish(error));
      request.on('end', () => finish(null, { status, body: Buffer.concat(chunks).toString('utf8') }));
      request.on('close', () => finish(new Error('Backend stream closed before responding')));
      request.end(body);
    });
  });
}

export function createWebServer({ backend = 'http://127.0.0.1:8082' } = {}) {
  return createServer(async (request, response) => {
    const controller = new AbortController();
    response.on('close', () => controller.abort());
    const json = (status, error) => {
      if (!response.destroyed) {
        response.writeHead(status, { 'content-type': 'application/json', 'cache-control': 'no-store' });
        response.end(JSON.stringify({ error }));
      }
    };
    try {
      if (request.url === '/api/analyze') {
        if (request.method !== 'POST') return json(405, 'method_not_allowed');
        let size = 0;
        const chunks = [];
        for await (const chunk of request) {
          size += chunk.length;
          if (size > MAX_BYTES) {
            json(413, 'csv_too_large');
            return;
          }
          chunks.push(chunk);
        }
        const reply = await analyze(Buffer.concat(chunks), backend, { signal: controller.signal });
        response.writeHead(reply.status, { 'content-type': 'application/json', 'cache-control': 'no-store' });
        response.end(reply.body);
        return;
      }
      if (request.method !== 'GET') return json(405, 'method_not_allowed');
      const assets = {
        '/': ['web/index.html', 'text/html; charset=utf-8'],
        '/app.js': ['web/app.ts', 'text/javascript; charset=utf-8'],
        '/style.css': ['web/style.css', 'text/css; charset=utf-8'],
        '/sample.csv': ['samples/order.csv', 'text/csv; charset=utf-8'],
      };
      const asset = assets[request.url];
      if (!asset) return json(404, 'not_found');
      let content = await readFile(new URL(asset[0], import.meta.url), 'utf8');
      if (request.url === '/app.js') content = stripTypeScriptTypes(content);
      response.writeHead(200, { 'content-type': asset[1], 'cache-control': 'no-store' });
      response.end(content);
    } catch (error) {
      console.error(`CSV example request failed: ${error.message}`);
      json(502, 'backend_unavailable');
    }
  });
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const port = Number(process.argv[2] ?? 8080);
  const backendPort = Number(process.argv[3] ?? 8082);
  if (![port, backendPort].every(value => Number.isInteger(value) && value >= 0 && value <= 65535)) {
    throw new Error('Usage: node examples/csv_analyzer/server.mjs [web-port] [backend-port]');
  }
  const server = createWebServer({ backend: `http://127.0.0.1:${backendPort}` });
  server.requestTimeout = 10000;
  server.listen(port, '127.0.0.1', () => console.log(`CSV analyzer: http://127.0.0.1:${server.address().port}`));
}
