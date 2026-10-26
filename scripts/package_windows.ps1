<#
.SYNOPSIS
    Script de empacotamento do 3dnator para Windows (x64).
    Reúne o executável compilado, as DLLs do OpenCASCADE (OCCT)
    e os avisos de licença em um pacote portátil independente.
#>

[CmdletBinding()]
param (
    [string]$BinPath,
    [string]$OcctBinDir,
    [string]$OutputDir,
    [switch]$Archive
)

$ErrorActionPreference = "Stop"

# Raiz do repositório
$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $RepoRoot

# Localizar executável se não fornecido
if (-not $BinPath) {
    $Candidates = @(
        "target\release\nator-prototype.exe",
        "target\debug\nator-prototype.exe"
    )
    foreach ($Cand in $Candidates) {
        if (Test-Path $Cand) {
            $BinPath = $Cand
            break
        }
    }
}

if (-not $BinPath -or -not (Test-Path $BinPath)) {
    Write-Error "Nenhum executável válido encontrado. Especifique com -BinPath ou compile o projeto."
    exit 1
}

Write-Host "==> Usando executável: $BinPath"

# Definir diretório de destino
if (-not $OutputDir) {
    $OutputDir = Join-Path $RepoRoot "dist\3dnator-windows-x64"
}

Write-Host "==> Preparando diretório de destino: $OutputDir"
if (Test-Path $OutputDir) {
    Remove-Item -Recurse -Force $OutputDir
}
New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

# Copiar executável renomeando para 3dnator.exe
Copy-Item -Path $BinPath -Destination (Join-Path $OutputDir "3dnator.exe")

# Localizar diretório das DLLs do OCCT
if (-not $OcctBinDir) {
    if ($env:OpenCASCADE_DIR) {
        $PotentialDirs = @(
            (Join-Path $env:OpenCASCADE_DIR "win64\vc14\bin"),
            (Join-Path $env:OpenCASCADE_DIR "bin")
        )
        foreach ($Dir in $PotentialDirs) {
            if (Test-Path $Dir) {
                $OcctBinDir = $Dir
                break
            }
        }
    }
}

$CopiedCount = 0
if ($OcctBinDir -and (Test-Path $OcctBinDir)) {
    Write-Host "==> Copiando DLLs do OCCT de: $OcctBinDir"
    $Dlls = Get-ChildItem -Path $OcctBinDir -Filter "TK*.dll"
    foreach ($Dll in $Dlls) {
        Copy-Item -Path $Dll.FullName -Destination $OutputDir
        $CopiedCount++
    }
    Write-Host "==> DLLs copiadas: $CopiedCount"
} else {
    Write-Warning "Diretório de DLLs do OCCT não encontrado. Se o binário foi compilado com suporte a OCCT, copie as DLLs TK*.dll manualmente para a pasta de distribuição."
}

# Copiar licenças e documentação essencial
$Docs = @("LICENSE", "THIRD_PARTY.md", "README.md")
foreach ($Doc in $Docs) {
    $DocPath = Join-Path $RepoRoot $Doc
    if (Test-Path $DocPath) {
        Copy-Item -Path $DocPath -Destination $OutputDir
    }
}

Write-Host "==> Pacote portátil montado com sucesso em: $OutputDir"

if ($Archive) {
    $ZipPath = "$OutputDir.zip"
    Write-Host "==> Gerando arquivo compactado: $ZipPath"
    Compress-Archive -Path "$OutputDir\*" -DestinationPath $ZipPath -Force
    Write-Host "==> Arquivo zip gerado com sucesso: $ZipPath"
}

Write-Host "==> Concluído!"
