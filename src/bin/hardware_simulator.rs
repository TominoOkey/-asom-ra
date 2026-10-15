use clap::Parser;

#[path = "hardware_simulator/protocol.rs"]
mod protocol;
#[cfg(unix)]
#[path = "hardware_simulator/pty.rs"]
mod pty;

use protocol::{run, valid_port_name};
#[cfg(unix)]
use pty::launch_with_pty;
use serialport::SerialPort;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(about = "Simulátor časomíry a sériového protokolu")]
struct Args {
    #[arg(long, help = "Vypíše dostupné sériové porty")]
    list: bool,
    #[arg(long, num_args = 2..=6, value_names = ["APP_PORT", "PEER_PORT"], help = "Vytvoří jeden až tři páry portů pomocí com0com setupc")]
    create: Vec<String>,
    #[arg(long, default_value = "COM1")]
    input: String,
    #[arg(long, default_value = "COM20")]
    out1: String,
    #[arg(long, default_value = "COM20", hide = true)]
    out2: String,
    #[arg(
        long,
        value_name = "APP",
        help = "Na macOS vytvoří pseudoporty a spustí Rust aplikaci"
    )]
    launch_app: Option<PathBuf>,
    #[arg(
        long,
        value_name = "SOCKET",
        help = "Parallels serial socket for COM1 (default: $TMPDIR/COM20)"
    )]
    parallels_socket: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();
    if let Some(app) = &args.launch_app {
        #[cfg(unix)]
        if let Err(e) = launch_with_pty(app, args.parallels_socket.as_deref()) {
            eprintln!("Spuštění se nezdařilo: {e}");
            std::process::exit(1);
        }
        #[cfg(not(unix))]
        {
            let _ = app;
            eprintln!(
                "--launch-app s pseudoporty je podporováno na macOS/Linux; na Windows vytvořte páry com0com."
            );
            std::process::exit(2);
        }
        return;
    }
    if args.list {
        match serialport::available_ports() {
            Ok(ports) if !ports.is_empty() => println!(
                "Dostupné sériové porty: {}",
                ports
                    .iter()
                    .map(|p| p.port_name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Ok(_) => println!("Nebyly nalezeny žádné sériové porty."),
            Err(e) => {
                eprintln!("Porty nelze vypsat: {e}");
                std::process::exit(1);
            }
        }
        println!("Pro výstup aplikace a externí program potřebujete propojený pár COM19 a COM20.");
        return;
    }
    if !args.create.is_empty() {
        if args.create.len() % 2 != 0 {
            eprintln!("Zadejte názvy portů po dvojicích, například --create COM19 COM20.");
            std::process::exit(2);
        }
        for pair in args.create.chunks_exact(2) {
            if pair.iter().any(|s| !valid_port_name(s)) {
                eprintln!("Názvy portů musí být ve formátu COM3, COM4 atd.");
                std::process::exit(2);
            }
            let command = format!("install PortName={} PortName={}", pair[0], pair[1]);
            println!("Pár {} <-> {}: {command}", pair[0], pair[1]);
            let status = std::process::Command::new("setupc").arg(&command).status();
            match status {
                Ok(s) if s.success() => {}
                Ok(s) => {
                    eprintln!("com0com setupc skončil stavem {s}");
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!(
                        "setupc nebyl nalezen ({e}); nainstalujte com0com a vytvořte páry výše uvedenými příkazy."
                    );
                    return;
                }
            }
        }
        return;
    }

    if let Err(e) = run(&args.input, &[&args.out1]) {
        eprintln!("Port nelze otevřít: {e}");
        eprintln!(
            "Zkontrolujte, že virtuální COM porty existují a nejsou používány jiným programem."
        );
        std::process::exit(1);
    }
}
