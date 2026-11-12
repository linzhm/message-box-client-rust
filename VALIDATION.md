# Validation record for the source-only candidate

This source-only projection retains the code, tests, manifests, lockfiles and
harness inputs from locally reviewed candidate
`ff84ccf5cd7c500fa6482b5bcba8e029b936300a` (tree
`1443333568b20a41a5e995cbb7bc159f32a4c2cc`). Its parent is the original repository
main `85f466de43e03d67627c57d8913a824be40c0a96`; it does not inherit the three
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

This projection does not claim a new execution of those services. Documentation
changes and removal of logs do not change the tested functional inputs.
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
