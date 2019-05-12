import distutils.ccompiler
import distutils.sysconfig
import tempfile
import cffi


def ffibuilder():
    builder = cffi.FFI()
    builder.set_source(
        'deltachat.capi',
        """
            #include <deltachat/deltachat.h>
            const char * dupstring_helper(const char* string)
            {
                return strdup(string);
            }
            int dc_get_event_signature_types(int e)
            {
                int result = 0;
                if (DC_EVENT_DATA1_IS_STRING(e))
                    result |= 1;
                if (DC_EVENT_DATA2_IS_STRING(e))
                    result |= 2;
                if (DC_EVENT_RETURNS_STRING(e))
                    result |= 4;
                if (DC_EVENT_RETURNS_INT(e))
                    result |= 8;
                return result;
            }
        """,
        libraries=['deltachat'],
    )
    builder.cdef("""
        typedef int... time_t;
        void free(void *ptr);
        extern const char * dupstring_helper(const char* string);
        extern int dc_get_event_signature_types(int);
    """)
    cc = distutils.ccompiler.new_compiler(force=True)
    distutils.sysconfig.customize_compiler(cc)
    with tempfile.NamedTemporaryFile(mode='w', suffix='.h') as src_fp:
        src_fp.write('#include <deltachat/deltachat.h>')
        src_fp.flush()
        with tempfile.NamedTemporaryFile(mode='r') as dst_fp:
            cc.preprocess(source=src_fp.name,
                          output_file=dst_fp.name,
                          macros=[('PY_CFFI', '1')])
            builder.cdef(dst_fp.read())
    builder.cdef("""
        extern "Python" uintptr_t py_dc_callback(
            dc_context_t* context,
            int event,
            uintptr_t data1,
            uintptr_t data2);
    """)
    return builder


if __name__ == '__main__':
    import os.path
    pkgdir = os.path.join(os.path.dirname(__file__), '..')
    builder = ffibuilder()
    builder.compile(tmpdir=pkgdir, verbose=True)
