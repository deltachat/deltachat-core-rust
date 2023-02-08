import distutils.ccompiler
import distutils.log
import distutils.sysconfig
import os
import platform
import re
import shutil
import subprocess
import tempfile
import textwrap

import cffi
import pkgconfig  # type: ignore


def local_build_flags(projdir, target):
    """Construct build flags for building against a checkout.

    :param projdir: The root directory of the deltachat-core-rust project.
    :param target: The rust build target, `debug` or `release`.
    """
    flags = {}
    if platform.system() == "Darwin":
        flags["libraries"] = ["resolv", "dl"]
        flags["extra_link_args"] = [
            "-framework",
            "CoreFoundation",
            "-framework",
            "CoreServices",
            "-framework",
            "Security",
        ]
    elif platform.system() == "Linux":
        flags["libraries"] = ["rt", "dl", "m"]
        flags["extra_link_args"] = []
    else:
        raise NotImplementedError("Compilation not supported yet on Windows, can you help?")
    target_dir = os.environ.get("CARGO_TARGET_DIR")
    if target_dir is None:
        target_dir = os.path.join(projdir, "target")
    flags["extra_objects"] = [os.path.join(target_dir, target, "libdeltachat.a")]
    assert os.path.exists(flags["extra_objects"][0]), flags["extra_objects"]
    flags["include_dirs"] = [os.path.join(projdir, "deltachat-ffi")]
    return flags


def system_build_flags():
    """Construct build flags for building against an installed libdeltachat."""
    return pkgconfig.parse("deltachat")


def extract_functions(flags):
    """Extract the function definitions from deltachat.h.

    This creates a .h file with a single `#include <deltachat.h>` line
    in it.  It then runs the C preprocessor to create an output file
    which contains all function definitions found in `deltachat.h`.
    """
    distutils.log.set_verbosity(distutils.log.INFO)
    cc = distutils.ccompiler.new_compiler(force=True)
    distutils.sysconfig.customize_compiler(cc)
    tmpdir = tempfile.mkdtemp()
    try:
        src_name = os.path.join(tmpdir, "include.h")
        dst_name = os.path.join(tmpdir, "expanded.h")
        with open(src_name, "w") as src_fp:
            src_fp.write("#include <deltachat.h>")
        cc.preprocess(
            source=src_name,
            output_file=dst_name,
            include_dirs=flags["include_dirs"],
            macros=[("PY_CFFI", "1")],
        )
        with open(dst_name, "r") as dst_fp:
            return dst_fp.read()
    finally:
        shutil.rmtree(tmpdir)


def find_header(flags):
    """Use the compiler to find the deltachat.h header location.

    This uses a small utility in deltachat.h to find the location of
    the header file location.
    """
    distutils.log.set_verbosity(distutils.log.INFO)
    cc = distutils.ccompiler.new_compiler(force=True)
    distutils.sysconfig.customize_compiler(cc)
    tmpdir = tempfile.mkdtemp()
    try:
        src_name = os.path.join(tmpdir, "where.c")
        obj_name = os.path.join(tmpdir, "where.o")
        dst_name = os.path.join(tmpdir, "where")
        with open(src_name, "w") as src_fp:
            src_fp.write(
                textwrap.dedent(
                    """
                #include <stdio.h>
                #include <deltachat.h>

                int main(void) {
                    printf("%s", _dc_header_file_location());
                    return 0;
                }
            """,
                ),
            )
        cwd = os.getcwd()
        try:
            os.chdir(tmpdir)
            cc.compile(
                sources=["where.c"],
                include_dirs=flags["include_dirs"],
                macros=[("PY_CFFI_INC", "1")],
            )
        finally:
            os.chdir(cwd)
        cc.link_executable(objects=[obj_name], output_progname="where", output_dir=tmpdir)
        return subprocess.check_output(dst_name)
    finally:
        shutil.rmtree(tmpdir)


def extract_defines(flags):
    """Extract the required #DEFINEs from deltachat.h.

    Since #DEFINEs are interpreted by the C preprocessor we can not
    use the compiler to extract these and need to parse the header
    file ourselves.

    The defines are returned in a string that can be passed to CFFIs
    cdef() method.
    """
    header = find_header(flags)
    defines_re = re.compile(
        r"""
        \#define\s+   # The start of a define.
        (             # Begin capturing group which captures the define name.
            (?:       # A nested group which is not captured, this allows us
                      #    to build the list of prefixes to extract without
                      #    creation another capture group.
                DC_EVENT
                | DC_QR
                | DC_MSG
                | DC_LP
                | DC_EMPTY
                | DC_CERTCK
                | DC_STATE
                | DC_STR
                | DC_CONTACT_ID
                | DC_GCL
                | DC_GCM
                | DC_SOCKET
                | DC_CHAT
                | DC_PROVIDER
                | DC_KEY_GEN
                | DC_IMEX
                | DC_CONNECTIVITY
                | DC_DOWNLOAD
            )         # End of prefix matching
            _[\w_]+   # Match the suffix, e.g. _RSA2048 in DC_KEY_GEN_RSA2048
        )             # Close the capturing group, this contains
                      # the entire name e.g. DC_MSG_TEXT.
        \s+\S+        # Ensure there is whitespace followed by a value.
    """,
        re.VERBOSE,
    )
    defines = []
    with open(header) as fp:
        for line in fp:
            match = defines_re.match(line)
            if match:
                defines.append(match.group(1))
    return "\n".join(f"#define {d} ..." for d in defines)


def ffibuilder():
    projdir = os.environ.get("DCC_RS_DEV")
    if projdir:
        target = os.environ.get("DCC_RS_TARGET", "release")
        flags = local_build_flags(projdir, target)
    else:
        flags = system_build_flags()
    builder = cffi.FFI()
    builder.set_source(
        "deltachat.capi",
        """
            #include <deltachat.h>
            int dc_event_has_string_data(int e)
            {
                return DC_EVENT_DATA2_IS_STRING(e);
            }
        """,
        **flags,
    )
    builder.cdef(
        """
        typedef int... time_t;
        void free(void *ptr);
        extern int dc_event_has_string_data(int);
    """,
    )
    function_defs = extract_functions(flags)
    defines = extract_defines(flags)
    builder.cdef(function_defs)
    builder.cdef(defines)
    return builder


if __name__ == "__main__":
    import os.path

    pkgdir = os.path.join(os.path.dirname(__file__), "..")
    builder = ffibuilder()
    builder.compile(tmpdir=pkgdir, verbose=True)
