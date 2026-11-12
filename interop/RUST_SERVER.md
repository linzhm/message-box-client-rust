# Real Rust server interoperability and normal restart

Use only the authorized clean root in an independent temporary checkout:

```sh
# SERVER_CHECKOUT is a fresh isolated directory chosen by the operator.
git clone --depth 1 --single-branch --branch review/clean-initial-c43f8da0 \
  https://github.com/linzhm/message-box-server-rust.git "$SERVER_CHECKOUT"
git -C "$SERVER_CHECKOUT" rev-parse HEAD 'HEAD^{tree}'
git -C "$SERVER_CHECKOUT" cat-file -p HEAD
# Verify c43f8da0dd4c9b7082933cd0f7398bb95e092794,
# tree f5b3ca9ab38a3ffb45af71cb678a9fda8ac2420a, no parent.
cd "$SERVER_CHECKOUT"
cargo +1.94.1 build --locked --release --bin message-box-server
# Back in the client checkout, SERVER_BINARY is the absolute built binary path:
MESSAGEBOX_RUST_SERVER_BIN="$SERVER_BINARY" \
  cargo test --locked --features experimental-auth --test live_rust -- --ignored --nocapture
```

The test owns its listener, generated identities and disposable private SQLite
files. It clears the server environment, disables Firebase, uses explicit free
inbox policy and disables websockets/CORS. The first listener uses port 0;
subsequent starts reuse the actual selected port with the same identity/data.
No production credentials, message data or external service are used.

A retained SDK AuthFetch instance failed its first post-restart read with a
connection-send error. Initial failing evidence is retained locally with the original candidate; raw
logs are deliberately excluded from this source-only tree. The passing test records that narrow error,
then explicitly reconstructs AuthFetch with the same identity. Persisted unacked
message content/sender, device and permission must match; acknowledgement must
delete the message, and deletion must survive another restart. Three graceful
SIGINT stops each require exit 0 and closed listener; owned temporary data is
removed afterward. Transparent retained-instance recovery is not claimed.

## SDK policy review

Pinned rs-sdk `99d2313f2ede48a465aa8be1907d26d098aac8fd`:
`auth/transports/http.rs` constructs reqwest Client::new. AuthFetch exposes wallet,
certificate request and payment retry options, not a redirect policy, injectable
HTTP client or expected pre-request identity. Its response contains the identity
verified on that response frame. Comparing it after a send would not prevent the
initial disclosure/write, so this task does not present that as a preflight pin.

The MessageBox contract requires mutual authentication. An application-configured
expected identity and redirect policy are deployment trust choices, not additional
business endpoints or prerequisites for this controlled free-API acceptance.
No new framework, crypto, redirect exploit test or pin policy is implemented.
The existing SDK adapter remains explicitly experimental.
