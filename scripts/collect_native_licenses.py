#!/usr/bin/env python3
"""Reúne os avisos disponibilizados pelo SDK, com o inventário de pacotes fonte.

Ubuntu: copyrights instalados dos pacotes e licenças comuns. Conda: metadados e
textos do cache dos pacotes. O conjunto é deliberadamente maior que as DLLs/so
copiadas; NATIVE_DEPENDENCIES.txt identifica as bibliotecas efetivas do pacote.
"""
import argparse
import json
import shutil
import subprocess
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("output", type=Path)
parser.add_argument("--conda-prefix", type=Path)
parser.add_argument("--conda-info", type=Path)
args = parser.parse_args()
args.output.mkdir(parents=True, exist_ok=False)
if args.conda_prefix:
    # No Bash Windows, conda pode ser uma função/batch, indisponível ao
    # CreateProcess do Python. O shell fornece o JSON, sem shell=True.
    if args.conda_info is None:
        parser.error("Informe --conda-info com a saída de conda info --json.")
    cache = json.loads(args.conda_info.read_text(encoding="utf-8-sig"))["pkgs_dirs"]
    for metadata in (args.conda_prefix / "conda-meta").glob("*.json"):
        target = args.output / metadata.stem
        target.mkdir()
        shutil.copyfile(metadata, target / "package.json")
        for directory in cache:
            source = Path(directory) / metadata.stem / "info/licenses"
            if source.is_dir():
                shutil.copytree(source, target / "texts", dirs_exist_ok=True)
else:
    for source in Path("/usr/share/doc").glob("*/copyright"):
        target = args.output / source.parent.name
        target.mkdir()
        shutil.copyfile(source, target / "copyright")
    common = Path("/usr/share/common-licenses")
    if common.is_dir():
        shutil.copytree(common, args.output / "common-licenses")
    (args.output / "packages.txt").write_bytes(subprocess.check_output(["dpkg-query", "-W"]))
