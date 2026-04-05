use dicom_object::open_file;
use dicom_ul::association::client::ClientAssociationOptions;
use dicom_ul::pdu::{PDataValue, PDataValueType, Pdu};
use std::io::Write;

// ============================================================
// CONSTANTS
// ============================================================

// Fallback Transfer Syntax: Explicit VR Little Endian
// Used only if the file's Transfer Syntax cannot be determined
const EXPLICIT_VR_LE: &str = "1.2.840.10008.1.2.1";

// ============================================================
// READ SOP CLASS, INSTANCE UID AND TRANSFER SYNTAX FROM A DICOM FILE
// ============================================================
//
// Before opening an association, we need three pieces of information:
//
//   SOP Class UID    → identifies the type of data (CT, MR, X-Ray...)
//                      used to negotiate the Presentation Context
//
//   SOP Instance UID → unique identifier for this specific image
//                      included in the C-STORE-RQ command set
//
//   Transfer Syntax  → how the dataset is encoded (uncompressed, JPEG, etc.)
//                      must match what we negotiate in the association,
//                      otherwise the server will abort the connection
//
// All three are returned as a tuple.
//
fn read_dicom_info(path: &str) -> Result<(String, String, String), Box<dyn std::error::Error>> {
    let obj = open_file(path)?;

    // Read the SOP Class UID from the dataset
    let sop_class = obj
        .element_by_name("SOPClassUID")?
        .to_str()?
        .trim()
        .to_string();

    // Read the SOP Instance UID from the dataset
    let sop_instance = obj
        .element_by_name("SOPInstanceUID")?
        .to_str()?
        .trim()
        .to_string();

    // Read the Transfer Syntax from the File Meta Information (group 0002)
    // This tells us how the pixel data is encoded in the file
    let transfer_syntax = obj.meta().transfer_syntax().to_string();

    println!("[C-STORE] SOP Class       : {}", sop_class);
    println!("[C-STORE] SOP Instance    : {}", sop_instance);
    println!("[C-STORE] Transfer Syntax : {}", transfer_syntax);

    Ok((sop_class, sop_instance, transfer_syntax))
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

// ============================================================
// SEND A C-STORE REQUEST
// ============================================================
//
// Full exchange:
//
//   SCU (us)                         SCP (Orthanc)
//     |                                   |
//     |--- A-ASSOCIATE-RQ --------------> |  negotiate using file's Transfer Syntax
//     | <-- A-ASSOCIATE-AC ------------- |  accepted
//     |                                   |
//     |--- P-DATA (C-STORE-RQ) ---------> |  command set
//     |--- P-DATA (dataset) -----------> |  the actual image bytes (as-is from file)
//     | <-- P-DATA (C-STORE-RSP) ------- |  Status = 0x0000 (Success)
//     |                                   |
//     |--- A-RELEASE-RQ ----------------> |
//     | <-- A-RELEASE-RP --------------- |
//
pub fn send_store(
    host: &str,
    port: u16,
    calling_aet: &str,
    called_aet: &str,
    file_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // --- BLOCK 1 : Read the DICOM file ---
    let (sop_class, sop_instance, transfer_syntax) = read_dicom_info(file_path)?;

    // Read the raw file bytes.
    // We send the dataset as-is from the file to preserve the original
    // encoding — especially important for compressed Transfer Syntaxes
    // (JPEG, JPEG2000, RLE...) where re-encoding would corrupt the data.
    let file_bytes = std::fs::read(file_path)?;

    // Skip the File Meta Information to get to the actual dataset.
    // A DICOM file is structured as:
    //   128 bytes  → preamble (ignored)
    //   4 bytes    → "DICM" magic signature
    //   12 bytes   → tag (0002,0000) + length + value  (the group length element)
    //   N bytes    → rest of File Meta Information
    //   ...        → dataset starts here
    //
    // information_group_length gives us N (the size of the meta after the group length element)
    let obj = open_file(file_path)?;
    let meta_len = obj.meta().information_group_length as usize;
    let dataset_start = 128 + 4 + 12 + meta_len;
    let dataset_bytes = &file_bytes[dataset_start..];

    println!("[C-STORE] File size    : {} bytes", file_bytes.len());
    println!("[C-STORE] Dataset size : {} bytes", dataset_bytes.len());

    // --- BLOCK 2 : Establish the DICOM Association ---
    //
    // We negotiate using the file's actual Transfer Syntax.
    // If we negotiated Explicit VR Little Endian but the file is JPEG-compressed,
    // Orthanc would receive incompatible data and abort the connection.
    //
    let addr = format!("{}:{}", host, port);
    println!("[C-STORE] Connecting to {}...", addr);

    // Use the file's Transfer Syntax, fall back to Explicit VR LE if empty
    let ts = if transfer_syntax.is_empty() {
        EXPLICIT_VR_LE.to_string()
    } else {
        transfer_syntax.clone()
    };

    let mut association = ClientAssociationOptions::new()
        .calling_ae_title(calling_aet)
        .called_ae_title(called_aet)
        .with_presentation_context(
            &sop_class, // Abstract Syntax : SOP Class of our image
            vec![&ts],  // Transfer Syntax : from the file itself
        )
        .establish(&addr)?;

    println!("[C-STORE] DICOM association established ✓");

    let pc_id = association.presentation_contexts()[0].id;

    // --- BLOCK 3 : Send the C-STORE-RQ command set ---
    //
    // The command set tells the server what we are about to send:
    // the SOP Class, the SOP Instance UID, and that a dataset follows.
    //
    let cmd_bytes = build_c_store_rq(1, &sop_class, &sop_instance);

    association.send(&Pdu::PData {
        data: vec![PDataValue {
            presentation_context_id: pc_id,
            value_type: PDataValueType::Command,
            is_last: true,
            data: cmd_bytes,
        }],
    })?;
    println!("[C-STORE] C-STORE-RQ command sent");

    // --- BLOCK 4 : Send the dataset (the actual image) ---
    //
    // send_pdata() returns a writer that automatically splits data into
    // PDU fragments if it exceeds the negotiated max PDU size.
    // The writer is flushed and finalized when it goes out of scope (drop).
    //
    {
        let mut writer = association.send_pdata(pc_id);
        writer.write_all(dataset_bytes)?;
    } // writer dropped here → final P-DATA fragment is sent
    println!("[C-STORE] Dataset sent");

    // --- BLOCK 5 : Read the C-STORE-RSP ---
    //
    // Orthanc responds with a P-DATA containing the C-STORE-RSP
    // command set, with Status = 0x0000 (Success).
    //
    match association.receive()? {
        Pdu::PData { data } => {
            println!(
                "[C-STORE] Response received ({} bytes) ✓",
                data[0].data.len()
            );
        }
        pdu => {
            println!("[C-STORE] Unexpected PDU: {:?}", pdu);
        }
    }

    // --- BLOCK 6 : Release the association ---
    association.release()?;
    println!("[C-STORE] Association released ✓");

    Ok(())
}
