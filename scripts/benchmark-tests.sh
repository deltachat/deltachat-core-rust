#!/bin/sh
# Benchmark the tests with `hyperfine`.

set -e

# `--tests` selects unit tests but not the documentation tests.
# `--skip` arguments skip online tests.
hyperfine --warmup 1 --min-runs 20 "cargo test --tests --release -- --skip imex::transfer::tests::test_drop_provider --skip imex::transfer::tests::test_send_receive --skip oauth2::tests::test_oauth_from_mx --skip oauth2::tests::test_get_oauth2_token --skip oauth2::tests::test_get_oauth2_addr --skip configure::tests::test_no_panic_on_bad_credentials --skip qr_code_generator::tests::test_generate_backup_qr"
