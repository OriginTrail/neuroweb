{
  description = "OriginTrail parachain node";
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    flake-utils = {
      url = "github:numtide/flake-utils";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { self
    , nixpkgs
    , flake-utils
    , naersk
    , fenix
    }:
    flake-utils.lib.eachDefaultSystem (system:
    let
      lib = nixpkgs.lib;
      pkgs = nixpkgs.legacyPackages.${system};
      fenix-pkgs = fenix.packages.${system};
      rust = with fenix-pkgs; combine [
        default.toolchain
        targets.wasm32-unknown-unknown.latest.toolchain
      ];
      # Get a naersk with the input rust version
      naerskWithRust = rust: naersk.lib."${system}".override {
        rustc = rust;
        cargo = rust;
      };
      env = with pkgs; {
        LIBCLANG_PATH = "${llvmPackages.libclang.lib}/lib";
        PROTOC = "${protobuf}/bin/protoc";
        ROCKSDB_LIB_DIR = "${rocksdb}/lib";
      };
      # Naersk using the default rust version
      buildRustProject = pkgs.makeOverridable ({ naersk ? naerskWithRust rust, ... } @ args: with pkgs; naersk.buildPackage ({
        buildInputs = [
          clang
          pkg-config
        ] ++ lib.optionals stdenv.isDarwin [
          darwin.apple_sdk.frameworks.Security
        ];
        copyLibs = true;
        copyBins = true;
        targets = [ ];
        remapPathPrefix =
          true; # remove nix store references for a smaller output package
      } // env // args));

      crateName = "origintrail-parachain";
      root = ./.;
      # This is a wrapper around naersk build
      # Remember to add Cargo.lock to git for naersk to work
      project = buildRustProject {
        inherit root;
        copySources = [ "node" "pallets" "runtime" ];
      };
      # Running tests
      testProject = project.override {
        doCheck = true;
      };
    in
    {
      packages = {
        ${crateName} = project;
        "${crateName}-test" = testProject;
      };

      defaultPackage = self.packages.${system}.${crateName};

      # `nix develop`
      devShell = pkgs.mkShell (env // {
        inputsFrom = builtins.attrValues self.packages.${system};
        nativeBuildInputs = [ rust ];
        buildInputs = [
          fenix-pkgs.rust-analyzer
          pkgs.clippy
          fenix-pkgs.default.rustfmt
        ];
      });
    });
}
