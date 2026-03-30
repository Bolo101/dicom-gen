use dicom_object::open_file;
use dicom_dictionary_std::tags;

pub fn inspect_file(path: &str) {
    match open_file(path) {
        Ok(obj) => {
            println!("=== Inspection du fichier DICOM ===");

            // Patient
            if let Ok(elem) = obj.element(tags::PATIENT_NAME) {
                println!("Patient      : {}", elem.to_str().unwrap_or_default());
            }
            if let Ok(elem) = obj.element(tags::PATIENT_ID) {
                println!("Patient ID   : {}", elem.to_str().unwrap_or_default());
            }

            // Etude
            if let Ok(elem) = obj.element(tags::STUDY_DATE) {
                println!("Date étude   : {}", elem.to_str().unwrap_or_default());
            }

            // Image
            if let Ok(elem) = obj.element(tags::SOP_CLASS_UID) {
                println!("SOP Class    : {}", elem.to_str().unwrap_or_default());
            }
            if let Ok(elem) = obj.element(tags::MODALITY) {
                println!("Modalité     : {}", elem.to_str().unwrap_or_default());
            }
        }
        Err(e) => println!("Erreur lecture : {}", e),
    }
}