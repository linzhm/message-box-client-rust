# Reproduce the TypeScript interoperability acceptance

This is a test harness, not a deployable server. It imports the **unmodified real**
TypeScript BRC-103/104 middleware, SDK and MessageBox handlers from the fixed
upstream commit. It uses real ephemeral keys, a real SQLite database in memory,
and loopback only. No mock identity injection, production credentials, Firebase,
wallet payment service, Docker, or existing server checkout is used.

Create a fresh temporary directory containing this `interop/` directory, then:

```sh
# In the temporary copy of interop/
git clone --filter=blob:none https://github.com/bsv-blockchain/ts-stack.git ../ts-stack
git -C ../ts-stack checkout --detach 52d36a5ab1ae7cb9e3243ff4147558bcccde3c07
npm ci
python3 configure.py ../ts-stack
ENABLE_FIREBASE=false MESSAGE_BOX_MONETIZATION_ENABLED=false LOG_LEVEL=error   ./node_modules/.bin/tsx --tsconfig tsconfig.local.json server.ts
# In this Rust client checkout, another terminal:
cargo test --locked --features experimental-auth --test live_ts -- --ignored --nocapture
```

The harness listens on `127.0.0.1:5002`; stop it with Ctrl-C. Use a clean temporary
working directory without an `.env` file. `package-lock.json` locks the test-only
Node dependencies; the TypeScript protocol implementation comes from Git, not npm.
Node 26.13.0 was used. Native better-sqlite3 installation may require a compiler.

The schema is minimal and constructed for these free-operation tests, not the
production MySQL migration deployment. The SQLite result adapter converts timestamp
columns to JS Date objects to match the MySQL driver expected by getPermission;
it does **not** normalize SQL boolean 0/1. Firebase remains disabled.
The harness directly registers real handlers after real authentication; production
payment middleware, websocket notifications, rate limiting and telemetry are not
part of this acceptance. No upstream server source is edited.

Covered: mutual auth, signed success and error responses, free send/list/ack,
stored-body envelope, duplicate-ID error, recipient isolation, unauthorized ack,
permissions set/get/list/quote and blocked sender, device registration/list/ownership
conflict. Tests generate fresh keys so repeated runs do not collide.

## Exact production-handler coverage

The current harness imports `postAuth` from the pinned production
`infra/message-box-server/src/routes/index.ts` and registers those handlers after
real `createAuthMiddleware`. It does not supply replacement business responses.
The earlier standalone authentication probe with a synthetic list response is not
counted as the lifecycle acceptance below.

| Client method / HTTP route | Actual pinned production source under `src/routes/` | Exercised behavior |
|---|---|---|
| send / POST `/sendMessage` | `sendMessage.ts` | stored content, sender, duplicate and blocked errors |
| list / POST `/listMessages` | `listMessages.ts` | recipient isolation, envelope, deletion, malformed pagination |
| acknowledge / POST `/acknowledgeMessage` | `acknowledgeMessage.ts` | owner deletion, foreign identity rejection |
| register_device / POST `/registerDevice` | `registerDevice.ts` | registration and cross-identity token conflict |
| devices / GET `/devices` | `listDevices.ts` | owner isolation and SQL 0/1 decoding |
| set_permission / POST `/permissions/set` | `permissions/setPermission.ts` | box default and sender-specific block |
| permission / GET `/permissions/get` | `permissions/getPermission.ts` | stored default fee |
| permissions / GET `/permissions/list` | `permissions/listPermissions.ts` | owned records/count |
| quote / GET `/permissions/quote` | `permissions/getQuote.ts` | free single-recipient quote |

This covers the client's implemented methods, not every branch of each handler.
Production app assembly, payment middleware, MySQL deployment and non-client routes
are not tested by this slice. Official `active` is boolean; the observed SQL 0/1
handling and nonstandard `message` versus `description` envelope handling are
separate documented boundaries, not a generalized server-normalization layer.

## Negative response-authentication acceptance

With the real harness running on port 5002, in a second terminal in the same
isolated temporary interop copy:

```sh
./node_modules/.bin/tsx response_fault_proxy.ts
# In the client checkout:
cargo test --locked --features experimental-auth --test live_ts_response_auth -- --ignored --nocapture
```

The test-only proxy listens on loopback port 5003 and forwards requests to the
real port-5002 server unchanged. After an actual successful `/sendMessage` response,
it flips one bit in the last signature byte, preserving the response body/status
and DER framing. It does not forge a business result or create an identity.
`/_test/counts` is only this proxy's test control endpoint, not a MessageBox route.

The test asserts an immediate SDK authentication/signature error rather than a
typed success or timeout; verifies exactly one forwarded send and one injection;
then uses a direct authenticated recipient client to verify the exact stored ID,
sender and body. Finally it acknowledges and checks deletion through real routes.
This demonstrates that an invalid response proof is rejected even when the server
has already committed the write. It does not establish expected-server identity
pinning or every signature-error case. Stop both temporary processes afterward.
Raw logs remain local and are not tracked.
