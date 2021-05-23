#!/bin/bash

set -e -x

OPENSSL_VERSION=1.1.1a
OPENSSL_SHA256=fc20130f8b7cbd2fb918b2f14e2f429e109c31ddd0fb38fc5d71d9ffed3f9f41

curl -O https://www.openssl.org/source/openssl-${OPENSSL_VERSION}.tar.gz
echo "${OPENSSL_SHA256}  openssl-${OPENSSL_VERSION}.tar.gz" | sha256sum -c -
tar xzf openssl-${OPENSSL_VERSION}.tar.gz
cd openssl-${OPENSSL_VERSION} 
./config shared no-ssl2 no-ssl3 -fPIC --prefix=/usr/local

sed -i "s/^SHLIB_MAJOR=.*/SHLIB_MAJOR=200/" Makefile && \
sed -i "s/^SHLIB_MINOR=.*/SHLIB_MINOR=0.0/" Makefile && \
sed -i "s/^SHLIB_VERSION_NUMBER=.*/SHLIB_VERSION_NUMBER=200.0.0/" Makefile

make depend
make 
make install_sw install_ssldirs
ldconfig -v | grep ssl 
