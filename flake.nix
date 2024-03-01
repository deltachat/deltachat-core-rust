{
  description = "Delta Chat core";
  inputs = {
    fenix.url = "github:nix-community/fenix";
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    nix-filter.url = "github:numtide/nix-filter";
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };
  outputs = { self, nixpkgs, flake-utils, nix-filter, naersk, fenix }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        fenixPkgs = fenix.packages.${system};
        naersk' = pkgs.callPackage naersk { };
        manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
        cargoLock = {
          lockFile = ./Cargo.lock;
          outputHashes = {
            "email-0.0.20" = "sha256-rV4Uzqt2Qdrfi5Ti1r+Si1c2iW1kKyWLwOgLkQ5JGGw=";
            "encoded-words-0.2.0" = "sha256-KK9st0hLFh4dsrnLd6D8lC6pRFFs8W+WpZSGMGJcosk=";
            "lettre-0.9.2" = "sha256-+hU1cFacyyeC9UGVBpS14BWlJjHy90i/3ynMkKAzclk=";
          };
        };
        mkRustPackage = packageName:
          naersk'.buildPackage {
            pname = packageName;
            cargoBuildOptions = x: x ++ [ "--package" packageName ];
            version = manifest.version;
            src = pkgs.lib.cleanSource ./.;
            nativeBuildInputs = [
              pkgs.perl # Needed to build vendored OpenSSL.
            ];
            auditable = false; # Avoid cargo-auditable failures.
            doCheck = false; # Disable test as it requires network access.
          };
        pkgsWin64 = pkgs.pkgsCross.mingwW64;
        mkWin64RustPackage = packageName:
          let
            rustTarget = "x86_64-pc-windows-gnu";
          in
          let
            toolchainWin = fenixPkgs.combine [
              fenixPkgs.stable.rustc
              fenixPkgs.stable.cargo
              fenixPkgs.targets.${rustTarget}.stable.rust-std
            ];
            naerskWin = pkgs.callPackage naersk {
              cargo = toolchainWin;
              rustc = toolchainWin;
            };
          in
          naerskWin.buildPackage rec {
            pname = packageName;
            cargoBuildOptions = x: x ++ [ "--package" packageName ];
            version = manifest.version;
            strictDeps = true;
            src = pkgs.lib.cleanSource ./.;
            nativeBuildInputs = [
              pkgs.perl # Needed to build vendored OpenSSL.
            ];
            depsBuildBuild = [
              pkgsWin64.stdenv.cc
              pkgsWin64.windows.pthreads
            ];
            auditable = false; # Avoid cargo-auditable failures.
            doCheck = false; # Disable test as it requires network access.

            CARGO_BUILD_TARGET = rustTarget;
            TARGET_CC = "${pkgsWin64.stdenv.cc}/bin/${pkgsWin64.stdenv.cc.targetPrefix}cc";
            CARGO_BUILD_RUSTFLAGS = [
              "-C"
              "linker=${TARGET_CC}"
            ];

            CC = "${pkgsWin64.stdenv.cc}/bin/${pkgsWin64.stdenv.cc.targetPrefix}cc";
            LD = "${pkgsWin64.stdenv.cc}/bin/${pkgsWin64.stdenv.cc.targetPrefix}cc";
          };

        pkgsWin32 = pkgs.pkgsCross.mingw32;
        mkWin32RustPackage = packageName:
          let
            rustTarget = "i686-pc-windows-gnu";
          in
          let
            toolchainWin = fenixPkgs.combine [
              fenixPkgs.stable.rustc
              fenixPkgs.stable.cargo
              fenixPkgs.targets.${rustTarget}.stable.rust-std
            ];
            naerskWin = pkgs.callPackage naersk {
              cargo = toolchainWin;
              rustc = toolchainWin;
            };

            # Get rid of MCF Gthread library.
            # See <https://github.com/NixOS/nixpkgs/issues/156343>
            # and <https://discourse.nixos.org/t/statically-linked-mingw-binaries/38395>
            # for details.
            #
            # Use DWARF-2 instead of SJLJ for exception handling.
            winCC = pkgsWin32.buildPackages.wrapCC (
              (pkgsWin32.buildPackages.gcc-unwrapped.override
                ({
                  threadsCross = {
                    model = "win32";
                    package = null;
                  };
                })).overrideAttrs (oldAttr: rec{
                configureFlags = oldAttr.configureFlags ++ [
                  "--disable-sjlj-exceptions --with-dwarf2"
                ];
              })
            );
            winStdenv = pkgsWin32.buildPackages.overrideCC pkgsWin32.stdenv winCC;
          in
          naerskWin.buildPackage rec {
            pname = packageName;
            cargoBuildOptions = x: x ++ [ "--package" packageName ];
            version = manifest.version;
            strictDeps = true;
            src = pkgs.lib.cleanSource ./.;
            nativeBuildInputs = [
              pkgs.perl # Needed to build vendored OpenSSL.
            ];
            depsBuildBuild = [
              winCC
              pkgsWin32.windows.pthreads
            ];
            auditable = false; # Avoid cargo-auditable failures.
            doCheck = false; # Disable test as it requires network access.

            CARGO_BUILD_TARGET = rustTarget;
            TARGET_CC = "${winCC}/bin/${winCC.targetPrefix}cc";
            CARGO_BUILD_RUSTFLAGS = [
              "-C"
              "linker=${TARGET_CC}"
            ];

            CC = "${winCC}/bin/${winCC.targetPrefix}cc";
            LD = "${winCC}/bin/${winCC.targetPrefix}cc";
          };

        mkCrossRustPackage = rustTarget: crossTarget: packageName:
          let
            pkgsCross = import nixpkgs {
              system = system;
              crossSystem.config = crossTarget;
            };
          in
          let
            toolchain = fenixPkgs.combine [
              fenixPkgs.stable.rustc
              fenixPkgs.stable.cargo
              fenixPkgs.targets.${rustTarget}.stable.rust-std
            ];
            naersk-lib = pkgs.callPackage naersk {
              cargo = toolchain;
              rustc = toolchain;
            };
          in
          naersk-lib.buildPackage rec {
            pname = packageName;
            cargoBuildOptions = x: x ++ [ "--package" packageName ];
            version = manifest.version;
            strictDeps = true;
            src = pkgs.lib.cleanSource ./.;
            nativeBuildInputs = [
              pkgs.perl # Needed to build vendored OpenSSL.
            ];
            auditable = false; # Avoid cargo-auditable failures.
            doCheck = false; # Disable test as it requires network access.

            CARGO_BUILD_TARGET = rustTarget;
            TARGET_CC = "${pkgsCross.stdenv.cc}/bin/${pkgsCross.stdenv.cc.targetPrefix}cc";
            CARGO_BUILD_RUSTFLAGS = [
              "-C"
              "linker=${TARGET_CC}"
            ];

            CC = "${pkgsCross.stdenv.cc}/bin/${pkgsCross.stdenv.cc.targetPrefix}cc";
            LD = "${pkgsCross.stdenv.cc}/bin/${pkgsCross.stdenv.cc.targetPrefix}cc";
          };

        mk-aarch64-RustPackage = mkCrossRustPackage "aarch64-unknown-linux-musl" "aarch64-unknown-linux-musl";
        mk-i686-RustPackage = mkCrossRustPackage "i686-unknown-linux-musl" "i686-unknown-linux-musl";
        mk-x86_64-RustPackage = mkCrossRustPackage "x86_64-unknown-linux-musl" "x86_64-unknown-linux-musl";
        mk-armv7l-RustPackage = mkCrossRustPackage "armv7-unknown-linux-musleabihf" "armv7l-unknown-linux-musleabihf";
        mk-armv6l-RustPackage = mkCrossRustPackage "arm-unknown-linux-musleabihf" "armv6l-unknown-linux-musleabihf";
      in
      {
        formatter = pkgs.nixpkgs-fmt;

        packages = rec {
          # Run with `nix run .#deltachat-repl foo.db`.
          deltachat-repl = mkRustPackage "deltachat-repl";
          deltachat-rpc-server = mkRustPackage "deltachat-rpc-server";

          deltachat-repl-win64 = mkWin64RustPackage "deltachat-repl";
          deltachat-rpc-server-win64 = mkWin64RustPackage "deltachat-rpc-server";

          deltachat-repl-win32 = mkWin32RustPackage "deltachat-repl";
          deltachat-rpc-server-win32 = mkWin32RustPackage "deltachat-rpc-server";

          deltachat-repl-aarch64-linux = mk-aarch64-RustPackage "deltachat-repl";
          deltachat-rpc-server-aarch64-linux = mk-aarch64-RustPackage "deltachat-rpc-server";

          deltachat-repl-i686-linux = mk-i686-RustPackage "deltachat-repl";
          deltachat-rpc-server-i686-linux = mk-i686-RustPackage "deltachat-rpc-server";

          deltachat-repl-x86_64-linux = mk-x86_64-RustPackage "deltachat-repl";
          deltachat-rpc-server-x86_64-linux = mk-x86_64-RustPackage "deltachat-rpc-server";

          deltachat-repl-armv7l-linux = mk-armv7l-RustPackage "deltachat-repl";
          deltachat-rpc-server-armv7l-linux = mk-armv7l-RustPackage "deltachat-rpc-server";

          deltachat-repl-armv6l-linux = mk-armv6l-RustPackage "deltachat-repl";
          deltachat-rpc-server-armv6l-linux = mk-armv6l-RustPackage "deltachat-rpc-server";

          # Run `nix build .#docs` to get C docs generated in `./result/`.
          docs =
            pkgs.stdenv.mkDerivation {
              pname = "docs";
              version = manifest.version;
              src = pkgs.lib.cleanSource ./.;
              nativeBuildInputs = [ pkgs.doxygen ];
              buildPhase = ''scripts/run-doxygen.sh'';
              installPhase = ''mkdir -p $out; cp -av deltachat-ffi/html deltachat-ffi/xml $out'';
            };

          libdeltachat =
            pkgs.stdenv.mkDerivation rec {
              pname = "libdeltachat";
              version = manifest.version;
              src = nix-filter.lib {
                root = ./.;

                # Include only necessary files
                # to avoid rebuilds e.g. when README.md or flake.nix changes.
                include = [
                  ./benches
                  ./assets
                  ./Cargo.lock
                  ./Cargo.toml
                  ./CMakeLists.txt
                  ./CONTRIBUTING.md
                  ./deltachat_derive
                  ./deltachat-ffi
                  ./deltachat-jsonrpc
                  ./deltachat-ratelimit
                  ./deltachat-repl
                  ./deltachat-rpc-client
                  ./deltachat-time
                  ./deltachat-rpc-server
                  ./format-flowed
                  ./release-date.in
                  ./src
                ];
                exclude = [
                  (nix-filter.lib.matchExt "nix")
                  "flake.lock"
                ];
              };
              cargoDeps = pkgs.rustPlatform.importCargoLock cargoLock;

              nativeBuildInputs = [
                pkgs.perl # Needed to build vendored OpenSSL.
                pkgs.cmake
                pkgs.rustPlatform.cargoSetupHook
                pkgs.cargo
              ];

              postInstall = ''
                substituteInPlace $out/include/deltachat.h \
                  --replace __FILE__ '"${placeholder "out"}/include/deltachat.h"'
              '';
            };

          deltachat-rpc-client =
            pkgs.python3Packages.buildPythonPackage rec {
              pname = "deltachat-rpc-client";
              version = manifest.version;
              src = pkgs.lib.cleanSource ./deltachat-rpc-client;
              format = "pyproject";
              propagatedBuildInputs = [
                pkgs.python3Packages.setuptools
                pkgs.python3Packages.setuptools_scm
              ];
            };

          deltachat-python =
            pkgs.python3Packages.buildPythonPackage rec {
              pname = "deltachat-python";
              version = manifest.version;
              src = pkgs.lib.cleanSource ./python;
              format = "pyproject";
              buildInputs = [
                libdeltachat
              ];
              nativeBuildInputs = [
                pkgs.pkg-config
              ];
              propagatedBuildInputs = [
                pkgs.python3Packages.setuptools
                pkgs.python3Packages.setuptools_scm
                pkgs.python3Packages.pkgconfig
                pkgs.python3Packages.cffi
                pkgs.python3Packages.imap-tools
                pkgs.python3Packages.pluggy
                pkgs.python3Packages.requests
              ];
            };
          python-docs =
            pkgs.stdenv.mkDerivation {
              pname = "docs";
              version = manifest.version;
              src = pkgs.lib.cleanSource ./.;
              buildInputs = [
                deltachat-python
                deltachat-rpc-client
                pkgs.python3Packages.breathe
                pkgs.python3Packages.sphinx_rtd_theme
              ];
              nativeBuildInputs = [ pkgs.sphinx ];
              buildPhase = ''sphinx-build -b html -a python/doc/ dist/html'';
              installPhase = ''mkdir -p $out; cp -av dist/html $out'';
            };
        };
      }
    );
}
