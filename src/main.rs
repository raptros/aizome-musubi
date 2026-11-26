use aizome_musubi::effects::{
    paths::Paths,
    service::{EffectsService, Systemd},
};
use aizome_musubi::setups::{
    keeper::{self, Handover, UNIT},
    paths::Paths as SetupPaths,
    store::Store,
};
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
        [action, id] if action == "remove" => {
            let active = store.active()?.is_some_and(|active| active.id == *id);
            let mut errors = Vec::new();
            if active {
                if let Err(e) = stop_keeper() {
                    errors.push(e.to_string());
                }
            }
            if let Err(e) = store.delete(id) {
                errors.push(e.to_string());
            }
            if errors.is_empty() { Ok(()) } else { Err(io::Error::other(errors.join("; "))) }
        }
        [action] if action == "remove-all" => {
            let unit = stop_keeper();
            let files = store.remove_all();
            match (unit, files) {
                (Ok(()), Ok(())) => Ok(()),
                (Err(e), Ok(())) | (Ok(()), Err(e)) => Err(e),
                (Err(a), Err(b)) => Err(io::Error::other(format!("{a}; {b}"))),
            }
        }
        _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "setups: list | remove <id> | remove-all")),
    }
}

fn stop_keeper() -> io::Result<()> {
    use aizome_musubi::effects::service::Units;
    let units = Systemd::user().map_err(io::Error::other)?;
    let mut errors = Vec::new();
    if let Err(e) = units.stop(UNIT) {
        errors.push(e);
    }
    if let Err(e) = units.disable(UNIT) {
        errors.push(e);
    }
    if errors.is_empty() { Ok(()) } else { Err(io::Error::other(errors.join("; "))) }
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
    if arguments.as_slice() == ["--keeper"] {
        if let Err(error) = keeper::run() {
            eprintln!("Musubi keeper: {error}");
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
                "Musubi 結び — the PipeWire patchbay for Aizome\n\nUsage: aizome-musubi [--help | --version | --keeper | effects list | effects remove <chain> | effects remove-all | setups list | setups remove <id> | setups remove-all]"
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
    let _handover = match Handover::path().and_then(|path| Handover::acquire(&path)) {
        Ok(lock) => Some(lock),
        Err(error) => {
            eprintln!("Musubi: setup handover: {error}");
            return;
        }
    };
    aizome_kit::run(app);
}
