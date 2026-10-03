import json
from pathlib import Path

root = Path.cwd()
schemas = json.loads((root/"contracts/atlas-query-contract/aqc.schema.json").read_text())["$defs"]
held = []
def refs(value):
    if isinstance(value,dict):
        if "$ref" in value: yield value["$ref"]
        for child in value.values(): yield from refs(child)
    elif isinstance(value,list):
        for child in value: yield from refs(child)
for name,body in schemas.items():
    for prop,value in body.get("properties",{}).items():
        if prop.endswith("provenance"):
            assert set(refs(value)) == {"#/$defs/Provenance"},(name,prop,value)
            held.append(name+"."+prop)
assert len(held) == 8
print("All eight AQC provenance sites reference the same closed published Provenance shape:",", ".join(sorted(held)))
assert schemas["Provenance"]["properties"] == {"id":{"type":"string"},"title":{"type":"string"}}
assert set(schemas["Provenance"]["required"]) == {"id","title"}
count = 0
def check(value):
    global count
    if isinstance(value,dict):
        for key,child in value.items():
            if key.endswith("provenance"):
                values=child if isinstance(child,list) else [child]
                for p in values:
                    if p is None: continue
                    assert isinstance(p,dict) and set(p)=={"id","title"},(key,p)
                    assert p["id"].strip() and p["title"].strip(),p
                    count+=1
            else: check(child)
    elif isinstance(value,list):
        for child in value: check(child)
for folder in ["atlas-query-contract","atlas-graph-contract"]:
    for path in (root/"contracts"/folder/"fixtures").glob("*.json"):
        if path.name != "gazetteer-consumed.json":
            check(json.loads(path.read_text()))
for name in ["http","cli"]:
    doc=json.loads((root/"contracts/pacts"/(name+".json")).read_text())
    for entry in doc["entries"].values(): check(entry["body"])
assert count > 0
print("All",count,"provenance values in every AQC fixture and AGC served fixture and HTTP/CLI pact body have exactly id/title and nonblank values; no bare served ID retained.")

print("The existing gazetteer-consumed export fixture retains its raw provenance ID, as explicitly recorded in the item; file exports are outside its served-wire scope.")
