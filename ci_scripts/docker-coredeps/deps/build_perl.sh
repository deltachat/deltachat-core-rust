#!/bin/bash

PERL_VERSION=5.28.0
PERL_SHA256=7e929f64d4cb0e9d1159d4a59fc89394e27fa1f7004d0836ca0d514685406ea8
curl -O https://www.cpan.org/src/5.0/perl-${PERL_VERSION}.tar.gz
echo "${PERL_SHA256}  perl-${PERL_VERSION}.tar.gz" | sha256sum -c -
tar xzf perl-${PERL_VERSION}.tar.gz
cd perl-${PERL_VERSION} 

./Configure -de
make
make install
