use aizome_musubi::{
    app::Musubi,
    instance::{self, Claim},
};

fn main() {
    let mut args = std::env::args().skip(1);
    match (args.next().as_deref(), args.next()) {
        (Some("--version"), None) => {
            println!("aizome-musubi {}", env!("CARGO_PKG_VERSION"));
            return;
        }
        (Some("--help" | "-h"), None) => {
            println!("Musubi 結び — the PipeWire patchbay for Aizome\n\nUsage: aizome-musubi [--help | --version]");
            return;
        }
        (None, None) => {}
        _ => {
            eprintln!("Use --help for usage.");
            std::process::exit(2);
        }
    }
    let mut app = Musubi::default();
    app.live = true;
    match instance::claim() {
        Ok(Claim::Handed) => return,
        Ok(Claim::First(instance)) => app.instance = Some(instance),
        Err(error) => {
            eprintln!("Musubi: {error}");
            std::process::exit(1);
        }
    }
    aizome_kit::run(app);
}
