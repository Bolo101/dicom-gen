use clap::{Parser, ValueEnum};

// ValueEnum enables clap to convert a string as "tcp" in TransportMode::Tcp
#[derive(Debug, Clone, ValueEnum)]
pub enum TransportMode {
    Tcp,
    Udp,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum DicomCommand {
    Echo,
    Store,
    Find,
    Move,
}

// Parser generates arguments parsing
#[derive(Debug, Parser)]
#[command(
    name = "dicom-gen",
    about = "A DICOM traffic generator for debugging and development",
    version = "0.1.0"
)]
pub struct Cli {
    /// Transport mode : tcp or udp
    #[arg(long, default_value = "tcp")]
    pub mode: TransportMode,

    /// Target host (IP or hostname)
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    /// Target DICOM port
    #[arg(long, default_value_t = 4242)]
    pub port: u16,

    /// DICOM command to send
    #[arg(long, default_value = "echo")]
    pub command: DicomCommand,

    /// Called AET (the server's DICOM name)
    #[arg(long, default_value = "ORTHANC")]
    pub called_aet: String,

    /// Calling AET (our DICOM name)
    #[arg(long, default_value = "DICOM-GEN")]
    pub calling_aet: String,

    /// TTL for UDP mode (ignored in TCP mode)
    #[arg(long, default_value_t = 64)]
    pub ttl: u8,

    /// Path to a DICOM file (used with --inspect or --command store)
    #[arg(long)]
    pub file: Option<String>,

    /// Inspect a DICOM file and print its metadata
    #[arg(long, default_value_t = false)]
    pub inspect: bool,

    /// Local IP address to bind to (optional, e.g. "192.168.1.10")
    #[arg(long)]
    pub local_ip: Option<String>,

    /// Number of UDP packets to send (UDP mode only)
    #[arg(long, default_value_t = 1)]
    pub count: u32,

    /// Delay between UDP packets in milliseconds (UDP mode only)
    #[arg(long, default_value_t = 1000)]
    pub interval: u64,
}
