mod cli;
mod inspect;

#[tokio::main]
async fn main() {
    use clap::Parser;

    let args = cli::Cli::parse();

    println!("=== DICOM-GEN ===");
    println!("Mode     : {:?}", args.mode);
    println!("Host     : {}:{}", args.host, args.port);
    println!("Command  : {:?}", args.command);
    println!("Calling  : {}", args.calling_aet);
    println!("Called   : {}", args.called_aet);

    if let cli::TransportMode::Udp = args.mode {
        println!("TTL      : {}", args.ttl);
    }
}
