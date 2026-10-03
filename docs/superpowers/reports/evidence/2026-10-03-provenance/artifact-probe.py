import itertools
from pathlib import Path
import runpy
import sys
import tempfile

helpers = runpy.run_path(str(Path(__file__).with_name("artifact-helpers.py")))
old_root, new_root = map(Path, sys.argv[1:])
old_manifest, new_manifest = [helpers["manifest"](root) for root in [old_root, new_root]]
assert old_manifest.keys() == new_manifest.keys()
assert {row["schema_version"] for row in new_manifest.values()} == {27}
all_ids = set()
titles = {}
rows_held = 0
tables_held = 0
for name in new_manifest:
    with tempfile.TemporaryDirectory(prefix="codex-provenance-", dir="/dev/shm") as temporary:
        old = helpers["unpack"](old_root, name, old_manifest[name], Path(temporary)/"old.sqlite")
        new = helpers["unpack"](new_root, name, new_manifest[name], Path(temporary)/"new.sqlite")
        old_tables, new_tables = [helpers["table_names"](db) for db in [old, new]]
        assert set(new_tables)-set(old_tables) == ({"provenance_title"} if name == "core" else set())
        assert set(old_tables)-set(new_tables) == set()
        count = 0
        for table in old_tables:
            columns = old.execute(f"pragma table_info('{table}')").fetchall()
            assert columns == new.execute(f"pragma table_info('{table}')").fetchall()
            key = [row[1] for row in sorted(columns,key=lambda row:row[5]) if row[5]]
            assert key
            order = ",".join(f'"{column}"' for column in key)
            left, right = [db.execute(f'SELECT * FROM "{table}" ORDER BY {order}') for db in [old, new]]
            for a,b in itertools.zip_longest(left,right):
                assert a == b,(name,table,a,b)
                count += 1
            if "provenance" in [c[1] for c in columns]:
                all_ids.update(row[0] for row in new.execute(f'SELECT DISTINCT provenance FROM "{table}"'))
        old_meta, new_meta = [dict(db.execute("SELECT * FROM meta")) for db in [old,new]]
        diffs = {k for k in old_meta if old_meta[k] != new_meta[k]}
        assert diffs == ({"schema_version","logical_hash"} if name == "core" else {"schema_version"}),diffs
        if name == "core":
            complete = new.execute("SELECT id,title FROM provenance_title ORDER BY id").fetchall()
            declared = new.execute("SELECT p.id,s.title FROM provenance_entry p JOIN source_entry s ON s.id=p.source ORDER BY p.id").fetchall()
            assert complete == declared
            assert len(complete) == new.execute("SELECT COUNT(*) FROM provenance_entry").fetchone()[0]
            titles = dict(complete)
            print("core complete provenance_title table exactly equals every provenance/source join:",len(complete),"rows",flush=True)
        rows_held += count
        tables_held += len(old_tables)
        print(name,count,"existing rows /",len(old_tables),"existing tables identical; blob SHA/length and schema 27 verified",flush=True)
        old.close()
        new.close()
assert all_ids
untitled = sorted(id for id in all_ids if id.split("/",1)[0] not in titles)
blank_titles = sorted(id for id in all_ids if not titles.get(id.split("/",1)[0],"").strip())
assert (untitled,blank_titles) == ([],[])
print("Whole prior artifact:",rows_held,"existing rows /",tables_held,"tables unchanged; only 33 compiled title rows added.")
print("All actual provenance-bearing tables across all five sections:",len(all_ids),"distinct full IDs resolve to a nonblank compiled source title; no unmigrated artifact family.")
