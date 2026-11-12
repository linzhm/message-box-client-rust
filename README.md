# message-box-client-rust

An independent typed Rust MessageBox **wire client candidate**, with passing real
TypeScript and Rust loopback lifecycle tests. It does not depend on Kernel or MOSAIC.
This is local development, not a published or production-ready release.

## Use

The default crate provides `Client<T: Transport>`, typed requests/responses and
structured errors. Supply an authenticated transport; the trait does not itself
establish identity. The optional `experimental-auth` feature reuses official
`bsv-sdk` AuthFetch with a concrete non-paying `ProtoWallet`.

```rust,no_run
use message_box_client::{Client, ListRequest};
use message_box_client::transport::Transport;

async fn receive<T: Transport>(transport: T) -> Result<(), message_box_client::Error> {
    let client = Client::new("http://127.0.0.1:5002", transport)?;
    let page = client.list(&ListRequest {
        message_box: "inbox".into(), limit: Some(10), ..Default::default()
    }).await?;
    for message in page.messages {
        // Persist/process before explicitly acknowledging. body is the stored wire string.
        println!("{}", message.message_id);
    }
    Ok(())
}
```

`examples/loopback.rs` shows ephemeral SDK wallet construction:
`cargo run --locked --features experimental-auth --example loopback` expects an
isolated server on port 5001. The full acceptance harness uses port 5002 instead.
No production host or secret is implicit.

Methods: `send`, `list`, `acknowledge`, `register_device`, `devices`,
`set_permission`, `permission`, `permissions`, and single-recipient `quote`.
All map to the exact pinned HTTP routes; no resource-path API is invented.
A send takes an explicit message ID, recipient and serialized body. The pinned
server stores `body` as a JSON envelope containing `message` (and possibly
`payment`). Listing returns that stored string unchanged. This layer neither
automatically encrypts/decrypts nor processes received payments, auto-acks, or
walks pages. Callers must not mistake it for full high-level TS MessageBoxClient
behavior. Batch send, realtime, FCM delivery, PayMessage and MOSAIC drivers are
outside this candidate. Device API calls only manage registrations.

## Authentication limits

`experimental-auth` is deliberately opt-in. Official rs-sdk is pinned at
`99d2313f2ede48a465aa8be1907d26d098aac8fd` in Cargo.toml and Cargo.lock.
Its AuthFetch handles real BRC-103/104 signatures and sessions; this crate does
not implement cryptography. The adapter accepts only `ProtoWallet`, whose
`create_action` is not implemented, so SDK 402 handling cannot create or broadcast
a payment. A 402 may be exposed as an SDK authentication/payment error rather than
`Error::Http`; retry count zero would not disable SDK payments.

**Deployment limitations (not additional free-API protocol requirements):** the SDK HTTP transport follows redirects and exposes no
client injection to disable them; a redirect may forward message bytes and custom
authentication headers to another origin. Initial URL validation does not constrain
redirects. The adapter does not enforce a pre-request server identity pin. Use only
controlled test endpoints and synthetic keys for this experimental adapter. Redirect control and expected-identity pinning are
deployment policy decisions; no new policy framework is introduced here. A body limit is checked after SDK buffering, not while streaming.
Timeouts can leave a write outcome uncertain, and the SDK itself may retry stale
sessions. Do not blindly retry writes after errors.

## Validation

```sh
cargo fmt --check
cargo test --locked
cargo test --locked --all-features
cargo clippy --locked --all-targets --all-features -- -D warnings
```

Rust 1.94.1 tested; declared minimum 1.94 (lower versions not claimed).
See [VALIDATION.md](VALIDATION.md) for results and NOT_RUN boundaries, and
[interop/README.md](interop/README.md) for the real loopback acceptance recipe.

## License and sources

Original code retains this private repository's [LICENSE](LICENSE); `publish=false`.
The SDK is **Open BSV License Version 5**, with historical MIT text, not simply MIT.
Its full notice is retained in [LICENSES/rs-sdk-LICENSE](LICENSES/rs-sdk-LICENSE).
No upstream implementation was vendored. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

## Rust server and restart acceptance

The real Rust release binary at `c43f8da0dd4c9b7082933cd0f7398bb95e092794`
passes the authenticated free-API lifecycle against persistent private SQLite,
including recovery of unacknowledged content after a graceful stop/start and
persistent deletion after acknowledgement and another stop/start.

An existing AuthFetch instance can produce a connection-send error on its first
read immediately after restart. An independent authenticated read first allowed
the old instance to succeed in the controlled comparison, so this is not evidence
of a deterministic session/protocol incompatibility. The SDK does not preserve
the underlying reqwest error chain in its public error; the precise socket cause
has not been established.

Use `client.reconnect()` explicitly to replace cached SDK authentication and HTTP
state **within the same Client**. It preserves wallet identity, origin, timeout and
response limit. It performs no network I/O and does not replay the failed request;
the next operation authenticates anew. An uncertain write must not be blindly
repeated. Real TS and Rust tests verify content/identity preservation after reset.
The optional AuthTransport also exposes the same `reconnect()` method directly.

```rust,ignore
// client is a mutable Client<AuthTransport>.
client.reconnect();
let page = client.list(&request).await?; // explicit next operation chosen by caller
```

See [restart acceptance](interop/RUST_SERVER.md) for the control and its limits.
Raw logs remain local, outside this source-only Git history.

Device `active` accepts official booleans and the observed SQL 0/1 representation
(strictly rejecting other integers). This does not claim a tested production MySQL
deployment. Standard errors use `description`; if a server instead emits a
`message` field without it, `Error::Http` still retains status and raw body, while
its typed `error` field is `None`. The client does not silently normalize that
nonstandard error envelope.
