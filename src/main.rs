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

fn main() {
    let config = DicomConfig {
        host: String::from("127.0.0.1"),
        port: 4242,
        aet: String::from("DICOM-GEN"),
    };

    println!("Hôte   : {}", config.host);
    println!("Port   : {}", config.port);
    println!("AET    : {}", config.aet);
    config.display();

    let mode_tcp = TransportMode::Tcp;
    let mode_udp = TransportMode::Udp { ttl: 10 };

    describe_mode(&mode_tcp);
    describe_mode(&mode_udp);

    match parse_port("4242") {
        Ok(port) => println!("Port valide : {}", port),
        Err(msg) => println!("Erreur : {}", msg),
    }

    match parse_port("0") {
        Ok(port) => println!("Port valide : {}", port),
        Err(msg) => println!("Erreur : {}", msg),
    }

    match parse_port("abc") {
        Ok(port) => println!("Port valide : {}", port),
        Err(msg) => println!("Erreur : {}", msg),
    }
}
