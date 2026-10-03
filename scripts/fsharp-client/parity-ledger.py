import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys


def git(*arguments):
    return subprocess.check_output(["git", *arguments], text=True).strip()


def inventory(reference):
    entries = []
    tracked = git("ls-tree", "-r", "--name-only", reference).splitlines()
    for path in tracked:
        if path.startswith("client/") or path.startswith("tests/ux/") and path.endswith(".ts"):
            body = subprocess.check_output(["git", "show", reference + ":" + path])
            entries.append({"source": path, "sha256": hashlib.sha256(body).hexdigest()})
    contract = subprocess.check_output(["git", "show", reference + ":contracts/openapi.yaml"])
    return {"reference": git("rev-parse", reference), "contract_sha256": hashlib.sha256(contract).hexdigest(), "inventory": entries}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--reference")
    parser.add_argument("--ledger", default="tests/parity-fsharp/ledger.json")
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--require-complete", action="store_true")
    args = parser.parse_args()
    ledger = Path(args.ledger)
    if args.write:
        if not args.reference:
            parser.error("--write requires an explicit --reference")
        existing = json.loads(ledger.read_text()) if ledger.exists() else {"inventory": []}
        previous = {entry["source"]: entry for entry in existing["inventory"]}
        current = inventory(args.reference)
        for entry in current["inventory"]:
            old = previous.get(entry["source"], {})
            unchanged = old.get("sha256") == entry["sha256"]
            shared = entry["source"].startswith("client/wwwroot/") and not entry["source"].endswith(("index.html", ".json"))
            entry.update({"status": old.get("status", "pending") if unchanged else "shared-asset" if shared else "pending", "replacement": old.get("replacement", []) if unchanged else [], "evidence": old.get("evidence", []) if unchanged else []})
        ledger.write_text(json.dumps(current, indent=2) + "\n")
        print(f"Pinned {len(current['inventory'])} files at {current['reference']}")
        return 0
    current = json.loads(ledger.read_text())
    expected = inventory(args.reference or current["reference"])
    actual = {"reference": current["reference"], "contract_sha256": current["contract_sha256"], "inventory": [{"source": entry["source"], "sha256": entry["sha256"]} for entry in current["inventory"]]}
    if actual != expected:
        print("Parity inventory differs from its explicit reference", file=sys.stderr)
        return 1
    pending = []
    for entry in current["inventory"]:
        status = entry["status"]
        if status not in {"pending", "shared-asset", "verified"}:
            print(f"Unknown parity status: {status}", file=sys.stderr)
            return 1
        if status == "verified" and (not entry["replacement"] or not entry["evidence"]):
            print(f"Verified entry has no replacement/evidence: {entry['source']}", file=sys.stderr)
            return 1
        if status == "pending":
            pending.append(entry["source"])
    print(f"Inventory matches {current['reference']}; {len(pending)} entries pending")
    return 1 if args.require_complete and pending else 0


if __name__ == "__main__":
    sys.exit(main())
