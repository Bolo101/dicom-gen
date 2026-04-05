mod cli;
mod echo;
mod inspect;
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
            None => println!("[INSPECT] Error: please provide a file with --file <path>"),
        }
        return;
    }

    // --- Header ---
    println!("=== DICOM-GEN ===");
    println!("Mode     : {:?}", args.mode);
    println!("Host     : {}:{}", args.host, args.port);
    println!("Command  : {:?}", args.command);
    println!("Calling  : {}", args.calling_aet);
    println!("Called   : {}", args.called_aet);

    if let TransportMode::Udp = args.mode {
        println!("TTL      : {}", args.ttl);
    }

    println!("---");

    // --- Command dispatch ---
    match (&args.command, &args.mode) {
        (DicomCommand::Echo, TransportMode::Tcp) => {
            match echo::send_echo(&args.host, args.port, &args.calling_aet, &args.called_aet) {
                Ok(()) => println!("[C-ECHO] Success ✓"),
                Err(e) => println!("[C-ECHO] Failed : {}", e),
            }
        }

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
            None => println!("[C-STORE] Error: please provide a file with --file <path>"),
        },

        _ => {
            println!("Command not yet implemented.");
        }
    }
}
