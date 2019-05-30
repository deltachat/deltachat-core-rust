FROM quay.io/pypa/manylinux1_x86_64

# Configure ld.so/ldconfig and pkg-config
RUN echo /usr/local/lib64 > /etc/ld.so.conf.d/local.conf && \
    echo /usr/local/lib >> /etc/ld.so.conf.d/local.conf
ENV PKG_CONFIG_PATH /usr/local/lib64/pkgconfig:/usr/local/lib/pkgconfig

ENV PIP_DISABLE_PIP_VERSION_CHECK 1

# Install python tools (auditwheels,tox, ...)
ADD deps/build_python.sh /builder/build_python.sh
RUN mkdir tmp1 && cd tmp1 && bash /builder/build_python.sh && cd .. && rm -r tmp1

# Install Rust nightly 
ADD deps/build_rust.sh /builder/build_rust.sh
RUN mkdir tmp1 && cd tmp1 && bash /builder/build_rust.sh && cd .. && rm -r tmp1

# Install a recent Perl, needed to install OpenSSL
ADD deps/build_perl.sh /builder/build_perl.sh
RUN mkdir tmp1 && cd tmp1 && bash /builder/build_perl.sh && cd .. && rm -r tmp1

# Install OpenSSL
ADD deps/build_openssl.sh /builder/build_openssl.sh
RUN mkdir tmp1 && cd tmp1 && bash /builder/build_openssl.sh && cd .. && rm -r tmp1

