# External Aizome applications: precedent for Musubi

Research snapshot: 11 November 2026. Source references below use immutable commit identifiers. To keep the many citations readable, a reference of the form **U:path:Lx–Ly** means `https://github.com/bitemyapp/uta/blob/bb4523b798640041794a3f48415c34e50e5f7c6e/path#Lx-Ly`; **T** means `https://github.com/bitemyapp/tsutsumi/blob/4c5274179d17d9fea52b2f7c752e11a523fbc6c0/path#Lx-Ly`; **E** means `https://github.com/bitemyapp/sekka-edu/blob/8ad1e4ff41f0a141eec3624599665b19173d8c81/path#Lx-Ly`; **D** means `https://github.com/bitemyapp/aizome-design/blob/f36cb67b66e8371740b2a183e43027b90ad521cc/path#Lx-Ly`; **S** means `https://github.com/bitemyapp/sekka/blob/aebca32e5ef6245a5ec68d2c3c8953ca957c3375/path#Lx-Ly`. Thus every `U:Cargo.toml:L1-L8` below is an exact repo URL, path, SHA, and line range, rather than a branch-relative citation. The directory trees are inventories of those four checked-out commits; README layout tables and package manifests independently anchor the substantive entries. Absence claims refer to those snapshots, not a promise about future commits. No builds, tests, or Nix/Cargo commands were executed during this investigation.

## 1. The three independent repositories

### Uta: single application with an unusually complex service

Depth-two source layout (directories collapsed at that depth):

```text
uta/
  .github/workflows/ci.yml
  Cargo.toml  Cargo.lock  flake.nix  flake.lock  README.md
  LICENSE-MIT  LICENSE-APACHE  .gitignore
  nix/package.nix
  data/org.aizome.Uta.desktop  data/org.aizome.Uta.svg  data/genicon.py
  src/{app,artwork,benchmark,instance,lib,main,model,mpris,native_service,
       options,player,process,service,store,ui}.rs
  tests/{app,credentials,mpris,native,network,player,solver}.rs
  tests/snapshots/  service/uta_service.py  service/tests/
  scripts/{benchmark-youtube,benchmark-youtubejs.mjs,benchmark-ytdlp-versions,
           benchmark-distube.mjs,install-local.sh,smoke-service}
  docs/{architecture,performance,premium-latency-results,premium-playback,
        rustypipe-integration,youtube-integration-research}.md  docs/research/
  examples/{frame_budget,start_latency,youtube_latency}.rs
  vendor/{README.md,rustypipe/,rustypipe-visionos.patch}
```

The root is **one package**, not a Cargo `[workspace]`: package `aizome-uta` version `0.1.0`, Rust 2024, minimum Rust 1.95, `MIT OR Apache-2.0`. Its actual dependency is a *relative Cargo path*, not a Git dependency: `aizome-kit = { path = "../aizome/crates/aizome-kit" }`; there is no direct Cargo Sidestep dependency. Default `rustypipe` enables vendored `rustypipe` and Tokio; Python-only uses `--no-default-features`. Quoted manifest excerpt [U:Cargo.toml:L1-L28]:

```toml
[package]
name = "aizome-uta"
version = "0.1.0"
edition = "2024"
rust-version = "1.95"
license = "MIT OR Apache-2.0"
[features]
default = ["rustypipe"]
rustypipe = ["dep:rustypipe", "dep:tokio"]
[dependencies]
aizome-kit = { path = "../aizome/crates/aizome-kit" }
rustypipe = { version = "=0.11.4", path = "vendor/rustypipe", optional = true, default-features = false, features = ["rustls-tls-webpki-roots"] }
```

Its standalone flake **still points at the former Aizome repository**, revision `729cf1c140b349dc6f8a9d7ba1772c4b9bb4fe18`, and overrides its Sidestep input with `github:bitemyapp/sidestep/16a1e376c0bd85d52ef020e114bbd1707fb9b14b`; Nixpkgs follows Aizome. These are Nix input revisions, not Cargo git revisions. `flake.lock` resolves them identically [U:flake.nix:L1-L10; U:flake.lock:L3-L20; U:flake.lock:L48-L63]. The essential flake excerpt [U:flake.nix:L4-L8,L16-L44]:

```nix
inputs = {
  nixpkgs.follows = "aizome/nixpkgs";
  aizome.url = "github:bitemyapp/aizome/729cf1c140b349dc6f8a9d7ba1772c4b9bb4fe18";
  aizome.inputs.sidestep.url = "github:bitemyapp/sidestep/16a1e376c0bd85d52ef020e114bbd1707fb9b14b";
};
packages = each (pkgs: rec {
  uta = pkgs.callPackage ./nix/package.nix {
    src = pkgs.lib.cleanSource ./.;
    inherit aizome;
    sidestep = aizome.inputs.sidestep;
    python = pythonFor pkgs;
    fonts = aizome.packages.${pkgs.stdenv.hostPlatform.system}.fonts;
  };
  default = uta;
  uta-rustypipe = uta;
  uta-python = uta.override { enableRustyPipe = false; };
});
```

