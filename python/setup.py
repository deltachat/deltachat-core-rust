import setuptools
import os
import re


def main():
    with open("README.rst") as f:
        long_description = f.read()
    setuptools.setup(
        name='deltachat',
        setup_requires=['setuptools_scm', 'cffi>=1.0.0'],
        use_scm_version = {
            "root": "..",
            "relative_to": __file__,
            'tag_regex': r'^(?P<prefix>py-)?(?P<version>[^\+]+)(?P<suffix>.*)?$',
            'git_describe_command': "git describe --dirty --tags --long --match py-*.*",
        },
        description='Python bindings for the Delta Chat Core library using CFFI against the Rust-implemented libdeltachat',
        long_description=long_description,
        author='holger krekel, Floris Bruynooghe, Bjoern Petersen and contributors',
        install_requires=['cffi>=1.0.0', 'six'],
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


if __name__ == "__main__":
    main()
