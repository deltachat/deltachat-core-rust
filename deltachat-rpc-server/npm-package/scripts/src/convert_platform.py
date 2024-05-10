def convert_cpu_arch_to_npm_cpu_arch(arch):
    if arch == "x86_64":
        return "x64"
    if arch == "i686":
        return "i32"
    if arch == "aarch64":
        return "arm64"
    if arch == "armv7" or arch == "arm":
        return "arm"
    print("architecture might not be known by nodejs, please make sure it can be returned by 'process.arch':", arch)
    return arch

def convert_os_to_npm_os(os):
    if os == "windows":
        return "win32"
    if os == "darwin" or os == "linux":
        return os
    if os.startswith("android"):
        return "android"
    print("architecture might not be known by nodejs, please make sure it can be returned by 'process.platform':", os)
    return os