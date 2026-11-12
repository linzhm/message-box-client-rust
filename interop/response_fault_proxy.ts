// Test-only fault injector, not a MessageBox implementation or deployment proxy.
// Forwards unchanged requests to the real pinned TS harness on loopback.
import { createServer, request } from 'node:http'
let sends = 0
let faults = 0
let upstreamStatus: number | undefined
const server = createServer((incoming, outgoing) => {
  if (incoming.method === 'GET' && incoming.url === '/_test/counts') {
    const body = JSON.stringify({ sends, faults, upstreamStatus })
    outgoing.writeHead(200, { 'content-type': 'application/json', 'content-length': Buffer.byteLength(body) })
    outgoing.end(body)
    return
  }
  if (incoming.method === 'POST' && incoming.url === '/sendMessage') sends++
  const upstream = request({ hostname: '127.0.0.1', port: 5002,
    path: incoming.url, method: incoming.method, headers: incoming.headers }, response => {
    const headers = { ...response.headers }
    if (incoming.method === 'POST' && incoming.url === '/sendMessage' && response.statusCode === 200) {
      const signature = headers['x-bsv-auth-signature']
      if (typeof signature !== 'string' || !/^[0-9a-f]+$/i.test(signature)) {
        outgoing.writeHead(502); outgoing.end('test precondition: missing signed response'); response.resume(); return
      }
      // Flip one bit in the last signature byte, keeping DER framing unchanged.
      headers['x-bsv-auth-signature'] = signature.slice(0, -2) +
        (Number.parseInt(signature.slice(-2), 16) ^ 1).toString(16).padStart(2, '0')
      faults++; upstreamStatus = response.statusCode
      console.log('FAULT signed send response: last signature byte changed; original HTTP 200')
    }
    outgoing.writeHead(response.statusCode ?? 502, headers)
    response.pipe(outgoing)
  })
  upstream.on('error', () => { outgoing.writeHead(502); outgoing.end('test upstream connection failed') })
  incoming.pipe(upstream)
})
server.listen(5003, '127.0.0.1', () => console.log('READY test-only response fault proxy 127.0.0.1:5003 -> real TS 127.0.0.1:5002'))
