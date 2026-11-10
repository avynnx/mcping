& {
    $ErrorActionPreference = 'Stop'
    $ProgressPreference = 'SilentlyContinue'
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

    $repo = 'avynnx/mcping'
    $archive = 'mcping-x86_64-pc-windows-msvc.zip'
    $version = if ($env:MCPING_VERSION) { $env:MCPING_VERSION } else { 'latest' }
    $dir = if ($env:MCPING_INSTALL_DIR) { $env:MCPING_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'Programs\mcping' }

    if (-not [Environment]::Is64BitOperatingSystem) {
        throw 'mcping needs 64-bit Windows'
    }

    if ($version -eq 'latest') {
        $base = "https://github.com/$repo/releases/latest/download"
    } else {
        $base = "https://github.com/$repo/releases/download/$version"
    }

    $tmp = Join-Path ([IO.Path]::GetTempPath()) ('mcping-' + [guid]::NewGuid())
    New-Item -ItemType Directory -Path $tmp | Out-Null
    try {
        Write-Host 'downloading mcping...'
        Invoke-WebRequest "$base/$archive" -OutFile "$tmp\$archive" -UseBasicParsing
        Invoke-WebRequest "$base/$archive.sha256" -OutFile "$tmp\$archive.sha256" -UseBasicParsing

        $expected = ((Get-Content "$tmp\$archive.sha256" -Raw).Trim() -split '\s+')[0]
        $actual = (Get-FileHash "$tmp\$archive" -Algorithm SHA256).Hash
        if ($actual -ne $expected.ToUpper()) {
            throw 'checksum mismatch, download may be corrupted'
        }

        Expand-Archive "$tmp\$archive" -DestinationPath $tmp -Force
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
        Copy-Item "$tmp\mcping.exe" "$dir\mcping.exe" -Force
    } finally {
        Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
    }

    Write-Host "installed mcping to $dir\mcping.exe"

    $path = [Environment]::GetEnvironmentVariable('Path', 'User')
    if (($path -split ';') -notcontains $dir) {
        $new = if ($path) { $path.TrimEnd(';') + ";$dir" } else { $dir }
        [Environment]::SetEnvironmentVariable('Path', $new, 'User')
        $env:Path = $env:Path.TrimEnd(';') + ";$dir"
        Write-Host "added $dir to your PATH"
    }

    Write-Host 'run: mcping mc.hypixel.net'
}
