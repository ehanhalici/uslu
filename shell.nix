{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    pkg-config
    rustc
    rust-analyzer
    rustfmt
    cargo
    zenity
    tree
  ];

  buildInputs = with pkgs; [
    # X11 & Wayland Kütüphaneleri
    libx11
    libxcursor
    libxrandr
    libxi
    libxkbcommon
    wayland
    glib
    # Grafik Sürücü Kütüphaneleri (wgpu için şart)
    vulkan-loader
    libGL
  ];

  # wgpu ve iced'in .so kütüphanelerini bulabilmesi için LD_LIBRARY_PATH şarttır:
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
  '';
}
