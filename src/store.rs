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

// ============================================================
// BUILD THE C-STORE-RQ COMMAND SET
// ============================================================
//
// A C-STORE-RQ command set contains the following elements:
//
//   (0000,0000) Command Group Length   → total byte length of elements that follow
//   (0000,0002) Affected SOP Class UID → type of image being stored (CT, MR...)
//   (0000,0100) Command Field          → 0x0001 = C-STORE-RQ
//   (0000,0110) Message ID             → unique ID for this request
//   (0000,0700) Priority               → 0x0000 = MEDIUM
//   (0000,0800) Command Data Set Type  → 0x0102 = dataset DOES follow
//   (0000,1000) Affected SOP Instance  → unique ID of the image being stored
//
// Key difference from C-ECHO:
//   (0000,0800) is 0x0102 instead of 0x0101 → tells the server a dataset follows
//
fn build_c_store_rq(message_id: u16, sop_class: &str, sop_instance: &str) -> Vec<u8> {
    // DICOM requires UI values to have an even byte length
    // We pad with \0 if the length is odd
    let sop_class_bytes = pad_uid(sop_class);
    let sop_instance_bytes = pad_uid(sop_instance);

    // Calculate the Command Group Length
    // Every element costs: 4 (tag) + 4 (length field) + N (value)
    let group_length: u32 = (4 + 4 + sop_class_bytes.len()) as u32 +    // (0000,0002) SOP Class UID
        (4 + 4 + 2) +                                // (0000,0100) Command Field
        (4 + 4 + 2) +                                // (0000,0110) Message ID
        (4 + 4 + 2) +                                // (0000,0700) Priority
        (4 + 4 + 2) +                                // (0000,0800) Data Set Type
        (4 + 4 + sop_instance_bytes.len()) as u32; // (0000,1000) SOP Instance UID

    let mut data: Vec<u8> = Vec::new();

    // (0000,0000) Command Group Length
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
    data.extend_from_slice(&4u32.to_le_bytes());
    data.extend_from_slice(&group_length.to_le_bytes());

    // (0000,0002) Affected SOP Class UID
    data.extend_from_slice(&[0x00, 0x00, 0x02, 0x00]);
    data.extend_from_slice(&(sop_class_bytes.len() as u32).to_le_bytes());
    data.extend_from_slice(&sop_class_bytes);

    // (0000,0100) Command Field = 0x0001 (C-STORE-RQ)
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data.extend_from_slice(&2u32.to_le_bytes());
    data.extend_from_slice(&0x0001u16.to_le_bytes());

    // (0000,0110) Message ID
    data.extend_from_slice(&[0x00, 0x00, 0x10, 0x01]);
    data.extend_from_slice(&2u32.to_le_bytes());
    data.extend_from_slice(&message_id.to_le_bytes());

    // (0000,0700) Priority = 0x0000 (MEDIUM)
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x07]);
    data.extend_from_slice(&2u32.to_le_bytes());
    data.extend_from_slice(&0x0000u16.to_le_bytes());

    // (0000,0800) Command Data Set Type = 0x0102 (dataset follows)
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x08]);
    data.extend_from_slice(&2u32.to_le_bytes());
    data.extend_from_slice(&0x0102u16.to_le_bytes());

    // (0000,1000) Affected SOP Instance UID
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x10]);
    data.extend_from_slice(&(sop_instance_bytes.len() as u32).to_le_bytes());
    data.extend_from_slice(&sop_instance_bytes);

    data
}

// ============================================================
// PAD A DICOM UID TO EVEN LENGTH
// ============================================================
//
// DICOM requires all UI (UID) values to have an even byte length.
// If the UID string has an odd length, we append a null byte.
//
fn pad_uid(uid: &str) -> Vec<u8> {
    let mut bytes = uid.as_bytes().to_vec();
    if bytes.len() % 2 != 0 {
        bytes.push(0x00);
    }
    bytes
}
