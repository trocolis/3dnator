# Regressões de preservação de arquivos; não exigem SDK nem Pester.
$ErrorActionPreference = "Stop"
$Script = Join-Path $PSScriptRoot "package_windows.ps1"
$TestRoot = Join-Path ([IO.Path]::GetTempPath()) ("3dnator-package-" + [Guid]::NewGuid())
New-Item -ItemType Directory -Path $TestRoot | Out-Null
try {
    $FakeBin = Join-Path $TestRoot "fixture.exe"
    Set-Content -LiteralPath $FakeBin -Value "fixture"
    $ExistingDir = Join-Path $TestRoot "pacote existente"
    New-Item -ItemType Directory -Path $ExistingDir | Out-Null
    $Sentinel = Join-Path $ExistingDir "arquivo do usuário.txt"
    Set-Content -LiteralPath $Sentinel -Value "preservar"
    $Rejected = $false
    try { & $Script -BinPath $FakeBin -OutputDir $ExistingDir } catch {
        if ($_.Exception.Message -notlike '*destino já existe*') { throw }
        $Rejected = $true
    }
    if (-not $Rejected -or (Get-Content -LiteralPath $Sentinel) -ne "preservar") {
        throw "O destino existente não foi preservado."
    }
    $NewDir = Join-Path $TestRoot "pacote novo"
    Set-Content -LiteralPath "$NewDir.zip" -Value "preservar"
    $Rejected = $false
    try { & $Script -BinPath $FakeBin -OutputDir $NewDir -Archive } catch {
        if ($_.Exception.Message -notlike '*já existe*') { throw }
        $Rejected = $true
    }
    if (-not $Rejected -or (Test-Path -LiteralPath $NewDir) -or (Get-Content -LiteralPath "$NewDir.zip") -ne "preservar") {
        throw "O arquivo compactado existente não foi preservado."
    }
    Write-Host "2 regressões de preservação aprovadas."
} finally {
    # Remover somente a pasta temporária criada por este próprio ensaio.
    Remove-Item -LiteralPath $TestRoot -Recurse -Force
}
