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

The original candidate required constructing a new Client after an observed
connection-send error. The current API provides an explicit `Client::reconnect()`:
it replaces the existing adapter's AuthFetch with the same wallet, keeping origin,
timeout and response limit. No I/O or automatic request replay occurs in reset.

The regression controls the endpoint and ordering:

1. The server's actual bound address and public identity are asserted unchanged
   across normal stops/starts. Each stop waits for exit 0 and a closed listener.
2. At the first restart, an independent authenticated read succeeds at that exact
   origin before the retained client's read. Both reads passed in the observed run.
3. The retained Client explicitly reconnects; it must still read exact unacked
   content/sender, then acknowledge it successfully.
4. At the second restart, the retained client reads immediately after ready. The
   observed run returned the original connection-send error. Explicit reconnect
   within that same Client then successfully verifies persistent deletion.

The old-instance read may succeed or return only that specific observed error
category; other failures fail the test. The test does not force an intermittent
failure or turn it into a production retry policy. The comparison establishes
an ordering-sensitive transport outcome, not a proven SDK session protocol bug.
The SDK converts reqwest's send error to a display string, losing its deeper
source chain; no HTTP status was returned for the observed error.

The SDK's own stale-session recovery recognizes SessionNotFound, a stale nonce
marker or unsigned HTTP 401 from a known peer. The observed send error matches
none of those. It exposes no public reset/injected transport. Reconstructing the
existing SDK object is therefore the minimal explicit lifecycle operation; this
client does not implement authentication or cryptography itself.

Raw initial/baseline/regression logs remain in local preservation storage, not this
source-only tree. All runtime data is synthetic and removed after the test.
No transparent retained-instance recovery is claimed.

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
