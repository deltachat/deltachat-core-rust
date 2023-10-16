#!/usr/bin/env python3
"""Build Python wheels for deltachat-rpc-server.
Run scripts/zig-rpc-server.sh first."""
from pathlib import Path
from wheel.wheelfile import WheelFile
import tomllib


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
            f"""Metadata-Version: 2.1
Name: deltachat-rpc-server
Version: {version}
Summary: Delta Chat JSON-RPC server
""",
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
    build_wheel(
        version,
        "target/x86_64-unknown-linux-musl/release/deltachat-rpc-server",
        "py3-none-linux_x86_64.manylinux_2_17_x86_64.manylinux2014_x86_64.musllinux_1_1_x86_64",
    )
    build_wheel(
        version,
        "target/armv7-unknown-linux-musleabihf/release/deltachat-rpc-server",
        "py3-none-linux_armv7l.manylinux_2_17_armv7l.manylinux2014_armv7l.musllinux_1_1_armv7l",
    )
    build_wheel(
        version,
        "target/aarch64-unknown-linux-musl/release/deltachat-rpc-server",
        "py3-none-linux_aarch64.manylinux_2_17_aarch64.manylinux2014_aarch64.musllinux_1_1_aarch64",
    )
    build_wheel(
        version,
        "target/i686-unknown-linux-musl/release/deltachat-rpc-server",
        "py3-none-linux_i686.manylinux_2_12_i686.manylinux2010_i686.musllinux_1_1_i686",
    )


main()
