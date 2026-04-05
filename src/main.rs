mod cli;
mod echo;
mod inspect;
#[allow(unused)]
#[tokio::main]
async fn main() {
    use clap::Parser;
    use cli::{DicomCommand, TransportMode};

    let args = cli::Cli::parse();

    // --- Inspect mode ---
    if args.inspect {
        match &args.file {
            Some(path) => inspect::inspect_file(path),
            None => println!("[INSPECT] Error: please provide a file with --file <path>"),
        }
        return; // stop here, don't send any DICOM command
    }

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

    // Dispatch to the right handler based on command and transport mode
    match (&args.command, &args.mode) {
        (DicomCommand::Echo, TransportMode::Tcp) => {
            match echo::send_echo(&args.host, args.port, &args.calling_aet, &args.called_aet) {
                Ok(()) => println!("[C-ECHO] Success ✓"),
                Err(e) => println!("[C-ECHO] Failed : {}", e),
            }
        }
        // Other commands will be implemented in upcoming steps
        _ => {
            println!("Command not yet implemented.");
        }
    }
}
