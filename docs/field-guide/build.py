"""Merge the writers' JSON files into data.js for the Field Guide page."""
import json, glob, os, sys
ORDER = ["graph-types.json","atlas-graph-src.json","atlas-graph-tests.json","atlas-etl.json","atlas-core.json","atlas-server-cli.json","client.json","web-and-tests.json","contracts-data-docs.json"]
areas, problems = [], []
for name in ORDER:
    p = os.path.join("data", name)
    if not os.path.exists(p): problems.append(f"missing {name}"); continue
    try:
        d = json.load(open(p, encoding="utf-8"))
    except Exception as e:
        problems.append(f"invalid {name}: {e}"); continue
    if "files" not in d: problems.append(f"{name}: no files[]"); continue
    paths = [f["path"] for f in d["files"]]
    dup = {x for x in paths if paths.count(x) > 1}
    if dup: problems.append(f"{name}: duplicate paths {sorted(dup)[:3]}")
    ro = d.get("reading_order", [])
    missing_ro = [x for x in paths if x not in ro]
    if missing_ro: d["reading_order"] = ro + missing_ro   # tolerate; the page shows all
    grouped = {x for g in d.get("groups", []) for x in g["paths"]}
    ungrouped = [x for x in paths if x not in grouped]
    if ungrouped: d.setdefault("groups", []).append({"name": "Other files", "paths": ungrouped, "note": "files the writer did not place in a group"})
    for f in d["files"]:
        for k in ("l1","l2","l3","role"): f.setdefault(k, "")
        if f["path"].endswith(("graph.bin",".png",".zip")): f["lines"] = 0   # binary: wc -l is meaningless
        f.setdefault("key_items", []); f.setdefault("depends_on", []); f.setdefault("used_by", []); f.setdefault("concepts", [])
    areas.append(d)
gp = os.path.join("data", "glossary.json")
glossary = {"primer_order": [], "entries": []}
if os.path.exists(gp):
    try: glossary = json.load(open(gp, encoding="utf-8"))
    except Exception as e: problems.append(f"invalid glossary.json: {e}")
else: problems.append("missing glossary.json")
out = {"areas": areas, "glossary": glossary}
js = "window.EXPLORER = " + json.dumps(out, ensure_ascii=False, separators=(",", ":")) + ";\n"
open("data.js", "w", encoding="utf-8").write(js)
nfiles = sum(len(a["files"]) for a in areas); nlines = sum(f.get("lines", 0) for a in areas for f in a["files"])
print(f"data.js: {len(js)/1024:.0f} KB, {len(areas)} areas, {nfiles} files, {nlines} lines, {len(glossary.get('entries', []))} glossary entries")
for pr in problems: print("PROBLEM:", pr)
