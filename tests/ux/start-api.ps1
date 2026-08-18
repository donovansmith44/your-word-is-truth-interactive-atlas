$env:Path = "$env:Path;$env:USERPROFILE\.cargo\bin"
Set-Location "$PSScriptRoot\..\..\server"
cargo run -p atlas-server -- --data-dir ../data/compiled --port 8000
