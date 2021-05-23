
import os
import platform
import subprocess
import sys


if __name__ == "__main__":
    assert len(sys.argv) == 2
    workspacedir = sys.argv[1]
    arch = platform.machine()
    for relpath in os.listdir(workspacedir):
        if relpath.startswith("deltachat"):
            p = os.path.join(workspacedir, relpath)
            subprocess.check_call(
                ["auditwheel", "repair", p, "-w", workspacedir,
                 "--plat", "manylinux2014_" + arch])
