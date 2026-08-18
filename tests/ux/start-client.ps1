$env:Path = "$env:Path;$env:LOCALAPPDATA\Microsoft\dotnet"
$env:DOTNET_ROOT = "$env:LOCALAPPDATA\Microsoft\dotnet"
Set-Location "$PSScriptRoot\..\.."
dotnet run --project client --launch-profile http
