use miette::IntoDiagnostic;
use owo_colors::OwoColorize;
use std::{
    fs::{self, hard_link, remove_file, File},
    io::{self, Read, Write},
    path::PathBuf,
    process::Command,
};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about = "Switch cargo configurations with ease.")]
enum Config {
    /// Create a new cargo config
    Create { value: String },
    /// Switch between cargo configs
    Switch { value: String },
    /// List configs
    List,
    /// Remove a config
    Remove { value: String },
    /// Launch an editor to edit a config
    Edit {
        #[arg(short, long)]
        editor: String,
        value: String,
    },
}

fn main() -> miette::Result<()> {
    initialise().into_diagnostic()?;
    let cfg = Config::parse();

    match cfg {
        Config::Create { value } => {
            create_config(&value)
                .and_then(|_| Ok(println!("Success:   {}  Created {value}.toml", "✓".green())))
                .into_diagnostic()?;
            Ok(())
        }
        Config::Switch { value } => {
            switch_config(&value)
                .and_then(|_| Ok(println!("Success:   {}  Switched to {value}", "✓".green())))
                .into_diagnostic()?;
            Ok(())
        }
        Config::List => {
            list_config().into_diagnostic()?;
            Ok(())
        }
        Config::Remove { value } => {
            remove_config(&value)
                .and_then(|_| Ok(println!("Success:   {}  Removed {value}", "✓".green())))
                .into_diagnostic()?;
            Ok(())
        }
        Config::Edit { editor, value } => {
            edit_config(&editor, &value)
                .and_then(|_| {
                    Ok(println!(
                        "Success:   {}  Opened {editor} at {value}",
                        "✓".green()
                    ))
                })
                .into_diagnostic()?;

            Ok(())
        }
    }
}

fn create_config(name: &str) -> io::Result<()> {
    let mut path = resolve_config_dir()?;

    path.push(format!("{name}.toml"));
    File::create_new(path)?;
    Ok(())
}

fn switch_config(name: &str) -> io::Result<()> {
    let mut path = resolve_config_dir()?;
    let mut cargo_config_current = path.clone();
    cargo_config_current.push("cargo-config-current");

    if File::open(&cargo_config_current).is_err() {
        File::create(&cargo_config_current)?;
    }

    let mut current = File::options().write(true).open(&cargo_config_current)?;

    write!(&mut current, "{name}")?;

    let mut cargo = resolve_cargo_dir()?;
    cargo.push("config.toml");

    remove_file(&cargo)?;

    path.push(format!("{name}.toml"));

    hard_link(path, cargo)?;
    Ok(())
}

fn list_config() -> io::Result<()> {
    let path = resolve_config_dir()?;
    let mut current = String::new();

    let mut cargo_config_current = path.clone();
    cargo_config_current.push("cargo-config-current");

    File::open(&cargo_config_current)?.read_to_string(&mut current)?;

    fs::read_dir(path).and_then(|entry| {
        println!("List of entries:");
        for e in entry {
            if let Ok(entry) = e {
                let os_file_name = entry.file_name();
                let file_name = os_file_name.to_string_lossy();
                let names = file_name.split(".toml").collect::<Vec<&str>>();
                let name = names[0];

                if name != "cargo-config-current" {
                    println!("- {}", name)
                }
            }
        }
        Ok(())
    })?;

    Ok(())
}

fn remove_config(name: &str) -> io::Result<()> {
    let mut path = resolve_config_dir()?;
    path.push(format!("{name}.toml"));

    fs::remove_file(path).map_err(|_| {
        std::io::Error::new(io::ErrorKind::NotFound, format!("{name} does not exist"))
    })?;
    Ok(())
}

fn edit_config(editor: &str, name: &str) -> io::Result<()> {
    let mut config_dir = resolve_config_dir()?;
    config_dir.push(format!("{name}.toml"));

    let ed = which::which(editor).map_err(|err| match err {
        which::Error::CannotFindBinaryPath => {
            io::Error::new(io::ErrorKind::NotFound, err.to_string())
        }
        which::Error::CannotGetCurrentDirAndPathListEmpty => {
            io::Error::new(io::ErrorKind::PermissionDenied, err.to_string())
        }
        which::Error::CannotCanonicalize => io::Error::new(io::ErrorKind::Other, err.to_string()),
    })?;

    Command::new(ed).arg(config_dir).spawn()?;

    Ok(())
}

fn resolve_config_dir() -> io::Result<PathBuf> {
    let mut path = simple_home_dir::home_dir().ok_or(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "Cargo directory could not be found",
    ))?;

    path.push(".cargo/cargo-config/");
    let _ = fs::create_dir(&path);
    Ok(path)
}

fn resolve_cargo_dir() -> io::Result<PathBuf> {
    let mut path = simple_home_dir::home_dir().ok_or(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "Cargo directory could not be found",
    ))?;

    path.push(".cargo");
    Ok(path)
}

fn initialise() -> io::Result<()> {
    let mut cargo_config_current = resolve_config_dir()?;
    cargo_config_current.push("cargo-config-current");

    if File::open(&cargo_config_current).is_err() {
        let mut current_path = resolve_cargo_dir()?;
        current_path.push("config.toml");

        if let Ok(mut cfg) = File::open(&current_path) {
            println!(
            "Warning:   {}  config.toml exists in Cargo directory, moving to cargo-config/config.toml",
            "⚠".yellow()
        );
            let mut tmp = vec![];
            let mut mv = resolve_config_dir()?;

            mv.push("config.toml");
            let mut file = File::create_new(mv)?;

            cfg.read_to_end(&mut tmp)?;
            file.write_all(&mut tmp)?;

            switch_config("config")?;
        }
    }
    Ok(())
}
