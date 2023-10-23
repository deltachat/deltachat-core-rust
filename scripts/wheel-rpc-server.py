#!/usr/bin/env python3
"""Build Python wheels for deltachat-rpc-server.
Run scripts/zig-rpc-server.sh first."""
from pathlib import Path
from wheel.wheelfile import WheelFile
import tomllib
import tarfile
from io import BytesIO


def metadata_contents(version):
    return f"""Metadata-Version: 2.1
Name: deltachat-rpc-server
Version: {version}
Summary: Delta Chat JSON-RPC server
"""


def build_source_package(version):
    filename = f"dist/deltachat-rpc-server-{version}.tar.gz"

    with tarfile.open(filename, "w:gz") as pkg:

        def pack(name, contents):
            contents = contents.encode()
            tar_info = tarfile.TarInfo(f"deltachat-rpc-server-{version}/{name}")
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


def build_wheel(version, binary, tag):
    filename = f"dist/deltachat_rpc_server-{version}-{tag}.whl"

    with WheelFile(filename, "w") as wheel:
        wheel.write("LICENSE", "deltachat_rpc_server/LICENSE")
        wheel.write("deltachat-rpc-server/README.md", "deltachat_rpc_server/README.md")
        wheel.writestr(
            "deltachat_rpc_server/__init__.py",
            """import os, sys
def main():
    argv = [os.path.join(os.path.dirname(__file__), "deltachat-rpc-server"), *sys.argv[1:]]
    os.execv(argv[0], argv)
""",
        )

        wheel.write(
            binary,
            "deltachat_rpc_server/deltachat-rpc-server",
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


def main():
    with open("deltachat-rpc-server/Cargo.toml", "rb") as f:
        cargo_toml = tomllib.load(f)
        version = cargo_toml["package"]["version"]
    Path("dist").mkdir(exist_ok=True)
    build_source_package(version)
    build_wheel(
        version,
        "dist/deltachat-rpc-server-x86_64-linux",
        "py3-none-manylinux_2_17_x86_64.manylinux2014_x86_64.musllinux_1_1_x86_64",
    )
    build_wheel(
        version,
        "dist/deltachat-rpc-server-armv7-linux",
        "py3-none-manylinux_2_17_armv7l.manylinux2014_armv7l.musllinux_1_1_armv7l",
    )
    build_wheel(
        version,
        "dist/deltachat-rpc-server-aarch64-linux",
        "py3-none-manylinux_2_17_aarch64.manylinux2014_aarch64.musllinux_1_1_aarch64",
    )
    build_wheel(
        version,
        "dist/deltachat-rpc-server-i686-linux",
        "py3-none-manylinux_2_12_i686.manylinux2010_i686.musllinux_1_1_i686",
    )

    # macOS versions for platform compatibility tags are taken from https://doc.rust-lang.org/rustc/platform-support.html
    build_wheel(
        version,
        "dist/deltachat-rpc-server-x86_64-macos",
        "py3-none-macosx_10_7_x86_64",
    )
    build_wheel(
        version,
        "dist/deltachat-rpc-server-aarch64-macos",
        "py3-none-macosx_11_0_arm64",
    )


main()
