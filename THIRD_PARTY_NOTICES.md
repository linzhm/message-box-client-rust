# Third-party references and dependency notice

© BSV Association. `bsv-blockchain/rs-sdk` at
`99d2313f2ede48a465aa8be1907d26d098aac8fd` is an external dependency under Open BSV
License Version 5, including BSV-chain-use and notice conditions. The full upstream
license, including its historical MIT section, is in LICENSES/rs-sdk-LICENSE.
This dependency must not be described as simply MIT.

Authoritative protocol reference: `bsv-blockchain/ts-stack` at
`52d36a5ab1ae7cb9e3243ff4147558bcccde3c07`, specifically:

- specs/messaging/message-box-http.yaml
- infra/message-box-server/src/routes/ and security/messageFields.ts
- packages/messaging/message-box-client/src/MessageBoxClient.ts
- packages/middleware/auth-express-middleware/

The isolated test harness imports those original sources from a separate checkout;
they retain that checkout's LICENSE.txt and THIRD_PARTY_NOTICES.md. No upstream
source implementation is copied into this crate. Rust and Node lockfiles retain
the exact transitive dependency selections; their respective licenses still apply.
