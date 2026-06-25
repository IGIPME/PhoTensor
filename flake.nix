{
    description = "Photon neural network framework";

    inputs = {
        nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

        rust-overlay = {
            url = "github:oxalica/rust-overlay";
            inputs.nixpkgs.follows = "nixpkgs";
        };

        flake-utils.url = "github:numtide/flake-utils";
    };

    outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
        flake-utils.lib.eachDefaultSystem (system:
            let
                pkgs = import nixpkgs {
                    inherit system;
                    config = {
                        allowUnfree = true;

                        substituters = [
                            "https://mirrors.tuna.tsinghua.edu.cn/nix-channels/store"
                            "https://mirrors.ustc.edu.cn/nix-channels/store"
                            "https://mirror.sjtu.edu.cn/nix-channels/store"
                        ];
                    };
                    overlays = [ rust-overlay.overlays.default ];
                };

                rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

                nodejs = pkgs.nodejs_24;
            in
            {
                devShells.default = pkgs.mkShell {
                    buildInputs = with pkgs; [
                        # Rust Toolchain
                        rustToolchain

                        # Maturin
                        maturin

                        # Python
                        python3
                        python3Packages.twine

                        # Node.js runtime
                        nodejs

                        # pnpm package manager
                        pnpm

                        # system tools
                        lld
                        clang
                    ];

                    shellHook = ''
                        unset _PYTHON_HOST_PLATFORM

                        # Print current environment information
                        echo "Rust + Python + pnpm dev environment"
                        echo "Rust version: $(rustc --version)"
                        echo "Python version: $(python --version)"
                        echo "Node.js version: $(node --version)"
                        echo "pnpm version: $(pnpm --version)"
                    '';
                };
            }
        );
}