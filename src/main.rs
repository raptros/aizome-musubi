use aizome_musubi::effects::{
    paths::Paths,
    service::{EffectsService, Systemd},
};
use aizome_musubi::setups::{paths::Paths as SetupPaths, store::Store};
use aizome_musubi::{
    app::Musubi,
    instance::{self, Claim},
};
use std::io;

fn setups(args: &[String]) -> io::Result<()> {
    let store = Store { paths: SetupPaths::from_environment()? };
    match args {
        [action] if action == "list" => {
            for (id, result) in store.list()? {
                match result {
                    Ok(setup) => println!("{}\t{}", id, setup.name),
                    Err(error) => println!("{}\tERROR: {}", id, error),
                }
            }
            Ok(())
        }
        [action, id] if action == "remove" => store.delete(id),
        [action] if action == "remove-all" => store.remove_all(),
        _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "setups: list | remove <id> | remove-all")),
    }
}

fn effects(args: &[String]) -> io::Result<()> {
    let service =
        EffectsService { paths: Paths::from_environment()?, units: Systemd::user().map_err(io::Error::other)? };
    match args {
        [action] if action == "list" => {
            for chain in service.list()? {
                println!("{}\t{}\t{}", chain.id, if chain.enabled { "enabled" } else { "disabled" }, chain.name);
            }
            Ok(())
        }
        [action, id] if action == "remove" => service.delete(id),
        [action] if action == "remove-all" => service.remove_all(),
        _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "effects: list | remove <chain> | remove-all")),
    }
}

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if arguments.first().is_some_and(|a| a == "effects") {
        if let Err(error) = effects(&arguments[1..]) {
            eprintln!("Musubi effects: {error}");
            std::process::exit(1);
        }
        return;
    }
    if arguments.first().is_some_and(|a| a == "setups") {
        if let Err(error) = setups(&arguments[1..]) {
            eprintln!("Musubi setups: {error}");
            std::process::exit(1);
        }
        return;
    }
    let mut args = std::env::args().skip(1);
    match (args.next().as_deref(), args.next()) {
        (Some("--version"), None) => {
            println!("aizome-musubi {}", env!("CARGO_PKG_VERSION"));
            return;
        }
        (Some("--help" | "-h"), None) => {
            println!(
                "Musubi 結び — the PipeWire patchbay for Aizome\n\nUsage: aizome-musubi [--help | --version | effects list | effects remove <chain> | effects remove-all | setups list | setups remove <id> | setups remove-all]"
            );
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