Outputs cover x86_64-linux and aarch64-linux; `apps.default` launches `aizome-uta`; devShell supplies Rust tools, mpv, Deno, D-Bus and Python helper, with font and library settings; `checks` packages both default and Python-only variants; `nixosModules.default` defines `programs.uta.enable` and adds the package to `environment.systemPackages`. No Home Manager module appears in its outputs [U:flake.nix:L10-L45]. The recipe copies Uta, Aizome and Sidestep into neighboring paths and links `aizome/sidestep`; it uses `rustPlatform.buildRustPackage`, Uta's own `Cargo.lock`, its RustyPipe feature switch and Python tests, then installs and wraps the desktop program and runtime dependencies [U:nix/package.nix:L1-L49]. With RustyPipe linked, the *combined* Nix package is GPL-3.0-only and includes vendored-component notices; Uta's own source remains dual MIT/Apache [U:nix/package.nix:L28-L47; U:README.md:L238-L243].

The runtime ID, launcher and scalable icon all use `org.aizome.Uta`; executable `aizome-uta`, desktop name **Uta**, generic name **YouTube Music**, `%U`, `AudioVideo;Audio;Music;Player;`, `StartupWMClass=org.aizome.Uta`, and no D-Bus activation [U:data/org.aizome.Uta.desktop:L1-L12; U:src/app.rs:L16-L19,L683-L685]. Icon SVG is at `data/org.aizome.Uta.svg` and generated via `data/genicon.py`, not edited by hand [U:README.md:L230-L236; U:nix/package.nix:L33-L35]. The README begins with name/meaning and a Night screenshot, puts runnable functionality and installation before account/privacy instructions, and candidly separates working features from gaps, benchmark details and licensing [U:README.md:L1-L45,L87-L103,L172-L243]. There is **no** root AGENTS.md, DEVELOPERS.md, CLAUDE.md, rustfmt.toml or clippy config in this snapshot; README and architecture docs supply development guidance. Its sole CI workflow builds two variants on push to main and PR, with read-only contents, pinned checkout/install-nix actions, and verifies `--version`'s RustyPipe flag [U:.github/workflows/ci.yml:L1-L28]. Native `aizome_kit::testing::{Harness, assert_golden}` tests record Day/Night PNGs under `tests/snapshots/`; other Rust integration tests cover playback, network, credentials, MPRIS and solver, while Python helper tests run in Nix `postCheck`; optional network smoke tests are explicitly separate. No VM test is declared by this standalone flake [U:tests/app.rs:L1-L20; U:README.md:L212-L231; U:nix/package.nix:L25-L27; U:flake.nix:L28-L45]. Version is `0.1.0` in both Cargo and Nix, without a visible release job [U:Cargo.toml:L1-L8; U:nix/package.nix:L12-L17; U:.github/workflows/ci.yml:L1-L28].

### Tsutsumi: the closest *single native-app* package pattern

```text
tsutsumi/
  .github/workflows/ci.yml
  Cargo.toml  Cargo.lock  flake.nix  flake.lock  README.md
  LICENSE-MIT  LICENSE-APACHE  .gitignore
  nix/package.nix
  data/org.aizome.Tsutsumi.{desktop,svg,xml}
  src/{app,bundle,catalog,instance,lib,main,secure_fs}.rs
  tests/{app,backup}.rs  tests/snapshots/
```

This too is one package, no workspace members: `aizome-tsutsumi`, `0.1.0`, Rust 2024, MSRV 1.95, dual MIT/Apache, and **the same sibling path** to the old Aizome kit as Uta. It has no direct Cargo dependency on Sidestep, no Cargo feature switch and no vendored toolkit; backup-specific libraries (`age`, `globset`, `zeroize`) are ordinary registry dependencies [T:Cargo.toml:L1-L19]. Relevant exact Cargo excerpt [T:Cargo.toml:L1-L13]:

```toml
[package]
name = "aizome-tsutsumi"
version = "0.1.0"
edition = "2024"
rust-version = "1.95"
license = "MIT OR Apache-2.0"
[dependencies]
aizome-kit = { path = "../aizome/crates/aizome-kit" }
age = { version = "0.11", default-features = false }
```

