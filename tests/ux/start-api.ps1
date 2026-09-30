$env:Path = "$env:Path;$env:USERPROFILE\.cargo\bin"
Set-Location "$PSScriptRoot\..\..\server"
cargo run --release -p atlas-server -- --data-dir ../data/compiled --port $env:ATLAS_API_PORT
