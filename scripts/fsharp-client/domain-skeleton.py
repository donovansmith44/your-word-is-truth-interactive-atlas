import json
from pathlib import Path
import re
import sys
import xml.etree.ElementTree as ET

root = Path(__file__).resolve().parents[2]
core = root / 'client-fsharp/Core'
required = [
    'Domain/Positive.fs', 'Domain/NonEmpty.fs', 'Domain/Rooted.fs',
    'Domain/Skeleton.fs', 'Domain/Time.fs', 'Domain/Text.fs',
    'Domain/Containers.fs', 'Domain/Graph.fs', 'Domain/History.fs',
    'Domain/YearReading.fs', 'Domain/Position.fs',
    'Paging/ReadingWindow.fs', 'Paging/NeighbourWindow.fs', 'Paging/PageCache.fs',
    'Exploration/Trail.fs', 'Exploration/Focus.fs',
    'Admission/ArrayIndex.fs', 'Admission/DocumentPosition.fs',
    'Admission/Coordinates.fs', 'Admission/HttpUrl.fs', 'Admission/Scalars.fs',
    'Admission/TextSpans.fs', 'Admission/Paths.fs', 'Admission/WireFailure.fs',
    'Admission/ReadFailure.fs',
]
compile_files = [item.attrib['Include'] for item in ET.parse(core / 'BibleAtlas.FSharp.Core.fsproj').iter('Compile')]
failures = []
positions = []
pending = {}
for name in required:
    path = core / name
    if not path.is_file():
        failures.append(f'missing real module: {name}')
        continue
    if name not in compile_files:
        failures.append(f'module absent from compile order: {name}')
    else:
        positions.append(compile_files.index(name))
    source = path.read_text()
    operations = re.findall(r'DomainSkeleton\.pending\s+"([^"]+)"', source)
    if operations:
        pending[name] = operations
    if name != 'Domain/Skeleton.fs' and re.search(r'\b(failwith|raise|invalidOp|Unchecked\.defaultof)\b', source):
        failures.append(f'unmarked throwing/default body: {name}')
if positions != sorted(positions):
    failures.append('domain modules do not follow the declared dependency order')
if list((root / 'client-fsharp/Domain').glob('*.fsi')):
    failures.append('standalone proposal signatures remain')
for path in core.rglob('*.fs'):
    relative = path.relative_to(core).as_posix()
    if 'obj' in path.parts or 'bin' in path.parts:
        continue
    if relative not in required and 'DomainSkeleton.pending' in path.read_text():
        failures.append(f'placeholder outside domain inventory: {relative}')
marker = core / 'Domain/Skeleton.fs'
if marker.exists():
    source = marker.read_text()
    if source.count('NotImplementedException') != 1 or len(re.findall(r'\braise\b', source)) != 1:
        failures.append('the named marker must own exactly one exception construction and raise')
all_operations = [operation for operations in pending.values() for operation in operations]
if len(all_operations) != len(set(all_operations)):
    failures.append('placeholder operation names must be unique')
print(json.dumps({'pending_count': len(all_operations), 'pending': pending, 'failures': failures}, indent=2))
sys.exit(bool(failures))
