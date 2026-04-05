use dicom_ul::association::client::ClientAssociationOptions;
use dicom_ul::pdu::{PDataValue, PDataValueType, Pdu};
use std::net::TcpStream;

// ============================================================
// CONSTANTS
// ============================================================

// SOP Class UID for the Verification service (used by C-ECHO)
// This UID is defined by the DICOM standard and never changes
const VERIFICATION_SOP_CLASS: &str = "1.2.840.10008.1.1";

// Transfer Syntax: Explicit VR Little Endian
// This is the most common encoding in modern DICOM implementations
// "Explicit" means each field declares its own type (VR)
// "Little Endian" means least significant byte comes first
const EXPLICIT_VR_LE: &str = "1.2.840.10008.1.2.1";

// ============================================================
// BUILD THE C-ECHO-RQ COMMAND SET
// ============================================================
//
// A C-ECHO-RQ is a DIMSE command (DICOM Message Service Element).
// It is the DICOM equivalent of a network "ping".
//
// The command set is a series of DICOM data elements, each made of:
//   - 4 bytes : tag (group + element, each 2 bytes, little endian)
//   - 4 bytes : value length
//   - N bytes : value
//
// The command set is always encoded in Implicit VR Little Endian,
// regardless of the negotiated Transfer Syntax.
//
// Required elements for C-ECHO-RQ:
//   (0000,0000) Command Group Length   → total byte length of the other elements
//   (0000,0002) Affected SOP Class UID → identifies the service (Verification)
//   (0000,0100) Command Field          → 0x0030 = C-ECHO-RQ
//   (0000,0110) Message ID             → unique ID for this request
//   (0000,0800) Command Data Set Type  → 0x0101 = no dataset follows
//
fn build_c_echo_rq(message_id: u16) -> Vec<u8> {
    // DICOM requires all string values to have an even byte length.
    // "1.2.840.10008.1.1" is 17 bytes (odd), so we pad with a null byte.
    let sop_uid = b"1.2.840.10008.1.1\0"; // 18 bytes total (even)

    // Calculate the Command Group Length.
    // It is the sum of all elements AFTER (0000,0000), where each element
    // costs: 4 bytes (tag) + 4 bytes (length field) + N bytes (value)
    let group_length: u32 = (4 + 4 + sop_uid.len()) as u32 +  // (0000,0002) SOP Class UID  : 18 bytes value
        (4 + 4 + 2) +                      // (0000,0100) Command Field  :  2 bytes value
        (4 + 4 + 2) +                      // (0000,0110) Message ID     :  2 bytes value
        (4 + 4 + 2); // (0000,0800) Data Set Type  :  2 bytes value

    // We build the raw bytes of the command set manually.
    // Vec<u8> is a growable array of bytes in Rust.
    let mut data: Vec<u8> = Vec::new();

    // --- Element (0000,0000) : Command Group Length ---
    // Tag bytes: group=0x0000 → [0x00, 0x00], element=0x0000 → [0x00, 0x00]
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
    // Value length: always 4 bytes for a UL (unsigned long)
    data.extend_from_slice(&4u32.to_le_bytes());
    // Value: the group length we calculated above
    data.extend_from_slice(&group_length.to_le_bytes());

    // --- Element (0000,0002) : Affected SOP Class UID ---
    // Tag: group=0x0000 → [0x00, 0x00], element=0x0002 → [0x02, 0x00]
    data.extend_from_slice(&[0x00, 0x00, 0x02, 0x00]);
    data.extend_from_slice(&(sop_uid.len() as u32).to_le_bytes());
    data.extend_from_slice(sop_uid);

    // --- Element (0000,0100) : Command Field ---
    // Tag: group=0x0000 → [0x00, 0x00], element=0x0100 → [0x00, 0x01]
    // Value: 0x0030 = C-ECHO-RQ (defined by DICOM standard)
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    data.extend_from_slice(&2u32.to_le_bytes());
    data.extend_from_slice(&0x0030u16.to_le_bytes());

    // --- Element (0000,0110) : Message ID ---
    // Tag: group=0x0000, element=0x0110 → [0x10, 0x01]
    // Value: the message_id passed by the caller (starts at 1)
    data.extend_from_slice(&[0x00, 0x00, 0x10, 0x01]);
    data.extend_from_slice(&2u32.to_le_bytes());
    data.extend_from_slice(&message_id.to_le_bytes());

    // --- Element (0000,0800) : Command Data Set Type ---
    // Tag: group=0x0000, element=0x0800 → [0x00, 0x08]
    // Value: 0x0101 = no Data Set follows (C-ECHO carries no payload)
    data.extend_from_slice(&[0x00, 0x00, 0x00, 0x08]);
    data.extend_from_slice(&2u32.to_le_bytes());
    data.extend_from_slice(&0x0101u16.to_le_bytes());

    data
}

// ============================================================
// SEND A C-ECHO REQUEST
// ============================================================
//
// This function implements the full C-ECHO exchange:
//
//   SCU (us)                        SCP (Orthanc)
//     |                                  |
//     |--- A-ASSOCIATE-RQ -------------> |  "I want to connect"
//     | <-- A-ASSOCIATE-AC ------------ |  "Accepted"
//     |                                  |
//     |--- P-DATA-TF (C-ECHO-RQ) ------> |  "Ping?"
//     | <-- P-DATA-TF (C-ECHO-RSP) ----- |  "Pong! (Status=0x0000)"
//     |                                  |
//     |--- A-RELEASE-RQ --------------> |  "I'm done"
//     | <-- A-RELEASE-RP -------------- |  "Goodbye"
//
// The function returns Ok(()) on success, or an error if anything fails.
// Box<dyn std::error::Error> means "any kind of error" — convenient when
// combining errors from multiple crates (network, DICOM, IO...).
//
pub fn send_echo(
    host: &str,
    port: u16,
    calling_aet: &str,
    called_aet: &str,
    count: u32, // number of C-ECHO requests to send
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("{}:{}", host, port);

    for i in 1..=count {
        if count > 1 {
            println!("[C-ECHO] Request #{}", i);
        }
        println!("[C-ECHO] Connecting to {}...", addr);

        // Each C-ECHO opens its own association — this is the standard
        // DICOM behaviour. One association per request.
        let ts = EXPLICIT_VR_LE.to_string();

        let options = ClientAssociationOptions::new()
            .calling_ae_title(calling_aet)
            .called_ae_title(called_aet)
            .with_presentation_context(VERIFICATION_SOP_CLASS, vec![&ts]);

        let mut association = options.establish(&addr)?;
        println!("[C-ECHO] DICOM association established ✓");

        let pc_id = association.presentation_contexts()[0].id;
        let cmd_bytes = build_c_echo_rq(i as u16); // use loop index as message ID

        association.send(&Pdu::PData {
            data: vec![PDataValue {
                presentation_context_id: pc_id,
                value_type: PDataValueType::Command,
                is_last: true,
                data: cmd_bytes,
            }],
        })?;
        println!("[C-ECHO] C-ECHO-RQ sent");

        match association.receive()? {
            Pdu::PData { data } => {
                println!(
                    "[C-ECHO] Response received ({} bytes) ✓",
                    data[0].data.len()
                );
            }
            pdu => {
                println!("[C-ECHO] Unexpected PDU received: {:?}", pdu);
            }
        }

        association.release()?;
        println!("[C-ECHO] Association released ✓");
    }

    Ok(())
}
