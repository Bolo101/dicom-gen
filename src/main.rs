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
}
