from setuptools import setup

if __name__ == "__main__":
    setup(cffi_modules=["src/deltachat/_build.py:ffibuilder"])
