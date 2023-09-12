ARG BASEIMAGE=quay.io/pypa/manylinux2014_x86_64
#ARG BASEIMAGE=quay.io/pypa/musllinux_1_1_x86_64
#ARG BASEIMAGE=quay.io/pypa/manylinux2014_aarch64

FROM $BASEIMAGE
RUN pipx install tox
COPY install-rust.sh /scripts/
RUN /scripts/install-rust.sh
RUN yum install -y perl-IPC-Cmd
