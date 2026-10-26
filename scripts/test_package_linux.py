"""Regressões do pacote: preservar arquivos e resolver um SDK fora do sistema."""
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().with_name("package_linux.sh")


class PackageTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="3dnator-package-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.sdk = self.root / "sdk com espaços"
        self.sdk.mkdir()
        self.binary = self.root / "aplicativo"
        self.output = self.root / "pacote novo"
        # SDK mínimo real: o toolkit depende de uma biblioteca TBB transitiva.
        # Nenhuma das duas está disponível no carregador do sistema.
        sources = {
            "thread.c": "int thread_value(void) { return 42; }",
            "toolkit.c": "int thread_value(void); int fixture(void) { return thread_value(); }",
            "main.c": '#include <stdio.h>\nint fixture(void); int main(void) { puts("3dnator fixture"); return fixture() != 42; }',
        }
        for name, content in sources.items():
            (self.root / name).write_text(content)
        self.compile("-shared", "-fPIC", "thread.c", "-Wl,-soname,libtbbFixture.so.1", "-o", str(self.sdk / "libtbbFixture.so.1.2"))
        self.compile("-shared", "-fPIC", "toolkit.c", str(self.sdk / "libtbbFixture.so.1.2"), "-Wl,-soname,libTKFixture.so.1", "-o", str(self.sdk / "libTKFixture.so.1.2"))
        for name in ("libtbbFixture.so", "libTKFixture.so"):
            (self.sdk / (name + ".1")).symlink_to(name + ".1.2")
        self.compile("main.c", str(self.sdk / "libTKFixture.so.1.2"), "-Wl,-rpath-link," + str(self.sdk), "-o", str(self.binary))

    def compile(self, *args):
        subprocess.run(["cc", *args], cwd=self.root, check=True, capture_output=True)

    def package(self, *extra, sdk=True):
        command = ["bash", str(SCRIPT), "--bin", "aplicativo", "--output", self.output.name]
        if sdk:
            command += ["--occt-lib-dir", self.sdk.name]
        return subprocess.run(command + list(extra), cwd=self.root, env={"PATH": os.environ["PATH"]}, capture_output=True, text=True)

    def test_relocated_bundle_runs_without_sdk_and_carries_licenses(self):
        result = self.package("--archive")
        self.assertEqual(result.returncode, 0, result.stderr)
        for name in ("libtbbFixture.so.1", "libTKFixture.so.1"):
            self.assertTrue((self.output / "lib" / name).is_file())
            self.assertFalse((self.output / "lib" / name).is_symlink())
        self.assertTrue((self.output / "licenses/OCCT/LICENSE_LGPL_21.txt").is_file())
        self.assertTrue((self.output / "licenses/OCCT/OCCT_LGPL_EXCEPTION.txt").is_file())
        with tarfile.open(str(self.output) + ".tar.gz") as archive:
            self.assertIn(self.output.name + "/licenses/OCCT/LICENSE_LGPL_21.txt", archive.getnames())
        moved = self.root / "relocado"
        self.output.rename(moved)
        self.sdk.rename(self.root / "sdk indisponível")
        run = subprocess.run([str(moved / "3dnator"), "--version"], env={"PATH": "/usr/bin:/bin"}, capture_output=True)
        self.assertEqual(run.returncode, 0, run.stderr)

    def test_existing_output_is_preserved(self):
        self.output.mkdir()
        sentinel = self.output / "arquivo do usuário"
        sentinel.write_text("preservar")
        result = self.package()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(sentinel.read_text(), "preservar")
        self.assertEqual(list(self.output.iterdir()), [sentinel])

    def test_existing_archive_is_preserved_without_creating_output(self):
        archive = Path(str(self.output) + ".tar.gz")
        archive.write_bytes(b"preservar")
        self.assertNotEqual(self.package("--archive").returncode, 0)
        self.assertEqual(archive.read_bytes(), b"preservar")
        self.assertFalse(self.output.exists())

    def test_unresolved_dependency_does_not_create_output(self):
        result = self.package(sdk=False)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Dependências não encontradas", result.stderr)
        self.assertFalse(self.output.exists())

    def test_missing_option_value_has_readable_error(self):
        result = self.package("--occt-lib-dir")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Falta o valor", result.stderr)
        self.assertFalse(self.output.exists())


if __name__ == "__main__":
    unittest.main()
