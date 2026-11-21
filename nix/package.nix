{ lib, rustPlatform, runCommand, makeWrapper, pkg-config, pipewire, clang, libclang, fontconfig, wayland, libpulseaudio, src, aizome, fonts }:
let
  source = runCommand "musubi-source" { } ''
    mkdir -p $out/sekka/desktop
    cp -r ${src} $out/aizome-musubi
    cp -r ${aizome} $out/sekka/desktop/aizome
  '';
in rustPlatform.buildRustPackage {
  pname = "aizome-musubi";
  version = "0.1.0";
  src = source;
  sourceRoot = "musubi-source/aizome-musubi";
  cargoLock.lockFile = ../Cargo.lock;
  nativeBuildInputs = [ makeWrapper pkg-config clang ];
  buildInputs = [ pipewire libpulseaudio ];
  LIBCLANG_PATH = "${libclang.lib}/lib";
  AIZOME_FONT_DIR = fonts;
  LD_LIBRARY_PATH = lib.makeLibraryPath [ fontconfig wayland pipewire libpulseaudio ];
  # Headless Harness goldens use only pinned fonts; no running PipeWire is accessed.
  # Cargo's registry cache defaults to $HOME/.cargo; under an unsandboxed builder
  # (portable Nix) that creates /homeless-shelter and fails later derivations'
  # purity check. Keep it private. HOME itself stays unset-like (/homeless-shelter),
  # as the Harness goldens were drawn with no font cache under HOME.
  preConfigure = ''
    export CARGO_HOME=$(mktemp -d)
  '';
  postInstall = ''
    install -Dm644 data/org.aizome.Musubi.desktop $out/share/applications/org.aizome.Musubi.desktop
    install -Dm644 data/org.aizome.Musubi.svg $out/share/icons/hicolor/scalable/apps/org.aizome.Musubi.svg
    wrapProgram $out/bin/aizome-musubi \
      --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath [ fontconfig wayland pipewire libpulseaudio ]} \
      --set-default AIZOME_FONT_DIR ${fonts}
  '';
  meta = {
    description = "Musubi 結び, the PipeWire patchbay for Aizome";
    license = with lib.licenses; [ mit asl20 ];
    platforms = lib.platforms.linux;
    mainProgram = "aizome-musubi";
  };
}
