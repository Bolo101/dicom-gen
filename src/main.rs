#![allow(unused)]
mod cli;
mod inspect;

enum TransportMode {
    Tcp,             // mode TCP classique
    Udp { ttl: u8 }, // mode UDP avec un TTL (u8 = entier entre 0 et 255)
}

fn describe_mode(mode: &TransportMode) {
    match mode {
        TransportMode::Tcp => {
            println!("Mode : TCP (handshake DICOM complet)");
        }
        TransportMode::Udp { ttl } => {
            println!("Mode : UDP brut avec TTL = {}", ttl);
        }
    }
}

struct DicomConfig {
    host: String, // IP address or hostanem
    port: u16,    // network port (u16 = integer between 0 and 65535)
    aet: String,  // Application Entity Title
}

impl DicomConfig {
    // config display method
    fn display(&self) {
        println!(
            "Connexion vers {}:{} avec l'AET '{}'",
            self.host, self.port, self.aet
        );
    }
}

fn parse_port(input: &str) -> Result<u16, String> {
    // Convert string port in number
    match input.parse::<u16>() {
        Ok(port) => {
            if port == 0 {
                Err(String::from("Le port 0 n'est pas valide"))
            } else {
                Ok(port)
            }
        }
        Err(_) => Err(format!("'{}' n'est pas un numéro de port valide", input)),
    }
}

trait DicomMessage {
    fn message_type(&self) -> &str; // retourne le nom du message
    fn describe(&self); // affiche une description
}

struct CEcho {
    called_aet: String, // recipient
}

impl DicomMessage for CEcho {
    fn message_type(&self) -> &str {
        "C-ECHO"
    }

    fn describe(&self) {
        println!("[{}] Ping vers '{}'", self.message_type(), self.called_aet);
    }
}

async fn send_echo(host: &str, port: u16) -> Result<(), String> {
    println!("Envoi C-ECHO vers {}:{}...", host, port);
    // ici viendrait la vraie logique réseau
    Ok(()) // () signifie "rien" - comme void en C
}

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
