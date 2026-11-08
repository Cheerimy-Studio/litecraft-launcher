$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $Root

dotnet build "pcl/MeloongCore/Shared/MeloongCore.csproj" -c Release
dotnet build "pcl/MeloongCore/Wpf/MeloongCore.Wpf.csproj" -c Release
dotnet build "pcl/PCLCS/PCLCS.csproj" -c Release

msbuild "pcl/Plain Craft Launcher 2/Plain Craft Launcher 2.vbproj" -p:Configuration=Release -v:m

cargo build --release --manifest-path "backend/launcher/Cargo.toml"
