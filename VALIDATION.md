# Validation record for the source-only candidate

The original source-only projection retained the code, tests, manifests, lockfiles
and harness inputs from locally reviewed candidate
`ff84ccf5cd7c500fa6482b5bcba8e029b936300a` (tree
`1443333568b20a41a5e995cbb7bc159f32a4c2cc`). That projection was commit `9aa540dca83f2605692b826bef221b710f1c494d`, whose
parent is the original repository main `85f466de43e03d67627c57d8913a824be40c0a96`; it does not inherit the three
implementation/log commits. Raw execution logs, original history and a verified
restore bundle remain local and are not included or presented as published evidence.

## Fixed inputs

- Official Rust SDK: bsv-blockchain/rs-sdk
  `99d2313f2ede48a465aa8be1907d26d098aac8fd`.
- TS SDK, authentication middleware, MessageBox handlers, OpenAPI and client:
  bsv-blockchain/ts-stack `52d36a5ab1ae7cb9e3243ff4147558bcccde3c07`.
- Rust server: linzhm/message-box-server-rust root
  `c43f8da0dd4c9b7082933cd0f7398bb95e092794`, tree
  `f5b3ca9ab38a3ffb45af71cb678a9fda8ac2420a`, no parent.
- Cargo.lock and interop/package-lock.json retain exact dependency selections.
- Toolchains actually used: rustc 1.94.1 and Node 26.13.0.

## Previously executed against the identical functional inputs

| Command | Result |
|---|---|
| `cargo fmt --check` | exit 0 |
| `cargo test --locked` | exit 0; 13 contract tests |
| `cargo test --locked --all-features` | exit 0; 13 contract tests; live tests ignored |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | exit 0 |
| `cargo test --locked --features experimental-auth --test live_ts -- --ignored --nocapture` | exit 0; 2 live tests |
| `cargo +1.94.1 build --locked --release --bin message-box-server` (in pinned server checkout) | exit 0 |
| `MESSAGEBOX_RUST_SERVER_BIN="$SERVER_BINARY" cargo test --locked --features experimental-auth --test live_rust -- --ignored --nocapture` | final exit 0; 1 live lifecycle/restart test |

Those rows record the original projection inputs. Subsequent explicit-reconnect
changes and newly executed regressions are recorded below.
Reproduction instructions: [TS harness](interop/README.md) and
[Rust server acceptance](interop/RUST_SERVER.md).

## Actual coverage and observed failures

TS acceptance used real BRC-103/104 mutual authentication, unmodified pinned
handlers and in-memory SQLite, not injected identity or production MySQL.
The harness converts SQLite timestamp values to JS Date to match the handler's
MySQL-driver expectation, while retaining SQL boolean 0/1. It checks free
send/list/ack, exact stored body/sender, recipient isolation, foreign ack refusal,
permission set/get/list/quote and blocked sender, device ownership conflict,
and an authenticated malformed pagination request. Initial assertions were corrected
to the actual stored `message` envelope and hard-duplicate error response.

Rust acceptance used the real release binary, persistent disposable SQLite,
random loopback port, generated identities, explicit free inbox policy and disabled
Firebase. It checks the same free API subset, then two normal stop/start cycles.
Unacknowledged message ID/content/sender survive; device count remains one and
box-default fee remains zero. Acknowledgement deletes the message and deletion
survives another restart. Sender blocking is checked before the second restart,
not asserted separately after it. Three graceful SIGINT stops require exit 0 and
closed listeners; owned temporary runtime data is removed.

The first Rust restart run exited 101 because a retained AuthFetch instance's
first read returned a connection-send error. The passing test permits only that
observed error category and then reconstructs AuthFetch with the same identity;
the persisted message must be read successfully. This is explicit reconnection,
not transparent old-instance recovery. Both failed and successful raw logs remain
local; no automatic write retry was introduced.

## Independent review and boundaries

Independent agent `/root/client_review` reviewed exact local source candidate
`ff84ccf5cd7c500fa6482b5bcba8e029b936300a` / tree
`1443333568b20a41a5e995cbb7bc159f32a4c2cc`: limited acceptance, no blocking finding.
The report remains in the local task conversation, not a published report link.
Earlier review fixes cover uppercase recipient matching, SQL bool-or-0/1 decoding
and the tested Rust-version declaration.

