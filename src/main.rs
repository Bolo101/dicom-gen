mod cli;
mod echo;
mod inspect;
mod network;
mod store;

#[tokio::main]
async fn main() {
    use clap::Parser;
    use cli::{DicomCommand, TransportMode};

    let args = cli::Cli::parse();

    // --- Inspect mode (early return, no header) ---
    if args.inspect {
        match &args.file {
            Some(path) => inspect::inspect_file(path),
            None => println!("[INSPECT] Error: please provide a file with --file "),
        }
        return;
    }

    // --- Header ---
    println!("=== DICOM-GEN ===");
    println!("Mode : {:?}", args.mode);
    println!("Host : {}:{}", args.host, args.port);
    println!("Command : {:?}", args.command);
    println!("Calling : {}", args.calling_aet);
    println!("Called : {}", args.called_aet);

    if let TransportMode::Udp = args.mode {
        println!("TTL : {}", args.ttl);
        println!("Count : {}", args.count);
        println!("Interval : {}ms", args.interval);

        if let Some(ref ip) = args.local_ip {
            println!("Local IP : {}", ip);
        }
    }

    println!("---");

    // Build the network config used by both TCP and UDP modes
    let net_config = network::NetworkConfig::new(args.local_ip.clone(), args.ttl);

    // --- Command dispatch ---
    match (&args.command, &args.mode) {
        // TCP C-ECHO
        (DicomCommand::Echo, TransportMode::Tcp) => {
            match echo::send_echo(
                &args.host,
                args.port,
                &args.calling_aet,
                &args.called_aet,
                args.count,
            ) {
                Ok(()) => println!("[C-ECHO] Success ✓"),
                Err(e) => println!("[C-ECHO] Failed : {}", e),
            }
        }

        // TCP C-STORE
        (DicomCommand::Store, TransportMode::Tcp) => match &args.file {
            Some(path) => {
                match store::send_store(
                    &args.host,
                    args.port,
                    &args.calling_aet,
                    &args.called_aet,
                    path,
                ) {
                    Ok(()) => println!("[C-STORE] Success ✓"),
                    Err(e) => println!("[C-STORE] Failed : {}", e),
                }
            }
            None => println!("[C-STORE] Error: please provide a file with --file "),
        },

        // UDP mode — raw packets with TTL control, no DICOM handshake
        (_, TransportMode::Udp) => {
            match network::send_udp_packets(
                &args.host,
                args.port,
                &net_config,
                args.count,
                args.interval,
            ) {
                Ok(()) => println!("[UDP] Success ✓"),
                Err(e) => println!("[UDP] Failed : {}", e),
            }
        }

        _ => {
            println!("Command not yet implemented.");
        }
    }
}
