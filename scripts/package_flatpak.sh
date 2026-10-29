#!/usr/bin/env bash
# Gera um bundle Flatpak a partir do executável OCCT já compilado. A release usa
# Ubuntu 24.04, cuja ABI é anterior ao runtime Freedesktop 25.08; a execução real
# no runtime é obrigatória, pois ldd no hospedeiro não prova compatibilidade.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
[[ $# == 4 ]] || { echo "Uso: $0 EXECUTAVEL SAIDA.flatpak VERSAO LICENCAS" >&2; exit 1; }
BIN_PATH="$(realpath -e -- "$1")"
OUTPUT_PATH="$(realpath -m -- "$2")"
APP_VERSION="$3"
LICENSE_DIR="$(realpath -e -- "$4")"
[[ "$APP_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]] || { echo 'Versão inválida.' >&2; exit 1; }
[[ ! -e "$OUTPUT_PATH" && ! -e "$OUTPUT_PATH.work" ]] || { echo 'Destino já existe.' >&2; exit 1; }
APP_ID=io.github.trocolis.ThreeDnator
RUNTIME_VERSION=25.08
RUNTIME_DIR="$(flatpak info --show-location "org.freedesktop.Platform//$RUNTIME_VERSION")"
BUILD_DIR="$OUTPUT_PATH.work/build"
mkdir -p -- "$(dirname "$OUTPUT_PATH")"
flatpak build-init "$BUILD_DIR" "$APP_ID" org.freedesktop.Sdk org.freedesktop.Platform "$RUNTIME_VERSION"
mkdir -p "$BUILD_DIR/files/bin" "$BUILD_DIR/files/libexec" "$BUILD_DIR/files/lib" "$BUILD_DIR/files/share/3dnator"
install -m755 "$BIN_PATH" "$BUILD_DIR/files/libexec/3dnator"
cat > "$BUILD_DIR/files/bin/3dnator" <<'LAUNCHER'
#!/bin/sh
export LD_LIBRARY_PATH="/app/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
exec /app/libexec/3dnator "$@"
LAUNCHER
chmod 755 "$BUILD_DIR/files/bin/3dnator"
# Inclui OCCT e somente as dependências ausentes do runtime. Não copia glibc,
# carregador ELF nem drivers gráficos do hospedeiro para o sandbox.
python3 - "$BIN_PATH" "$RUNTIME_DIR/files" "$BUILD_DIR/files" <<'PY'
import re
import shutil
import subprocess
import sys
from pathlib import Path
binary, runtime, app = map(Path, sys.argv[1:])
text = subprocess.check_output(["ldd", str(binary)], text=True)
if "not found" in text:
    raise SystemExit("O executável contém bibliotecas não encontradas.")
manifest = []
for line in text.splitlines():
    match = re.match(r"\s*(\S+)\s+=>\s+(.+?)\s+\(0x[0-9a-f]+\)", line)
    if not match:
        continue
    name, source = match.groups()
    supplied = any((root / name).exists() for root in [runtime / "lib", runtime / "lib/x86_64-linux-gnu"])
    bundled = name.startswith("libTK") or not supplied
    if bundled:
        if name.startswith(("libc.so", "libstdc++", "libgcc_s", "ld-linux")):
            raise SystemExit(f"Runtime incompatível: {name} ausente.")
        shutil.copyfile(Path(source).resolve(), app / "lib" / name)
    manifest.append(f'{name}: {"incluída" if bundled else "runtime Freedesktop 25.08"}')
if not any(p.name.startswith("libTKernel") for p in (app / "lib").iterdir()):
    raise SystemExit("A pré-release Flatpak exige OCCT dinâmico.")
(app / "share/3dnator/NATIVE_DEPENDENCIES.txt").write_text("\n".join(manifest)+"\n", encoding="utf-8")
PY
cp "$REPO_ROOT/LICENSE" "$REPO_ROOT/THIRD_PARTY.md" "$REPO_ROOT/docs/PACKAGE_README.md" "$BUILD_DIR/files/share/3dnator/"
cp -R "$REPO_ROOT/licenses" "$BUILD_DIR/files/share/3dnator/"
cp -R "$LICENSE_DIR" "$BUILD_DIR/files/share/3dnator/licenses/dependencies"
install -Dm644 "$REPO_ROOT/packaging/flatpak/$APP_ID.desktop" "$BUILD_DIR/files/share/applications/$APP_ID.desktop"
install -Dm644 "$REPO_ROOT/packaging/flatpak/$APP_ID.svg" "$BUILD_DIR/files/share/icons/hicolor/scalable/apps/$APP_ID.svg"
mkdir -p "$BUILD_DIR/files/share/metainfo"
cat > "$BUILD_DIR/files/share/metainfo/$APP_ID.metainfo.xml" <<XML
<?xml version="1.0" encoding="UTF-8"?>
<component type="desktop-application">
  <id>$APP_ID</id>
  <metadata_license>CC0-1.0</metadata_license>
  <project_license>MIT</project_license>
  <name>3Dnator</name>
  <summary>Parametric CAD for mechanical parts</summary>
  <summary xml:lang="pt_BR">CAD paramétrico para peças mecânicas</summary>
  <description><p>Create sketches, model solids, paint surfaces and exchange STEP and STL files.</p></description>
  <launchable type="desktop-id">$APP_ID.desktop</launchable>
  <url type="homepage">https://github.com/trocolis/3dnator</url>
  <developer id="io.github.trocolis"><name>3Dnator</name></developer>
  <content_rating type="oars-1.1"/>
  <releases><release version="$APP_VERSION" date="$(date -u +%F)"/></releases>
</component>
XML
flatpak build-finish --command=3dnator --share=ipc --socket=wayland --socket=fallback-x11 --device=dri --filesystem=host "$BUILD_DIR"
# Os testes usam o Platform, não o Sdk, e uma pasta nova dentro da área de build.
# Não instalam o aplicativo nem alteram preferências reais do usuário.
flatpak build --runtime "$BUILD_DIR" /app/bin/3dnator --version
flatpak build --runtime "$BUILD_DIR" /app/bin/3dnator --validate /app/share/3dnator/validation/kernel
flatpak build --runtime "$BUILD_DIR" /app/bin/3dnator --validate-part /app/share/3dnator/validation/part
flatpak build --runtime "$BUILD_DIR" /app/bin/3dnator --validate-step /app/share/3dnator/validation/step
flatpak build-export "$OUTPUT_PATH.work/repo" "$BUILD_DIR" alpha
flatpak build-bundle --runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo "$OUTPUT_PATH.work/repo" "$OUTPUT_PATH" "$APP_ID" alpha
