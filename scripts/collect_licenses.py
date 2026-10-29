#!/usr/bin/env python3
"""Reúne textos e inventário das crates resolvidas, incluindo fontes embutidas.

O inventário inclui dependências de outras plataformas: não pretende afirmar que
cada uma foi vinculada ao executável. SDKs nativos têm inventários separados.
"""
import argparse
import json
import shutil
import subprocess
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("output", type=Path)
args = parser.parse_args()
metadata = json.loads(subprocess.check_output([
    "cargo", "metadata", "--locked", "--format-version", "1", "--features", "occt"
], text=True))
args.output.mkdir(parents=True, exist_ok=False)
inventory = []
for package in metadata["packages"]:
    if package["source"] is None:
        continue
    root = Path(package["manifest_path"]).parent
    target = args.output / f'{package["name"]}-{package["version"]}'
    texts = set()
    for path in root.rglob("*"):
        if not path.is_file():
            continue
        name = path.name.lower()
        if name.startswith(("license", "licence", "copying", "notice", "copyright")) or name in ("ofl.txt", "ufl.txt", "emoji-icon-font-mit-license.txt", "hack-regular.txt"):
            texts.add(path)
    if package.get("license_file"):
        texts.add(root / package["license_file"])
    for source in sorted(texts):
        relative = source.relative_to(root)
        destination = target / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, destination)
    inventory.append({key: package.get(key) for key in ["name", "version", "license", "repository", "source"]} | {"texts": [str(p.relative_to(root)) for p in sorted(texts)]})
(args.output / "inventory.json").write_text(json.dumps(inventory, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
print(f"Inventário e textos de {len(inventory)} crates em {args.output}.")
