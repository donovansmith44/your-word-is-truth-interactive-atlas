import runpy
import subprocess
import sys
import tempfile
from pathlib import Path

root = Path(sys.argv[1])
probe = sys.argv[2]
helpers = runpy.run_path(str(root / "docs/superpowers/reports/evidence/2026-10-03-f39/artifact-parity.py"))
for name, declared in helpers["manifest"](root).items():
    with tempfile.TemporaryDirectory(prefix="codex-f39-hash-", dir="/dev/shm") as temporary:
        path = Path(temporary) / "section.sqlite"
        helpers["unpack"](root, name, declared, path).close()
        actual = subprocess.check_output([probe, name, str(path)], text=True).strip()
        assert actual == declared["logical"], (name, actual, declared["logical"])
        print(name, actual, "production logical hash matches declared manifest", flush=True)
