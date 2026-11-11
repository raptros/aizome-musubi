{ lib, rustPlatform, runCommand, makeWrapper, pkg-config, pipewire, clang, libclang, fontconfig, wayland, src, aizome, fonts }:
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
  buildInputs = [ pipewire ];
  LIBCLANG_PATH = "${libclang.lib}/lib";
  AIZOME_FONT_DIR = fonts;
  LD_LIBRARY_PATH = lib.makeLibraryPath [ fontconfig wayland pipewire ];
  # Headless Harness goldens use only pinned fonts; no running PipeWire is accessed.
  preCheck = ''export RUST_TEST_THREADS=1'';
  postInstall = ''
    install -Dm644 data/org.aizome.Musubi.desktop $out/share/applications/org.aizome.Musubi.desktop
    install -Dm644 data/org.aizome.Musubi.svg $out/share/icons/hicolor/scalable/apps/org.aizome.Musubi.svg
    wrapProgram $out/bin/aizome-musubi \
      --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath [ fontconfig wayland pipewire ]} \
      --set-default AIZOME_FONT_DIR ${fonts}
  '';
  meta = {
    description = "Musubi 結び, the PipeWire patchbay for Aizome";
    license = with lib.licenses; [ mit asl20 ];
    platforms = lib.platforms.linux;
    mainProgram = "aizome-musubi";
  };
}
