
import os
import sys
import subprocess


if __name__ == "__main__":
    assert len(sys.argv) == 2
    workspacedir = sys.argv[1]
    for relpath in os.listdir(workspacedir):
        if relpath.startswith("deltachat"):
            p = os.path.join(workspacedir, relpath)
            subprocess.check_call(["auditwheel", "repair", p, "-w", workspacedir])
