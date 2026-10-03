import ctypes
import ctypes.util
import hashlib
import itertools
import json
from pathlib import Path
import re
import sqlite3
import tempfile


def verify(before_root, after_root):
    before = manifest(before_root)
    after = manifest(after_root)
    assert before.keys() == after.keys()
    total_rows = 0
    total_tables = 0
    for name in after:
        with tempfile.TemporaryDirectory(prefix="codex-f39-parity-", dir="/dev/shm") as temporary:
            left = unpack(before_root, name, before[name], Path(temporary) / "before.sqlite")
            right = unpack(after_root, name, after[name], Path(temporary) / "after.sqlite")
            left_tables = table_names(left)
            assert left_tables == table_names(right)
            held_rows = 0
            for table in left_tables:
                columns = left.execute(f"pragma table_info('{table}')").fetchall()
                assert columns == right.execute(f"pragma table_info('{table}')").fetchall()
                key = [row[1] for row in sorted(columns, key=lambda row: row[5]) if row[5]]
                assert key
                ordering = ",".join(f'"{column}"' for column in key)
                left_rows = left.execute(f'SELECT * FROM "{table}" ORDER BY {ordering}')
                right_rows = right.execute(f'SELECT * FROM "{table}" ORDER BY {ordering}')
                rows = 0
                for a, b in itertools.zip_longest(left_rows, right_rows):
                    assert a == b, (name, table, a, b)
                    rows += 1
                held_rows += rows
            assert left.execute("SELECT * FROM meta WHERE key != 'logical_hash' ORDER BY key").fetchall() == right.execute("SELECT * FROM meta WHERE key != 'logical_hash' ORDER BY key").fetchall()
            left.close()
            right.close()
            total_rows += held_rows
            total_tables += len(left_tables)
            print(name, "all non-meta rows identical:", held_rows, "rows,", len(left_tables), "tables; schema", after[name]["schema_version"])
    print("Whole artifact parity:", total_rows, "rows across", total_tables, "section tables unchanged; only meta.logical_hash differs.")


def manifest(root):
    text = (root / "data/compiled/manifest.toml").read_text()
    held = {}
    for block in text.split("[[section]]")[1:]:
        row = {}
        for line in block.splitlines():
            match = re.fullmatch(r"(\w+)\s*=\s*(.*)", line.strip())
            if match:
                key, value = match.groups()
                row[key] = json.loads(value)
        held[row["name"]] = row
    return held


def unpack(root, name, section, destination):
    blob = (root / "data/compiled/sections" / f"{name}.{section['logical']}.sqlite.zst").read_bytes()
    assert len(blob) == section["bytes"]
    assert hashlib.sha256(blob).hexdigest() == section["blob"]
    library = ctypes.CDLL(ctypes.util.find_library("zstd"))
    library.ZSTD_decompress.argtypes = [ctypes.c_void_p, ctypes.c_size_t, ctypes.c_void_p, ctypes.c_size_t]
    library.ZSTD_decompress.restype = ctypes.c_size_t
    library.ZSTD_isError.argtypes = [ctypes.c_size_t]
    library.ZSTD_isError.restype = ctypes.c_uint
    source = ctypes.create_string_buffer(blob)
    capacity = 1024 * 1024 * 1024
    output = ctypes.create_string_buffer(capacity)
    length = library.ZSTD_decompress(output, capacity, source, len(blob))
    assert not library.ZSTD_isError(length)
    destination.write_bytes(ctypes.string_at(output, length))
    db = sqlite3.connect(f"file:{destination}?mode=ro", uri=True)
    metadata = dict(db.execute("SELECT * FROM meta"))
    assert metadata["schema_version"] == str(section["schema_version"])
    assert metadata["logical_hash"] == section["logical"]
    return db


def table_names(db):
    return [row[0] for row in db.execute("SELECT name FROM sqlite_master WHERE type='table' AND name!='meta' ORDER BY name")]


if __name__ == "__main__":
    import sys
    verify(Path(sys.argv[1]), Path(sys.argv[2]))
