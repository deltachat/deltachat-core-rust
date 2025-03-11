#!/usr/bin/env python3
"""Build Python wheels for deltachat-rpc-server."""
from pathlib import Path
from wheel.wheelfile import WheelFile
import tomllib
import tarfile
import sys
from io import BytesIO


def metadata_contents(version):
    readme_text = (Path("deltachat-rpc-server") / "README.md").read_text()
    return f"""Metadata-Version: 2.1
Name: deltachat-rpc-server
Version: {version}
Summary: Delta Chat JSON-RPC server
Description-Content-Type: text/markdown

{readme_text}
"""


def build_source_package(version, filename):
    with tarfile.open(filename, "w:gz") as pkg:

        def pack(name, contents):
            contents = contents.encode()
            tar_info = tarfile.TarInfo(f"deltachat_rpc_server-{version}/{name}")
            tar_info.mode = 0o644
            tar_info.size = len(contents)
            pkg.addfile(tar_info, BytesIO(contents))

        pack("PKG-INFO", metadata_contents(version))
        pack(
            "pyproject.toml",
            f"""[build-system]
requires = ["setuptools==68.2.2", "pip"]
build-backend = "setuptools.build_meta"

[project]
name = "deltachat-rpc-server"
version = "{version}"

[project.scripts]
deltachat-rpc-server = "deltachat_rpc_server:main"
""",
        )
        pack(
            "setup.py",
            f"""
import sys
from setuptools import setup, find_packages
from distutils.cmd import Command
from setuptools.command.install import install
from setuptools.command.build import build
import subprocess
import platform
import tempfile
from zipfile import ZipFile
from pathlib import Path
import shutil


class BuildCommand(build):
    def run(self):
        tmpdir = tempfile.mkdtemp()
        subprocess.run(
            [
                sys.executable,
                "-m",
                "pip",
                "download",
                "--no-input",
                "--timeout",
                "1000",
                "--platform",
                "musllinux_1_1_" + platform.machine(),
                "--only-binary=:all:",
                "deltachat-rpc-server=={version}",
            ],
            cwd=tmpdir,
        )

        wheel_path = next(Path(tmpdir).glob("*.whl"))
        with ZipFile(wheel_path, "r") as wheel:
            exe_path = wheel.extract("deltachat_rpc_server/deltachat-rpc-server", "src")
            Path(exe_path).chmod(0o700)
            wheel.extract("deltachat_rpc_server/__init__.py", "src")

        shutil.rmtree(tmpdir)
        return super().run()


setup(
    cmdclass={{"build": BuildCommand}},
    package_data={{"deltachat_rpc_server": ["deltachat-rpc-server"]}},
)
""",
        )
        pack("src/deltachat_rpc_server/__init__.py", "")


def build_wheel(version, binary, tag, windows=False):
    filename = f"deltachat_rpc_server-{version}-{tag}.whl"

    with WheelFile(filename, "w") as wheel:
        wheel.write("LICENSE", "deltachat_rpc_server/LICENSE")
        wheel.write("deltachat-rpc-server/README.md", "deltachat_rpc_server/README.md")
        if windows:
            wheel.writestr(
                "deltachat_rpc_server/__init__.py",
                """import os, sys, subprocess
def main():
    argv = [os.path.join(os.path.dirname(__file__), "deltachat-rpc-server.exe"), *sys.argv[1:]]
    sys.exit(subprocess.call(argv))
""",
            )
        else:
            wheel.writestr(
                "deltachat_rpc_server/__init__.py",
                """import os, sys
def main():
    argv = [os.path.join(os.path.dirname(__file__), "deltachat-rpc-server"), *sys.argv[1:]]
    os.execv(argv[0], argv)
""",
            )

        Path(binary).chmod(0o755)
        wheel.write(
            binary,
            (
                "deltachat_rpc_server/deltachat-rpc-server.exe"
                if windows
                else "deltachat_rpc_server/deltachat-rpc-server"
            ),
        )
        wheel.writestr(
            f"deltachat_rpc_server-{version}.dist-info/METADATA",
            metadata_contents(version),
        )
        wheel.writestr(
            f"deltachat_rpc_server-{version}.dist-info/WHEEL",
            "Wheel-Version: 1.0\nRoot-Is-Purelib: false\nTag: {tag}",
        )
        wheel.writestr(
            f"deltachat_rpc_server-{version}.dist-info/entry_points.txt",
            "[console_scripts]\ndeltachat-rpc-server = deltachat_rpc_server:main",
        )


arch2tags = {
    "x86_64-linux": "manylinux_2_17_x86_64.manylinux2014_x86_64.musllinux_1_1_x86_64",
    "armv7l-linux": "linux_armv7l.manylinux_2_17_armv7l.manylinux2014_armv7l.musllinux_1_1_armv7l",
    "armv6l-linux": "linux_armv6l",
    "aarch64-linux": "manylinux_2_17_aarch64.manylinux2014_aarch64.musllinux_1_1_aarch64",
    "i686-linux": "manylinux_2_12_i686.manylinux2010_i686.musllinux_1_1_i686",
    "arm64-v8a-android": "android_21_arm64_v8a",
    "armeabi-v7a-android": "android_21_armeabi_v7a",
    "win64": "win_amd64",
    "win32": "win32",
    # macOS versions for platform compatibility tags are taken from https://doc.rust-lang.org/rustc/platform-support.html
    "x86_64-darwin": "macosx_10_7_x86_64",
    "aarch64-darwin": "macosx_11_0_arm64",
}


def main():
    with Path("Cargo.toml").open("rb") as fp:
        cargo_manifest = tomllib.load(fp)
    version = cargo_manifest["package"]["version"]
    if sys.argv[1] == "source":
        filename = f"deltachat_rpc_server-{version}.tar.gz"
        build_source_package(version, filename)
    else:
        arch = sys.argv[1]
        executable = sys.argv[2]
        tags = arch2tags[arch]

        if arch in ["win32", "win64"]:
            build_wheel(
                version,
                executable,
                f"py3-none-{tags}",
                windows=True,
            )
        else:
            build_wheel(version, executable, f"py3-none-{tags}")


main()
