#!/bin/bash

PERL_VERSION=5.34.0
# PERL_SHA256=551efc818b968b05216024fb0b727ef2ad4c100f8cb6b43fab615fa78ae5be9a
curl -O https://www.cpan.org/src/5.0/perl-${PERL_VERSION}.tar.gz
# echo "${PERL_SHA256}  perl-${PERL_VERSION}.tar.gz" | sha256sum -c -
tar -xzf perl-${PERL_VERSION}.tar.gz
cd perl-${PERL_VERSION}

./Configure -de
make
make install
