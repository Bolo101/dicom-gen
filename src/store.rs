use dicom_object::open_file;
use dicom_ul::association::client::ClientAssociationOptions;
use dicom_ul::pdu::{PDataValue, PDataValueType, Pdu};
use std::io::Write;
use std::net::TcpStream;

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

// ============================================================
// SEND A C-STORE REQUEST
// ============================================================
//
// Full exchange:
//
//   SCU (us)                         SCP (Orthanc)
//     |                                   |
//     |--- A-ASSOCIATE-RQ --------------> |  negotiate CT Image Storage
//     | <-- A-ASSOCIATE-AC ------------- |  accepted
//     |                                   |
//     |--- P-DATA (C-STORE-RQ) ---------> |  command set
//     |--- P-DATA (dataset) -----------> |  the actual image bytes
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
    //
    // We need the SOP Class and Instance UIDs before opening the association,
    // because they define the Presentation Context to negotiate.
    //
    let (sop_class, sop_instance) = read_dicom_info(file_path)?;

    // Instead of slicing raw bytes (fragile, offset-dependent),
    // we use dicom-object to re-encode the dataset cleanly into a buffer.
    // write_dataset() serializes only the dataset (no preamble, no file meta),
    // which is exactly what the DICOM network protocol expects.
    let obj = open_file(file_path)?;
    let mut dataset_bytes: Vec<u8> = Vec::new();
    obj.write_dataset(&mut dataset_bytes)?;

    println!("[C-STORE] Dataset size : {} bytes", dataset_bytes.len());

    // --- BLOCK 2 : Establish the DICOM Association ---
    let addr = format!("{}:{}", host, port);
    println!("[C-STORE] Connecting to {}...", addr);

    let ts = EXPLICIT_VR_LE.to_string();

    let association = ClientAssociationOptions::new()
        .calling_ae_title(calling_aet)
        .called_ae_title(called_aet)
        .with_presentation_context(
            &sop_class, // Abstract Syntax : SOP Class of our image
            vec![&ts],  // Transfer Syntax
        );

    // dicom-ul 0.7.1 expects an AE address (&str) here,
    // so we let dicom-ul open the TCP connection itself.
    let mut association = association.establish(&addr)?;
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
        writer.write_all(&dataset_bytes)?;
    } // writer dropped here → final P-DATA fragment is sent
    println!("[C-STORE] Dataset sent");

    // --- BLOCK 5 : Read the C-STORE-RSP ---
    //
    // Orthanc should respond with a P-DATA containing the C-STORE-RSP
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
