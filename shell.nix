{ pkgs ? import <nixpkgs> {} }:

let
  fenix = import (fetchTarball "https://github.com/nix-community/fenix/archive/main.tar.gz") { };

  rustToolchain = fenix.combine [
    fenix.stable.cargo
    fenix.stable.rustc
    fenix.stable.rust-analyzer
    fenix.targets.x86_64-pc-windows-gnu.stable.rust-std
  ];

  crossPkgs = pkgs.pkgsCross.mingwW64;
  mingwCC = crossPkgs.stdenv.cc;
  mingwLib = crossPkgs.windows.pthreads;
in
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    rustToolchain
    pkg-config
    tree
    mingwCC
    rustup
  ];

  buildInputs = with pkgs; [
    libx11
    libxcursor
    libxrandr
    libxi
    libxkbcommon
    wayland
    glib
    vulkan-loader
    libGL
  ];

  shellHook = ''
    export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${pkgs.lib.makeLibraryPath (with pkgs; [
      vulkan-loader
      libGL
      libx11
      libxcursor
      libxrandr
      libxi
      libxkbcommon
      wayland
      glib
    ])}"

    export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER="${mingwCC}/bin/x86_64-w64-mingw32-gcc"
    export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS="-L ${mingwLib}/lib"
  '';
}
