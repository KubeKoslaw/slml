# Installs the `slml` compiler on Windows from the latest GitHub release.
#
#   irm https://raw.githubusercontent.com/KubeKoslaw/slml/main/install.ps1 | iex
#
# Set SLML_INSTALL_DIR to choose the folder (default: %LOCALAPPDATA%\slml\bin).
$ErrorActionPreference = "Stop"
$repo = "KubeKoslaw/slml"
$dest = if ($env:SLML_INSTALL_DIR) { $env:SLML_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA "slml\bin" }
$tmp = Join-Path ([IO.Path]::GetTempPath()) ("slml-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $tmp | Out-Null
try {
    $release = Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest"
    $asset = $release.assets | Where-Object { $_.name -like "*x86_64-pc-windows-msvc.zip" } | Select-Object -First 1
    if (-not $asset) { throw "no Windows build found in the latest release" }
    Invoke-WebRequest $asset.browser_download_url -OutFile (Join-Path $tmp "slml.zip")
    Expand-Archive (Join-Path $tmp "slml.zip") -DestinationPath $tmp
    New-Item -ItemType Directory -Force -Path $dest | Out-Null
    $exe = Get-ChildItem -Recurse -Path $tmp -Filter slml.exe | Select-Object -First 1
    Copy-Item $exe.FullName (Join-Path $dest "slml.exe") -Force
    $path = [Environment]::GetEnvironmentVariable("Path", "User")
    if (($path -split ";") -notcontains $dest) {
        [Environment]::SetEnvironmentVariable("Path", "$path;$dest", "User")
        Write-Host "added $dest to your PATH (open a new terminal to use it)"
    }
    & (Join-Path $dest "slml.exe") --version
} finally {
    Remove-Item -Recurse -Force $tmp
}
