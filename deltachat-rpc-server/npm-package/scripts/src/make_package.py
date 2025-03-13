import tomllib
import json

from .convert_platform import convert_cpu_arch_to_npm_cpu_arch, convert_os_to_npm_os

def write_package_json(platform_path, rust_target, my_binary_name):
    if len(rust_target.split("-")) == 3:
        [cpu_arch, vendor, os] = rust_target.split("-")
    else:
        [cpu_arch, vendor, os, _env] = rust_target.split("-")

    # read version
    tomlfile = open("../../Cargo.toml", 'rb')
    version = tomllib.load(tomlfile)['package']['version']

    package_json = {
        "name": "@deltachat/stdio-rpc-server-"
        + convert_os_to_npm_os(os)
        + "-"
        + convert_cpu_arch_to_npm_cpu_arch(cpu_arch),
        "version": version,
        "os": [convert_os_to_npm_os(os)],
        "cpu": [convert_cpu_arch_to_npm_cpu_arch(cpu_arch)],
        "main": my_binary_name,
        "license": "MPL-2.0",
        "repository": {
            "type": "git",
            "url": "https://github.com/chatmail/core.git",
        },
    }

    file = open(platform_path + "/package.json", 'w')
    file.write(json.dumps(package_json, indent=4))