The optional SDK authentication adapter remains experimental. Its underlying HTTP
client follows redirects and has no exposed pre-request expected-identity policy.
These are deployment trust choices, not new free-operation protocol requirements;
this projection adds no policy framework, cryptography or retries.

NOT_RUN/out of scope: production MySQL, real FCM, paid delivery, certificate-policy
exchange, exhaustive adversarial canonicalization, signed 402 error preservation,
streaming memory limits, cancellation/timeout ambiguity, realtime/HA, and
Kernel/MOSAIC/PayMessage integration. Ordinary real authentication passed against
both pinned servers; full protocol or production parity is not claimed.

## Explicit reconnect improvement (after source-only 9aa540d)

Implementation adds `AuthTransport::reconnect()` and
`Client<AuthTransport>::reconnect()`. A retained Arc of the original ProtoWallet
constructs a fresh official AuthFetch; origin, configured timeout and response
limit stay unchanged. Reset does no I/O and retries no read or write. Existing
SDK stale-session retry behavior is unchanged. No server or SDK source was edited.

Actual same-endpoint controls on the pinned c43 release server:

- Baseline: immediate retained AuthFetch read after restart produced a connection-send
  error again; explicitly constructing a new client recovered as previously recorded.
- New regression: fresh independently authenticated read first, then old-client read,
  both passed at the identical origin. Explicit reconnect in that same Client then
  retained exact content/sender and allowed acknowledgement.
- Next restart: immediate retained read produced the observed connection-send error;
  explicit reconnect in that same Client successfully read the persisted empty box.
- Three graceful stops require exit 0, closed listeners and final temporary-data cleanup.

This is ordering-sensitive transport behavior, not established deterministic
session incompatibility. The SDK error is generated by reqwest send before an HTTP
status is available; its underlying cause chain is not exposed. No automatic
request replay, timing sleeps as a repair, custom crypto or new policy framework.

New executed results (raw logs retained locally, not tracked):

| Command | Result |
|---|---|
| `cargo test --locked --all-features` | exit 0; 14 contract tests; live tests ignored |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | exit 0 |
| Rust live command above | exit 0; same-client reconnect and two restart controls |
| TS live command above | exit 0; 2 tests, including same-client reconnect preserving exact stored content/sender |

The new contract test verifies nonstandard `message`-only errors retain raw body
and HTTP status without being converted to the standard `description` envelope.
Existing tests retain strict bool-or-0/1 device decoding. TS acceptance still uses
real authentication and handlers with in-memory SQLite, not production MySQL.
Redirect/expected-identity policies remain unchanged and experimental.

## Real signed-response rejection acceptance (after 82dd7d1)

Source audit confirmed all nine implemented client methods already call actual
pinned TS production handlers through `postAuth`, not fabricated business routes.
See the exact method/source matrix in interop/README.md. This remains an explicitly
bounded real-auth/handler slice with in-memory SQLite; complete production app
assembly and MySQL are not claimed. Previously covered lifecycle tests were not
rerun just for this audit.

New missing authentication property tested: reject an invalid response signature
without silently accepting success or replaying a write. A loopback-only test proxy
forwards a genuine request to the unchanged pinned TS implementation and flips one
bit in the real successful send response's signature. The actual response status
and business body are otherwise unchanged. Request and fault counts are exposed
only through the test proxy's control endpoint.

Executed:
`cargo test --locked --features experimental-auth --test live_ts_response_auth -- --ignored --nocapture`
returned exit 0: one new live test passed. It observed Authentication/signature
error (not timeout/success), exactly one send and one corrupted response, then
verified one stored message with exact ID/content/sender via direct authenticated
production listMessages. Owner ack and empty reread also passed. Raw execution
output is retained locally, not in this source-only payload.

No product or upstream implementation change was needed: the pinned SDK already
rejects the invalid proof. This closes a negative acceptance gap and documents the
real uncertain-write boundary; it does not add pinning, redirect policy, retries,
cryptography or general response normalization. Official boolean versus observed
SQL 0/1 and standard description versus nonstandard message-only errors remain
separate previously documented facts.
