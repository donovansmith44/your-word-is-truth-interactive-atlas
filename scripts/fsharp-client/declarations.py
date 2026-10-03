import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

root = Path(__file__).resolve().parents[2]
environment = dict(os.environ, DOTNET_GCHeapHardLimit="0x100000000")
self_test = sys.argv[1:] == ["--self-test"]
projects = [
    root / 'client-fsharp.ContractGenerator/BibleAtlas.FSharp.ContractGenerator.fsproj',
    root / 'client-fsharp/Core/BibleAtlas.FSharp.Core.fsproj',
    root / 'client-fsharp/BibleAtlas.FSharp.Client.fsproj',
]
manifest = []
for project in projects:
    output = subprocess.run([
        'dotnet', 'msbuild', str(project), '-t:Compile', '-p:BuildProjectReferences=false',
        '-p:ProvideCommandLineArgs=true', '-p:SkipCompilerExecution=true', '-p:NonExistentFile=force-declaration-analysis',
        '-getItem:FscCommandLineArgs',
    ], cwd=project.parent, check=True, capture_output=True, text=True)
    arguments = [item['Identity'] for item in json.loads(output.stdout)['Items']['FscCommandLineArgs']]
    manifest.append({'project': str(project), 'arguments': arguments})
with tempfile.TemporaryDirectory(prefix='atlas-declarations-') as temporary:
    if self_test:
        directory = Path(temporary)
        definitions = directory / 'Definitions.fs'
        definitions.write_text('namespace BibleAtlas.FSharp.Domain\nmodule internal DomainSkeleton =\n    let pending (name: string) : \'a = failwith name\nnamespace Probe\n[<RequireQualifiedAccess>]\ntype Choices = Used | NeverUsed\ntype UnusedType = { UnusedField: int }\nmodule Values =\n    let used () = Choices.Used\n    let phantom<\'unused> (value: int) = value\n    let unusedPublic () = 1\n    let internal unusedInternal () = 2\n    let private unusedPrivate () = 3\n    let testOnly () = 4\n    let pendingBody (unused: int) : int = BibleAtlas.FSharp.Domain.DomainSkeleton.pending "Values.pendingBody"\n')
        caller = directory / 'Caller.fs'
        caller.write_text('module Probe.Program\n[<EntryPoint>]\nlet main _ =\n    let value = Probe.Values.phantom<string> 0\n    match Probe.Values.used () with\n    | Probe.Choices.Used -> value\n    | _ -> 1\n')
        test_directory = directory / 'Probe.Tests'
        test_directory.mkdir()
        tests = test_directory / 'Tests.fs'
        tests.write_text('module Probe.Tests\nlet value = Probe.Values.testOnly ()\n')
        arguments = [argument for argument in manifest[1]['arguments'] if argument.startswith('-')]
        arguments += [str(definitions), str(tests), str(caller)]
        manifest = [{'project': str(directory / 'Probe.fsproj'), 'arguments': arguments}]
    source = Path(temporary) / 'projects.json'
    source.write_text(json.dumps(manifest))
    result = subprocess.run(['dotnet', 'fsi', str(Path(__file__).with_suffix('.fsx')), str(source)], cwd=root, env=environment, timeout=120, capture_output=self_test, text=self_test)
    if self_test:
        if result.returncode != 1:
            print(result.stdout)
            print(result.stderr, file=sys.stderr)
            sys.exit(2)
        report = json.loads(result.stdout)
        actual = sorted(item['name'] for item in report['unused'])
        expected = sorted(['NeverUsed', 'UnusedType', 'UnusedField', 'unusedPublic', 'unusedInternal', 'unusedPrivate', 'testOnly', 'unused'])
        assert report['errors'] == [], report['errors']
        assert report['pending_bodies'] == 1, report['pending_bodies']
        assert actual == expected, (expected, actual)
        print(json.dumps({'errors': [], 'detected': actual, 'pending_bodies': 1}))
        sys.exit(0)
    sys.exit(result.returncode)
