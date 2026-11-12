"""Configure only an isolated upstream checkout; never use a working server repo."""
import json, subprocess, sys
from pathlib import Path
root = Path(sys.argv[1]).resolve()
expected = "52d36a5ab1ae7cb9e3243ff4147558bcccde3c07"
assert subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip() == expected
here = Path(__file__).resolve().parent
paths = {
 "@bsv/sdk": ["packages/sdk/mod.ts"],
 "@bsv/sdk/primitives": ["packages/sdk/src/primitives/index.ts"],
 "@bsv/sdk/*": ["packages/sdk/src/*"],
 "@bsv/auth-express-middleware": ["packages/middleware/auth-express-middleware/mod.ts"],
 "@reference/auth": ["packages/middleware/auth-express-middleware/mod.ts"],
 "@reference/runtime": ["infra/message-box-server/src/runtimeDeps.ts"],
 "@reference/routes": ["infra/message-box-server/src/routes/index.ts"],
}
(here / "tsconfig.local.json").write_text(json.dumps({"compilerOptions":{"baseUrl":str(root),"paths":paths}}, indent=2))
link = root / "node_modules"
if not link.exists() and not link.is_symlink(): link.symlink_to(here / "node_modules", target_is_directory=True)
else: assert link.resolve() == (here / "node_modules").resolve(), "Use a fresh isolated checkout; will not replace existing dependencies"
