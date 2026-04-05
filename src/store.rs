use dicom_object::open_file;
use dicom_ul::association::client::ClientAssociationOptions;
use dicom_ul::pdu::{PDataValue, PDataValueType, Pdu};
use std::io::Write;

// Transfer Syntax: Explicit VR Little Endian
// The most widely supported encoding in modern DICOM implementations
const EXPLICIT_VR_LE: &str = "1.2.840.10008.1.2.1";

// ============================================================
// READ SOP CLASS AND INSTANCE UID FROM A DICOM FILE
// ============================================================
//
// Before opening an association, we need two pieces of information
// from the file itself:
//
//   SOP Class UID    → identifies the type of data (CT, MR, X-Ray...)
//                      used to negotiate the Presentation Context
//
//   SOP Instance UID → unique identifier for this specific image
//                      included in the C-STORE-RQ command set
//
// Both are returned as a tuple (sop_class, sop_instance).
//
fn read_dicom_info(path: &str) -> Result<(String, String), Box<dyn std::error::Error>> {
    let obj = open_file(path)?;

    // Read the SOP Class UID from the DICOM file metadata
    let sop_class = obj
        .element_by_name("SOPClassUID")?
        .to_str()?
        .trim()
        .to_string();

    // Read the SOP Instance UID from the DICOM file metadata
    let sop_instance = obj
        .element_by_name("SOPInstanceUID")?
        .to_str()?
        .trim()
        .to_string();

    println!("[C-STORE] SOP Class    : {}", sop_class);
    println!("[C-STORE] SOP Instance : {}", sop_instance);

    Ok((sop_class, sop_instance))
}
