import setuptools
import os
import re


def main():
    with open("README.rst") as f:
        long_description = f.read()
    setuptools.setup(
        name='deltachat',
        description='Python bindings for the Delta Chat Core library using CFFI against the Rust-implemented libdeltachat',
        long_description=long_description,
        author='holger krekel, Floris Bruynooghe, Bjoern Petersen and contributors',
        install_requires=['cffi>=1.0.0', 'pluggy', 'imapclient', 'requests'],
        setup_requires=['setuptools_scm'], # required for compatibility with `python3 setup.py sdist`
        packages=setuptools.find_packages('src'),
        package_dir={'': 'src'},
        cffi_modules=['src/deltachat/_build.py:ffibuilder'],
        entry_points = {
            'pytest11': [
                'deltachat.testplugin = deltachat.testplugin',
            ],
        },
        classifiers=[
            'Development Status :: 4 - Beta',
            'Intended Audience :: Developers',
            'License :: OSI Approved :: Mozilla Public License 2.0 (MPL 2.0)',
            'Programming Language :: Python :: 3',
            'Topic :: Communications :: Email',
            'Topic :: Software Development :: Libraries',
        ],
    )


if __name__ == "__main__":
    main()
