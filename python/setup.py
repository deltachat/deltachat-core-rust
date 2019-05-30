import setuptools
import os
import re


def main():
    long_description, version = read_meta()
    setuptools.setup(
        name='deltachat',
        version=version,
        description='Python bindings for deltachat-core using CFFI',
        long_description=long_description,
        author='holger krekel, Floris Bruynooghe, Bjoern Petersen and contributors',
        setup_requires=['cffi>=1.0.0'],
        install_requires=['cffi>=1.0.0', 'requests', 'attrs', 'six'],
        packages=setuptools.find_packages('src'),
        package_dir={'': 'src'},
        cffi_modules=['src/deltachat/_build.py:ffibuilder'],
        classifiers=[
            'Development Status :: 4 - Beta',
            'Intended Audience :: Developers',
            'License :: OSI Approved :: Mozilla Public License 2.0 (MPL 2.0)',
            'Programming Language :: Python :: 3',
            'Topic :: Communications :: Email',
            'Topic :: Software Development :: Libraries',
        ],
    )


def read_meta():
    with open(os.path.join("src", "deltachat", "__init__.py")) as f:
        for line in f:
            m = re.match('__version__ = "(\S*).*"', line)
            if m:
                version, = m.groups()
                break

    with open("README.rst") as f:
        long_desc = f.read()
    return long_desc, version


if __name__ == "__main__":
    main()
