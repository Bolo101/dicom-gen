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
    // &self signifie "je lis la fiche mais je ne la modifie pas"
    fn display(&self) {
        println!(
            "Connexion vers {}:{} avec l'AET '{}'",
            self.host, self.port, self.aet
        );
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
}
