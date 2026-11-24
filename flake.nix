{
  description = "Musubi 結び: the PipeWire patchbay for Aizome";
  inputs.sekka.url = "github:bitemyapp/sekka/aebca32e5ef6245a5ec68d2c3c8953ca957c3375";
  inputs.nixpkgs.follows = "sekka/nixpkgs";

  outputs = { self, nixpkgs, sekka }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
      aizomeFor = pkgs.runCommand "aizome-kit-source" { } ''
        a=${sekka}/desktop/aizome
        mkdir -p $out/crates $out/sukumo
        cp $a/Cargo.toml $out/
        for c in aizome-kit aizome-theme aizome-config aizome-shell-proto aizome-sys aizome-apps; do
          cp -r $a/crates/$c $out/crates/
        done
        cp $a/sukumo/Cargo.toml $out/sukumo/
        cp -r $a/sukumo/crates $out/sukumo/
      '';
      fontsFor = pkgs.callPackage "${sekka}/desktop/aizome/nix/aizome-fonts.nix" {
        archivo = pkgs.callPackage "${sekka}/desktop/aizome/nix/fonts.nix" { };
      };
      # Harness goldens are drawn with exactly these fonts, as Sekka's own tests
      # are: the pinned Aizome faces plus the CJK fallback for 結び. Without it,
      # fontconfig falls back to whatever the build machine has (a sandbox has
      # none), and the brand's glyphs differ from the goldens.
      fontsConfFor = pkgs.makeFontsConf { fontDirectories = [ fontsFor pkgs.noto-fonts-cjk-sans ]; };
      musubi = pkgs.callPackage ./nix/package.nix {
        src = pkgs.lib.cleanSource ./.;
        aizome = aizomeFor;
        fonts = fontsFor;
        fontsConf = fontsConfFor;
      };
    in {
      packages.${system} = {
        inherit musubi;
        default = musubi;
        # For running the Harness tests outside the package build (DEVELOPERS.md).
        test-fonts = fontsFor;
        test-fonts-conf = fontsConfFor;
      };
      apps.${system}.default = {
        type = "app";
        program = "${musubi}/bin/aizome-musubi";
      };
      devShells.${system}.default = pkgs.mkShell {
        packages = with pkgs; [ rustc cargo rustfmt clippy pkg-config pipewire clang libpulseaudio ];
        LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
        LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [ pkgs.fontconfig pkgs.wayland pkgs.pipewire pkgs.libpulseaudio ];
        AIZOME_FONT_DIR = fontsFor;
        FONTCONFIG_FILE = fontsConfFor;
      };
      checks.${system} = {
        inherit musubi;
        vm = import ./nix/vm-test.nix {
          inherit pkgs sekka;
          musubiModule = self.nixosModules.default;
        };
      };
      nixosModules.default = { config, lib, pkgs, ... }: {
        options.programs.musubi.enable = lib.mkEnableOption "Musubi, the PipeWire patchbay for Aizome";
        config = lib.mkIf config.programs.musubi.enable {
          environment.systemPackages = [ self.packages.${pkgs.stdenv.hostPlatform.system}.musubi ];
        };
      };
    };
}