Its flake likewise pins former Aizome `729cf1c140b349dc6f8a9d7ba1772c4b9bb4fe18` and Sidestep `16a1e376c0bd85d52ef020e114bbd1707fb9b14b`, Nixpkgs follows Aizome, and packages build for x86_64 and aarch64. It exposes `packages.tsutsumi`/`default`, `apps.default`, `devShells.default`, build-derived `checks.tsutsumi`, and `nixosModules.default` with `programs.tsutsumi.enable`; no Home Manager module [T:flake.nix:L1-L36]. Exact excerpt [T:flake.nix:L3-L20,L22-L34]:

```nix
inputs = {
  nixpkgs.follows = "aizome/nixpkgs";
  aizome.url = "github:bitemyapp/aizome/729cf1c140b349dc6f8a9d7ba1772c4b9bb4fe18";
  aizome.inputs.sidestep.url = "github:bitemyapp/sidestep/16a1e376c0bd85d52ef020e114bbd1707fb9b14b";
};
packages = each (pkgs: rec {
  tsutsumi = pkgs.callPackage ./nix/package.nix {
    src = pkgs.lib.cleanSource ./.;
    inherit aizome;
    sidestep = aizome.inputs.sidestep;
    fonts = aizome.packages.${pkgs.stdenv.hostPlatform.system}.fonts;
  };
  default = tsutsumi;
});
```

Its Nix recipe constructs a sibling source layout, `cargoLock.lockFile = ../Cargo.lock`, sets fonts and native runtime library paths, installs desktop SVG and custom MIME XML, and wraps its executable [T:nix/package.nix:L1-L35]. Desktop ID is `org.aizome.Tsutsumi` both in `AppInfo` and `.desktop`; app name **Tsutsumi**, generic **Backup & Restore**, `%f`, `Utility;Archiving;` and `MimeType=application/x-tsutsumi`; icon is `data/org.aizome.Tsutsumi.svg` installed into hicolor scalable apps [T:src/app.rs:L381-L383; T:data/org.aizome.Tsutsumi.desktop:L1-L13; T:nix/package.nix:L21-L27]. README opens with meaning and Night screenshot; user-centered steps, privacy/security boundaries and honest backup portability limitations dominate, followed by development and exact fixture coverage. It explicitly says the app runs without root and does not install apps or system settings [T:README.md:L1-L32,L34-L78,L107-L177]. Root AGENTS/DEVELOPERS/CLAUDE and rustfmt/clippy config are absent in this snapshot. CI is one read-only GitHub workflow on main/PR: pinned Nix setup, `nix build`, version invocation; no release workflow [T:.github/workflows/ci.yml:L1-L19]. `tests/app.rs` drives the kit's headless Harness through actual UI backup/import, and captures both-theme golden PNGs under `tests/snapshots/`; `tests/backup.rs` covers backend fixture cases. There is no standalone VM check [T:tests/app.rs:L1-L7,L22-L76,L132-L189; T:README.md:L159-L166; T:flake.nix:L22-L35]. Its Cargo and Nix versions both read `0.1.0` [T:Cargo.toml:L1-L8; T:nix/package.nix:L11-L17].

### Sekka Edu: modern Sekka-local kit precedent, but a multi-app workspace

```text
sekka-edu/
  AGENTS.md  DEVELOPERS.md  README.md  rustfmt.toml
  Cargo.toml  Cargo.lock  flake.nix  flake.lock
  LICENSE-MIT  LICENSE-APACHE  .gitignore
  apps/anzan/{Cargo.toml,src/,tests/,data/}
  apps/kids/{Cargo.toml,src/,tests/,data/}
  crates/sekka-kid/{Cargo.toml,src/,tests/}
  crates/sekka-kid-kit/{Cargo.toml,src/}
  docs/ideas.md  nix/package.nix  scripts/wx
```

Its root `[workspace]` uses resolver 3, four members and `[workspace.package]` version `0.1.0`, edition 2024, Rust 1.95, dual MIT/Apache. Here the **current Sekka tree** is the sibling, not former Aizome; direct Cargo dependency is by path, no Git SHA or direct Sidestep dependency. `sekka-kid-kit`, Anzan and Kids inherit it with `aizome-kit.workspace = true`; `sekka-kid` is intentionally UI-free [E:Cargo.toml:L1-L21; E:apps/anzan/Cargo.toml:L14-L25; E:crates/sekka-kid-kit/Cargo.toml:L1-L13; E:DEVELOPERS.md:L3-L12]. Exact excerpt [E:Cargo.toml:L1-L18]:

```toml
[workspace]
resolver = "3"
members = ["crates/sekka-kid", "crates/sekka-kid-kit", "apps/anzan", "apps/kids"]
[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.95"
license = "MIT OR Apache-2.0"
[workspace.dependencies]
aizome-kit = { path = "../sekka/desktop/aizome/crates/aizome-kit" }
sekka-kid = { path = "crates/sekka-kid" }
sekka-kid-kit = { path = "crates/sekka-kid-kit" }
```

