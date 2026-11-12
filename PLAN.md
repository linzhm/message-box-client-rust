# Follow-up plan and upstream review

This is a plan, not an implementation or a test report. No follow-up implementation
is part of this initialization.

## Read-only upstream review (2026-11-12)

Reviewed [bsv-blockchain/rs-sdk](https://github.com/bsv-blockchain/rs-sdk) at commit
`99d2313f2ede48a465aa8be1907d26d098aac8fd` via its recursive file inventory and selected source files:

- [AuthFetch](https://github.com/bsv-blockchain/rs-sdk/blob/99d2313f2ede48a465aa8be1907d26d098aac8fd/src/auth/clients/auth_fetch.rs)
  provides an authenticated HTTP client, including handshake/session handling,
  wallet integration, certificate exchange, and payment retry options. It is a
  candidate to reuse after compatibility review; it has not been integrated here.
- [Service message exports](https://github.com/bsv-blockchain/rs-sdk/blob/99d2313f2ede48a465aa8be1907d26d098aac8fd/src/services/messages/mod.rs)
  cover BRC-77 signing and BRC-78 encryption. These are message primitives, not
  evidence of a complete MessageBox send/list/ack client. The inspected inventory
  and service exports did not identify a dedicated MessageBox client; this was a
  lightweight inspection, not an exhaustive audit of all behavior.
- [Upstream LICENSE](https://github.com/bsv-blockchain/rs-sdk/blob/99d2313f2ede48a465aa8be1907d26d098aac8fd/LICENSE)
  begins with Open BSV License Version 5, including BSV-chain use and notice
  conditions, and contains a historical MIT section. Do not label the current SDK
  simply MIT. Review the exact selected version's terms before adopting it.
- [bsv-blockchain/bsv-messagebox-cloudflare](https://github.com/bsv-blockchain/bsv-messagebox-cloudflare)
  is an existing official-organization Rust/Wasm server repository. Its README
  reports TS-server parity; this initialization did not independently verify that
  claim. The existence of this server must inform future interoperability work.

No upstream code was copied. Reuse is a candidate, not a dependency decision.

## Minimum future acceptance gate

1. Pin concrete TypeScript and Rust server revisions/configurations and their
   authoritative OpenAPI/auth contracts. Record which Rust server is tested.
2. Exercise real authentication and send/list/ack against both pinned servers;
   mocks alone are insufficient. Preserve reproducible, sanitized evidence.
3. Include permission and device boundaries, denied access, malformed requests,
   error/status semantics, numeric boundaries and serialization, plus restart and
   re-authentication behavior. Derive expected semantics from the pinned contracts.
4. Validate authentication SDK reuse before building an alternative. Keep realtime
   transport adaptation separate and assess Socket.IO compatibility independently.
5. Only after the client contract is validated, plan the thin MOSAIC driver in its
   own repository using existing file-resource conventions, then PayMessage above it.

No resource paths, new endpoints, Cargo dependencies, workflows, load testing,
driver code, or PayMessage behavior are introduced by this repository initialization.
