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