Its flake pins **Sekka** `cd82f3c1ab8c06dd1fdc70ec6f49c37ba11c089c`, with `nixpkgs.follows = "sekka/nixpkgs"`, rather than an Aizome/Sidestep input. `aizomeFor` copies only Aizome workspace manifest, kit, theme, config, shell-proto, and Sukumo's manifest/crates to the path needed by Cargo; `fontsFor` imports Sekka's font derivations. Excerpt [E:flake.nix:L4-L38]:

```nix
inputs.sekka.url = "github:bitemyapp/sekka/cd82f3c1ab8c06dd1fdc70ec6f49c37ba11c089c";
inputs.nixpkgs.follows = "sekka/nixpkgs";
aizomeFor = pkgs: pkgs.runCommand "aizome-kit-source" { } ''
  a=${sekka}/desktop/aizome
  mkdir -p $out/crates $out/sukumo
  cp $a/Cargo.toml $out/
  for c in aizome-kit aizome-theme aizome-config aizome-shell-proto; do
    cp -r $a/crates/$c $out/crates/
  done
  cp $a/sukumo/Cargo.toml $out/sukumo/
  cp -r $a/sukumo/crates $out/sukumo/
'';
```

Only x86_64-linux is declared. `packages.sekka-edu` and `default` build both apps; `apps.default` selects Anzan, plus named `apps.anzan` and `apps.kids`; `checks.sekka-edu` is the build; `nixosModules.default` enables the pair via `programs.sekka-edu.enable`. **There is no devShell output and no Home Manager module** [E:flake.nix:L17-L18,L39-L86]. Nix copies `src` and the limited kit tree under `sekka/desktop/aizome`, locks with `../Cargo.lock`, installs both `.desktop`/SVG pairs, and supplies PipeWire/espeak-ng to Anzan. The `postInstall` reads these files from `apps/anzan/data` and `apps/kids/data` [E:nix/package.nix:L1-L76]. Their IDs are `computer.sekka.Anzan` and `computer.sekka.Kids` (not `org.aizome.*`), with binaries `sekka-anzan`/`sekka-kids`; generic names are **Mental Abacus**/**Kids' Profiles**, icons and desktop files share those IDs [E:apps/anzan/data/computer.sekka.Anzan.desktop:L1-L12; E:apps/kids/data/computer.sekka.Kids.desktop:L1-L12; E:apps/anzan/src/app.rs:L29-L31; E:apps/kids/src/app.rs:L16-L19]. Both source licenses are included. There is no `.github/workflows` in this snapshot, and the README explicitly says its Nix package has not been built and it is absent from Sekka's catalog; do not mistake an exported Nix check for an exercised check [E:README.md:L73-L101; E:flake.nix:L68-L84].

The family-facing README tells what children actually experience, that data stays local, how to try Anzan and Kids, and candid limitations; engineering material lives in DEVELOPERS.md. `AGENTS.md` requires topic branch, kit work on wx-workstation rather than laptop, coordinated Sekka pin/Cargo.lock, Day/Night golden inspection, honest headless-versus-live evidence, no manipulative children's features [E:README.md:L1-L13,L49-L101; E:AGENTS.md:L1-L17]. `DEVELOPERS.md` documents the layout, sibling checkout, lock-update process, remote workflow, tests and component limits; `rustfmt.toml` sets 120-column width and `use_small_heuristics = "Max"`; no CLAUDE.md or clippy config appears in the snapshot [E:DEVELOPERS.md:L1-L78; E:rustfmt.toml:L1-L3]. Anzan tests run kit Harness with a manual clock and Day/Night `tests/golden/*.png`, Kids tests render a report pair, and an ignored frame-budget test measures drawing; README warns these are headless component tests, not actual user/VM validation [E:apps/anzan/tests/anzan.rs:L13-L16,L31-L52,L282-L287; E:apps/kids/tests/kids.rs:L7-L10,L89-L95,L152-L157; E:DEVELOPERS.md:L58-L78,L157-L169].

## 2. How Sekka ships the external app, and what changes at the kit boundary

Sekka's own `desktop/aizome` now contains Sukumo. Sukumo's crates retain the names `sidestep-engine` and `sidestep-ui`, but are **local paths**, not a fresh Sidestep Git checkout. Aizome moved from the historical `bitemyapp/aizome` repository; its packages/module/checks/dev shell are now supplied by Sekka `desktop/aizome/nix/default.nix` [S:DEVELOPERS.md:L1480-L1496,L2156-L2168]. Consequently Uta's independent flake and Sekka's Uta build are **different integration paths**; copying Uta's former-Aizome pin blindly would be wrong for a *new* app built against present Sekka [U:flake.nix:L4-L8; S:nix/uta.nix:L1-L4; E:flake.nix:L4-L8].

The precise `.#uta` path is `inputs.uta = { url = "github:bitemyapp/uta/6281c579fa213294327afc8be5fdb6711b528925"; flake = false; };` in Sekka's flake: a **pinned flake input containing Uta's source**, not fetchFromGitHub, not vendored in Sekka and not Uta's own flake output. Sekka imports its own `nix/components.nix`, which imports `nix/uta.nix` using `applications.legacyPackages`, the pinned source and Aizome's session fonts; `packages.x86_64-linux.uta` inherits that catalog package [S:flake.nix:L22-L31,L47-L65,L254-L255; S:nix/components.nix:L42-L63]. Sekka's wrapper calls **Uta's own** `${uta}/nix/package.nix`, passes `src = uta`, the fileset of current `desktop/aizome` workspace/kit/sibling crates and Sukumo, its own Python helper, fonts, and `pkgs.emptyDirectory` for the now-unused historical Sidestep slot. It applies an MPRIS-readiness patch and supplies license/source provenance, not a forked copy of Uta's package [S:nix/uta.nix:L11-L41,L42-L98].

Genkan's catalog JSON entry is `id=uta`, `source=sekka`, `packages=["uta"]`, media category, Wayland description, both CPU platforms, and **`preload=false`**. The test asserts it is selectable but neither preloaded nor included in the ISO; cache publication deliberately includes `uta` and `uta.source` [S:installer/genkan/rust/src/applications.json:L31-L34; S:nix/uta-installation-test.nix:L11-L41; S:nix/push-cache.nix:L45-L50]. Hence users can choose Uta in Genkan or Yukimi (download at installation/discovery), and Sekka users can build `.#uta`; it is not present offline on the installation ISO, while ordinary Nix users can instead use Uta's own flake/module [S:DEVELOPERS.md:L1769-L1780; S:docs/applications.md:L93-L115; U:README.md:L27-L45]. Tsutsumi and Sekka Edu **are not present in the currently inspected Sekka catalog or components inputs**; their own flakes support `nix run github:bitemyapp/tsutsumi`, `nix run github:bitemyapp/sekka-edu`, or importing their respective NixOS modules [S:installer/genkan/rust/src/applications.json:L1-L39; S:nix/components.nix:L4-L10,L42-L63; T:README.md:L10-L28; E:README.md:L73-L97].

The ABI and lock choreography is explicit. A changed kit or Sukumo API must leave `.#uta` building; if a path crate is newly needed, add it to Sekka's selective `nix/uta.nix` fileset. Crates.io additions/bumps or path changes require updating **Uta's own** `Cargo.lock`, since Nix vendors exactly that lock. Work on Uta's **`sekka` branch**, not `main`: arrange `uta/` and Sekka `desktop/aizome` as sibling `aizome/`, run `cargo update --workspace` in Sekka's Aizome shell, build locked, commit updated lock/source, then update Sekka `inputs.uta.url`, flake lock, documented pin, and build the package and RustyPipe/installation checks in the same Sekka PR. Cargo `[patch]` settings in kit's workspace do not automatically cross the workspace boundary. Broken Uta blocks cache/nightly publication, which is why the pin is fixed rather than followed by nightly input updates [S:DEVELOPERS.md:L1705-L1748; S:flake.nix:L22-L30]. Sekka Edu's **corresponding** instructions move `inputs.sekka` and sibling checkout to one revision, update its own lock, then lock and build its flake; its current pinned Sekka commit `cd82f3c…` is *not* the checkout commit `aebca32…` inspected here [E:DEVELOPERS.md:L15-L38; E:flake.nix:L4-L8; S:flake.nix:L1-L4].

## 3. Aizome-design: the applicable visual grammar

The design repository is intentionally separate from Sekka/Sidestep and records a private canvas as of 14 October 2026. It distinguishes **settled** Aizome desktop decisions from reference/candidate boards, and expressly says the full third-party app icon set and app icons beyond the Aizome mark are **not settled**; there is no audio mixer board or Musubi-specific icon specification in its listed material [D:README.md:L1-L7,L68-L109,L119-L123]. Its concrete inventory: `tokens/aizome-tokens.json` is the authoritative Night/Day roles, brand, focus, typography, dimensions, logo geometry; `tokens/aizome.css` is generated `--az-*`; `boards/` has day/night desktop and menu renders plus name/mark and mark-spec boards; `logo/svg/` has dark/light/mono/symbolic marks, wordmarks, lockups, and 64-grid app icon, `logo/png/` contains size-specific pixel renders; `logo/source/{genlogo.py,build.sh}` regenerates masters. `reference/{html,png,canvas-source,wallpapers}` preserves rejected candidates and source canvas; the wallpaper credit file is `reference/wallpapers/CREDITS-from-tatami.md` [D:README.md:L62-L107,L111-L128]. A visual designer should open the two desktop and menu day/night boards and the mark construction boards, but should not treat Harbor/Strip/Rooms candidate art as settled Aizome policy [D:README.md:L13-L23,L96-L109].

For a matrix/mixer: use kit theme roles rather than inventing a second palette [INFERENCE], mapping chrome `#222436` Night / `#dde0ec` Day to toolbars, surface `#1a1b26` / `#f4f5fa` to content, sunk to wells, line to dividers, `selection` to selected rows, green/red **with text or icon** for success/danger, amber for warning or progress; do not put important meter labels in `faint` (under 4.5:1). These exact roles and accessibility warning come from token definitions [D:tokens/aizome-tokens.json:L7-L35]. Night is primary; focus can be amber/cyan/rose/lime, while the Aizome mark's amber **never** follows focus. Fonts: Archivo UI, IBM Plex Mono for numeric meter readouts and shortcuts, Noto Serif JP for Kanji only; use tabular digits for meters. Layout constants are 4pt grid, minimum 24pt pointer targets, 1pt bevel and 3pt hard unblurred offset shadow [D:tokens/aizome-tokens.json:L37-L49,L59-L77; D:README.md:L21-L36,L56-L64].

Preserve integer-first scaling 1×/2×/3×, opaque regions, repaint damage only (idle means zero frames), keyboard access to every control, Super as command modifier, short motion ≤150ms (or none for reduced motion), and make the UI finished at Austere effects tier. For an audio patch matrix, readable row/column labels, keyboard traversal, accurate numeric levels and damage-scoped live meters are reasonable applications of those rules **[INFERENCE]**, not design-repo-prescribed audio-specific screens [D:README.md:L25-L36; D:tokens/aizome-tokens.json:L59-L77]. The Aizome *brand* mark itself is three indigo stripes and fixed amber top-left tab, with 2m clear space and mono variant below 24px. Musubi needs its own distinct app icon **[INFERENCE]**: Uta and Tsutsumi use 64-grid app-specific SVGs and the Aizome palette, rather than reusing the desktop logo [D:README.md:L38-L60,L73-L89; U:data/org.aizome.Uta.desktop:L3-L10; T:data/org.aizome.Tsutsumi.desktop:L3-L12]. Naming precedent is a short Japanese romanization + kanji in public title and an immediately understandable English generic name, with an `org.aizome.*` reverse-domain ID for independent Aizome apps; the educational **Sekka** apps instead use `computer.sekka.*`. This is observed precedent, **not a formal rule stated by aizome-design** [U:README.md:L1-L7; U:data/org.aizome.Uta.desktop:L1-L7; T:README.md:L1-L6; T:data/org.aizome.Tsutsumi.desktop:L1-L7; E:apps/anzan/data/computer.sekka.Anzan.desktop:L1-L9].

## 4. Style, attribution and releases

There is no common formal conventional-commit syntax evident from the four latest visible commits: subjects include “Import LikeStatus from ytmusicapi's top level”, “Build Tsutsumi, native encrypted backup and restore for Aizome”, “Say the Nix package hasn't been built yet”, and “Stash the settled Aizome design”. These are **commit metadata**, not source-file claims: [Uta commit](https://github.com/bitemyapp/uta/commit/bb4523b798640041794a3f48415c34e50e5f7c6e), [Tsutsumi commit](https://github.com/bitemyapp/tsutsumi/commit/4c5274179d17d9fea52b2f7c752e11a523fbc6c0), [Edu commit](https://github.com/bitemyapp/sekka-edu/commit/8ad1e4ff41f0a141eec3624599665b19173d8c81), [design commit](https://github.com/bitemyapp/aizome-design/commit/f36cb67b66e8371740b2a183e43027b90ad521cc). Uta, Edu and design latest commit metadata carry `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`; Tsutsumi's latest inspected commit has no such trailer (same four commit links). Sekka's documented own workflow is topic branch → PR → squash merge → delete branch, with releases identified by version tags and nightly branch managed exclusively by workflow [S:DEVELOPERS.md:L1-L4]. Follow any repo's actual `AGENTS.md` and Sekka's attribution policy rather than inferring a universal co-author trailer from those samples [E:AGENTS.md:L1-L17; S:DEVELOPERS.md:L1-L4].

All three apps use edition 2024 and Rust 1.95; the single-app manifests share the `line-tables-only` dev debug, dependency debug=false and opt-level=2, tiny-skia overflow checks=false, thin-LTO release settings. Edu explicitly explains tiny-skia's intentional wrapping and adds release `debug = "line-tables-only"`; its `rustfmt.toml` alone specifies width 120/Max heuristics [U:Cargo.toml:L34-L46; T:Cargo.toml:L21-L30; E:Cargo.toml:L23-L38; E:rustfmt.toml:L1-L3]. No bespoke clippy config or root copyright header is visible in these snapshots; compare Edu's documented `//!` prose with links and concrete data layout rather than using hollow module summary comments [E:crates/sekka-kid/src/lib.rs:L1-L21; E:DEVELOPERS.md:L1-L13]. The separate `LICENSE-MIT` and `LICENSE-APACHE` files correspond to Cargo's dual-license expression, while Uta's default vendored RustyPipe changes combined-binary licensing [E:Cargo.toml:L5-L10; U:README.md:L238-L243; T:Cargo.toml:L1-L8]. Versions are `0.1.0` and no version-tag/release automation exists in the inspected standalone CI declarations; do not invent a release cadence [U:.github/workflows/ci.yml:L1-L28; T:.github/workflows/ci.yml:L1-L19; E:README.md:L73-L101].

## 5. Recommended **aizome-musubi** skeleton

**Closest packaging sibling:** Tsutsumi: one native Rust Aizome-kit executable, one Nix package, one desktop/icon pair, NixOS enable option, a simple build CI, actual Harness/UI and backend fixtures. Uta is useful for media/audio and Sekka's downstream catalog pin procedure, but its Python/Deno/mpv/RustyPipe/service stack and GPL implications are not warranted for a PipeWire patchbay. Edu gives the **correct newer kit provenance**: pin Sekka containing local Sukumo, rather than copying historical Aizome/Sidestep inputs from either single-app sibling [T:Cargo.toml:L1-L19; T:flake.nix:L1-L36; U:Cargo.toml:L10-L28; E:flake.nix:L4-L38; S:DEVELOPERS.md:L2156-L2168]. Recommended *source* and tests are a design recommendation **[INFERENCE]**, not files already in existence:

```text
aizome-musubi/
  Cargo.toml  Cargo.lock  flake.nix  flake.lock
  LICENSE-MIT  LICENSE-APACHE  README.md  DEVELOPERS.md  AGENTS.md
  rustfmt.toml                       # if choosing Edu's 120-column convention
  .github/workflows/ci.yml
  nix/package.nix
  data/org.aizome.Musubi.desktop
  data/org.aizome.Musubi.svg
  src/main.rs  src/lib.rs  src/app.rs  src/pipewire.rs  src/matrix.rs
  tests/app.rs  tests/pipewire.rs  tests/snapshots/{matrix-night,matrix-day}.png
```

The minimal single-package manifest should use the **exact new sibling path form** that resolves when `aizome-musubi/` and `sekka/` are siblings; the two older apps use `../aizome/...` because they still address the *former* repository. Add PipeWire crates only when their API and version have actually been selected, and keep all runtime library/input decisions in `nix/package.nix` **[INFERENCE]** [E:Cargo.toml:L1-L21; E:DEVELOPERS.md:L15-L26; T:Cargo.toml:L1-L19]:

```toml
[package]
name = "aizome-musubi"
version = "0.1.0"
edition = "2024"
rust-version = "1.95"
description = "Musubi 結び, the PipeWire patchbay for Aizome"
license = "MIT OR Apache-2.0"
repository = "https://github.com/bitemyapp/aizome-musubi"

[dependencies]
aizome-kit = { path = "../sekka/desktop/aizome/crates/aizome-kit" }

[profile.dev]
debug = "line-tables-only"
[profile.dev.package."*"]
debug = false
opt-level = 2
[profile.dev.package.tiny-skia]
overflow-checks = false
[profile.release]
lto = "thin"
codegen-units = 4
```

**Which pin?** The closest *modern* sibling `sekka-edu` actually pins Sekka `cd82f3c1ab8c06dd1fdc70ec6f49c37ba11c089c` [E:flake.nix:L4-L8]. The *current local Sekka checkout examined here* is commit `aebca32e5ef6245a5ec68d2c3c8953ca957c3375`; the source snapshot is cited as S throughout. For a Musubi that targets this current kit, recommend pinning **`aebca32e5ef6245a5ec68d2c3c8953ca957c3375`**, provided that commit is available as a published GitHub revision: the sibling pin `cd82f3c…` is an older known working **reference**, not proof of current kit compatibility. The repo can choose its released target revision and develop/test against the *same* sibling checkout; update `Cargo.lock` and the flake lock together when changing it **[INFERENCE]** [E:DEVELOPERS.md:L15-L38; S:DEVELOPERS.md:L1705-L1748]. Do **not** add direct Sidestep Git deps or pretend a Sidestep rev of `16a1e…` is today's Sukumo; the latter lives in the pinned Sekka source. The kit resolves `sidestep-ui.workspace = true` through Sekka's Aizome workspace [S:desktop/aizome/crates/aizome-kit/Cargo.toml:L1-L23; S:DEVELOPERS.md:L2156-L2168].

Flake outline **[INFERENCE]**, adapted from Edu's selective kit closure and Tsutsumi's single app: `inputs.sekka.url = "github:bitemyapp/sekka/aebca32e5ef6245a5ec68d2c3c8953ca957c3375"; inputs.nixpkgs.follows = "sekka/nixpkgs";` then `outputs = { self, nixpkgs, sekka }: …`; take `aizomeFor` and `fontsFor` from Edu nearly verbatim, using only the kit's required crates and Sukumo, so Nix stages `${src}` at `$out/aizome-musubi` and `${aizomeFor pkgs}` at `$out/sekka/desktop/aizome` [E:flake.nix:L4-L38; E:nix/package.nix:L18-L29]. Define `packages.${system}.musubi` and `default`, `apps.${system}.default.program = "${self.packages.${system}.musubi}/bin/aizome-musubi"`, `devShells.default` with Rust, pkg-config and relevant PipeWire runtime/dev dependencies, `checks.musubi` as the build, and `nixosModules.default.options.programs.musubi.enable` adding the package to `environment.systemPackages`. Decide x86_64-only versus both architectures based on actually supportable PipeWire dependencies and tests **[INFERENCE]** [T:flake.nix:L10-L35; E:flake.nix:L17-L18,L41-L85]. A package recipe should use `rustPlatform.buildRustPackage`, its own committed `Cargo.lock`, fonts and runtime library paths, include the genuine PipeWire runtime dependency, and install `data/org.aizome.Musubi.desktop` to `share/applications/` and the SVG to `share/icons/hicolor/scalable/apps/` [T:nix/package.nix:L11-L35; E:nix/package.nix:L18-L36].

Desktop entry: `Name=Musubi`, `GenericName=Patchbay`, `Comment=Connect PipeWire audio and MIDI devices, mix levels and manage effects` **[INFERENCE]**, `Exec=aizome-musubi`, `Icon=org.aizome.Musubi`, `StartupWMClass=org.aizome.Musubi`, `Terminal=false`, `Categories=AudioVideo;Audio;Mixer;` **[INFERENCE]**. The precise Name/GenericName/icon/WMClass synchronization and `org.aizome.*` form follow Uta/Tsutsumi; design the icon with the Night/Day palette and 64-grid hard-shadow language without claiming that Aizome's brand mark itself is Musubi's icon [U:data/org.aizome.Uta.desktop:L1-L12; T:data/org.aizome.Tsutsumi.desktop:L1-L13; D:README.md:L56-L60,L73-L89]. `AppInfo::new("org.aizome.Musubi", "Musubi", "Patchbay")` should agree with that descriptor **[INFERENCE]** [T:src/app.rs:L381-L383].

**Installation:** before any Sekka catalog change, `nix run github:bitemyapp/aizome-musubi` and `imports = [ inputs.musubi.nixosModules.default ]; programs.musubi.enable = true;` are the independent-flake route **[INFERENCE]** based on Tsutsumi [T:README.md:L10-L28; T:flake.nix:L22-L35]. `nix build .#musubi` *from Sekka* would not exist until a separate Sekka PR adds a pinned `flake = false` input and `nix/components.nix` import/package forwarding, plus catalog JSON entry (`source=sekka`, `packages=["musubi"]`), platform/test/cache policy and explicit lock/pin maintenance. One must not announce that it already ships on Sekka. If an optional downloaded application, set catalog `preload=false`; if desired offline, define and test the distinct ISO preload/cache policy **[INFERENCE]** [S:flake.nix:L22-L31,L254-L255; S:nix/components.nix:L42-L63; S:installer/genkan/rust/src/applications.json:L31-L34; S:DEVELOPERS.md:L1769-L1780].

README should lead with the human experience: **Musubi 結び — Patchbay**, what connections/mixer/effects are actually supported, screenshot for Night and Day, a truthful run/install section, keyboard and accessibility directions, what happens when PipeWire is absent, data/privacy, limitations, then license. DEVELOPERS should document source/daemon boundary, pinned Sekka/Sukumo and exact lock-update choreography, real hardware/permissions assumptions, fixtures and tests, and clearly distinguish headless Harness goldens from a live-session/VM result. AGENTS should direct contributors to this DEVELOPERS document, require keeping flake pin, sibling kit checkout and Cargo.lock together, inspect both-theme goldens, and honestly report what was run. These are recommendations **[INFERENCE]** supported by the siblings' user/developer split, not an already-existing Musubi policy [U:README.md:L1-L45,L172-L243; T:README.md:L1-L32,L139-L177; E:AGENTS.md:L1-L17; E:DEVELOPERS.md:L15-L78].
