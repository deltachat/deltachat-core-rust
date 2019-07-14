
import os
import sys
import subprocess


if __name__ == "__main__":
    assert len(sys.argv) == 2
    wheelhousedir = sys.argv[1]
    # pip wheel will build in an isolated tmp dir that does not have git
    # history so setuptools_scm can not automatically determine a
    # version there. So pass in the version through an env var.
    version = subprocess.check_output(["python", "setup.py", "--version"]).strip().split(b"\n")[-1]
    os.environ["SETUPTOOLS_SCM_PRETEND_VERSION"] = version.decode("ascii")
    subprocess.check_call(("pip wheel . -w %s" % wheelhousedir).split())
