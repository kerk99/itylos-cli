$ErrorActionPreference = "Stop"

$Repo = "kerk99/itylos-cli"
$Binary = "itylos.exe"
$Version = if ($env:ITYLOS_VERSION) { $env:ITYLOS_VERSION } else { "latest" }
$InstallDir = if ($env:ITYLOS_INSTALL_DIR) { $env:ITYLOS_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA "Programs\itylos\bin" }

function Get-Target {
    return "x86_64-pc-windows-msvc"
}

function Get-ReleaseApiUrl {
    if ($Version -eq "latest") {
        return "https://api.github.com/repos/$Repo/releases/latest"
    }

    return "https://api.github.com/repos/$Repo/releases/tags/$Version"
}

function Get-AssetName {
    $target = Get-Target
    $release = Invoke-RestMethod -Uri (Get-ReleaseApiUrl)
    $asset = $release.assets | Where-Object { $_.name -match "^itylos-.*-$([regex]::Escape($target))\.zip$" } | Select-Object -First 1
    if (-not $asset) {
        throw "No release asset found for target $target"
    }

    return $asset.name
}

function Add-UserPathIfMissing([string]$PathEntry) {
    $current = [Environment]::GetEnvironmentVariable("Path", "User")
    $entries = @()
    if ($current) {
        $entries = $current.Split(';') | Where-Object { $_ -ne "" }
    }

    if ($entries -contains $PathEntry) {
        return
    }

    $updated = ($entries + $PathEntry) -join ';'
    [Environment]::SetEnvironmentVariable("Path", $updated, "User")
}

$assetName = Get-AssetName
$downloadUrl = if ($Version -eq "latest") {
    "https://github.com/$Repo/releases/latest/download/$assetName"
} else {
    "https://github.com/$Repo/releases/download/$Version/$assetName"
}

$tempRoot = Join-Path ([IO.Path]::GetTempPath()) ("itylos-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $tempRoot | Out-Null

try {
    $archivePath = Join-Path $tempRoot $assetName
    Invoke-WebRequest -Uri $downloadUrl -OutFile $archivePath

    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    Expand-Archive -LiteralPath $archivePath -DestinationPath $tempRoot -Force
    Copy-Item -LiteralPath (Join-Path $tempRoot $Binary) -Destination (Join-Path $InstallDir $Binary) -Force
    Add-UserPathIfMissing $InstallDir

    Write-Host "Installed $Binary to $InstallDir"
    Write-Host "Open a new terminal and run: itylos --help"
}
finally {
    if (Test-Path $tempRoot) {
        Remove-Item -LiteralPath $tempRoot -Recurse -Force
    }
}
