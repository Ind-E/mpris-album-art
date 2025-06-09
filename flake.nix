{
  description = "mrpis album art";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      ...
    }:
    let
      inherit (nixpkgs) lib;
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      eachSystem = lib.genAttrs systems;
      pkgsFor = eachSystem (
        system:
        import nixpkgs {
          localSystem.system = system;
          overlays = [ (import rust-overlay) ];
        }
      );
    in
    {
      checks = lib.mapAttrs (
        system: pkgs:
        let
          toolchain = pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
          rustPlatform = pkgs.makeRustPlatform {
            cargo = toolchain;
            rustc = toolchain;
          };
        in
        {
          build = rustPlatform.buildRustPackage {
            pname = "mpris_album_art";
            version = "0.1.0";
            src = self;
            cargoLock = {
              lockFile = ./Cargo.lock;
            };
          };
        }
      ) pkgsFor;

      devShells = lib.mapAttrs (system: pkgs: {
        default =
          let
            commonRustFlagsEnv = "-C link-arg=-fuse-ld=lld -C target-cpu=native --cfg tokio_unstable";
            platformRustFlagsEnv = lib.optionalString pkgs.stdenv.isLinux "-Clink-arg=-Wl,--no-rosegment";
          in
          pkgs.mkShell rec {
            inputsFrom = [ self.checks.${system}.build ];
            nativeBuildInputs = with pkgs; [
              lld
              pkg-config
              rust-bin.nightly.latest.rust-analyzer
            ];
            shellHook = ''
              export RUST_BACKTRACE="1"
              export RUSTFLAGS="''${RUSTFLAGS:-""} ${commonRustFlagsEnv} ${platformRustFlagsEnv}"
            '';

            LD_LIBRARY_PATH = "${lib.makeLibraryPath nativeBuildInputs}";

          };
      }) pkgsFor;

    };
}
