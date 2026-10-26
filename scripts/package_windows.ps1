<#
.SYNOPSIS
    Cria pacote Windows em destino novo. Requer dumpbin do SDK MSVC.
    Caminhos informados são relativos ao diretório de invocação.
#>
[CmdletBinding()]
param (
    [string]$BinPath,
    [string]$OcctBinDir,
    [string]$OutputDir,
    [switch]$Archive
)
$ErrorActionPreference = "Stop"
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
if (-not $BinPath) { $BinPath = Join-Path $RepoRoot "target\release\nator-prototype.exe" }
if (-not (Test-Path -LiteralPath $BinPath -PathType Leaf)) { throw "Executável ausente: $BinPath." }
$BinPath = (Resolve-Path -LiteralPath $BinPath).Path
if (-not $OutputDir) { $OutputDir = Join-Path $RepoRoot "dist\3dnator-windows-x64" }
$OutputDir = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($OutputDir)
if (Test-Path -LiteralPath $OutputDir) { throw "O destino já existe: $OutputDir. Escolha um diretório novo." }
if ($Archive -and (Test-Path -LiteralPath "$OutputDir.zip")) { throw "O arquivo $OutputDir.zip já existe." }
$Dumpbin = (Get-Command dumpbin.exe -ErrorAction Stop).Source

# Inclui o layout conda usado pela CI, além dos SDKs convencionais.
if (-not $OcctBinDir) {
    $Candidates = @()
    if ($env:OpenCASCADE_DIR) {
        $Candidates += (Join-Path $env:OpenCASCADE_DIR "win64\vc14\bin")
        $Candidates += (Join-Path $env:OpenCASCADE_DIR "bin")
        $Candidates += (Join-Path $env:OpenCASCADE_DIR "..\..\..\bin")
    }
    if ($env:CONDA_PREFIX) { $Candidates += (Join-Path $env:CONDA_PREFIX "Library\bin") }
    $Candidates += $env:PATH.Split([IO.Path]::PathSeparator)
    foreach ($Candidate in $Candidates) {
        if (-not $Candidate) { continue }
        if (Test-Path -LiteralPath (Join-Path $Candidate "TKernel.dll") -PathType Leaf) {
            $OcctBinDir = $Candidate
            break
        }
    }
}
if (-not $OcctBinDir -or -not (Test-Path -LiteralPath $OcctBinDir -PathType Container)) {
    throw "Diretório de DLLs OCCT ausente. Informe -OcctBinDir."
}
$OcctBinDir = (Resolve-Path -LiteralPath $OcctBinDir).Path
$SearchDirs = @((Split-Path $BinPath), $OcctBinDir) + $env:PATH.Split([IO.Path]::PathSeparator)
$SystemDir = [Environment]::GetFolderPath("System")

# Percorrer os imports reais, incluindo TBB, FreeType e dependências do SDK.
# Copiar apenas TK*.dll deixava o pacote dependente do PATH do conda.
$Pending = [Collections.Generic.Queue[string]]::new()
$Pending.Enqueue($BinPath)
$Libraries = @{}
$HostLibraries = @{}
while ($Pending.Count -gt 0) {
    $Current = $Pending.Dequeue()
    $Imports = & $Dumpbin /nologo /dependents $Current
    if ($LASTEXITCODE -ne 0) { throw "Falha ao analisar $Current com dumpbin." }
    foreach ($Line in $Imports) {
        if ($Line.Trim() -notmatch '^[\w.+-]+\.dll$') { continue }
        $Name = $Line.Trim()
        if ($Libraries.ContainsKey($Name) -or $HostLibraries.ContainsKey($Name)) { continue }
        # API sets são contratos do carregador Windows, não arquivos do SDK.
        if ($Name -match '^(api|ext)-ms-') { $HostLibraries[$Name] = $true; continue }
        if (Test-Path -LiteralPath (Join-Path $SystemDir $Name) -PathType Leaf) {
            $HostLibraries[$Name] = $true
            continue
        }
        $Found = $null
        foreach ($Dir in $SearchDirs) {
            if (-not $Dir) { continue }
            $Candidate = Join-Path $Dir $Name
            if (Test-Path -LiteralPath $Candidate -PathType Leaf) { $Found = (Resolve-Path -LiteralPath $Candidate).Path; break }
        }
        if (-not $Found) { throw "Dependência $Name de $Current não encontrada. Configure o SDK/PATH." }
        $Libraries[$Name] = $Found
        $Pending.Enqueue($Found)
    }
}
if (-not $Libraries.ContainsKey("TKernel.dll")) { throw "O executável não possui vínculo dinâmico com OCCT." }
$Docs = @("LICENSE", "THIRD_PARTY.md", "docs\PACKAGE_README.md", "licenses\OCCT\LICENSE_LGPL_21.txt", "licenses\OCCT\OCCT_LGPL_EXCEPTION.txt")
foreach ($Doc in $Docs) {
    if (-not (Test-Path -LiteralPath (Join-Path $RepoRoot $Doc) -PathType Leaf)) { throw "Documento obrigatório ausente: $Doc." }
}
# Não remover nem sobrescrever um destino recebido do usuário.
$Parent = Split-Path $OutputDir
New-Item -ItemType Directory -Force -Path $Parent | Out-Null
New-Item -ItemType Directory -Path $OutputDir | Out-Null
Copy-Item -LiteralPath $BinPath -Destination (Join-Path $OutputDir "3dnator.exe")
foreach ($Entry in $Libraries.GetEnumerator()) {
    Copy-Item -LiteralPath $Entry.Value -Destination (Join-Path $OutputDir $Entry.Key)
}
foreach ($Doc in @("LICENSE", "THIRD_PARTY.md", "licenses")) {
    Copy-Item -LiteralPath (Join-Path $RepoRoot $Doc) -Destination $OutputDir -Recurse
}
Copy-Item -LiteralPath (Join-Path $RepoRoot "docs\PACKAGE_README.md") -Destination (Join-Path $OutputDir "README.md")
@("DLLs incluídas:") + @($Libraries.Keys | Sort-Object) + @("DLLs fornecidas pelo Windows/runtime MSVC:") + @($HostLibraries.Keys | Sort-Object) |
    Set-Content -LiteralPath (Join-Path $OutputDir "NATIVE_DEPENDENCIES.txt") -Encoding utf8
if ($Archive) { Compress-Archive -LiteralPath $OutputDir -DestinationPath "$OutputDir.zip" }
Write-Host "Pacote criado em $OutputDir ($($Libraries.Count) DLLs)."
