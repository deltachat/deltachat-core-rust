FROM quay.io/pypa/manylinux2014_aarch64
RUN pipx install tox

# Install Rust
ADD deps/build_rust.sh /builder/build_rust.sh
RUN mkdir tmp1 && cd tmp1 && bash /builder/build_rust.sh && cd .. && rm -r tmp1
