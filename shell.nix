{ pkgs ? import <nixpkgs> {} }:
  pkgs.mkShell rec {
    buildInputs = with pkgs; [
      gcc
      rustup
      glib
      pkg-config
      wayland
      gtk3
      libxkbcommon
      xorg.libX11
      xorg.libXcursor
      xorg.libXrandr
      xorg.libXi
      libglvnd
    ];
    CARGO_HOME = toString ./.cargo;
    RUSTUP_HOME = toString ./.rustup;
}
