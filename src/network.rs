use socket2::{Domain, Protocol, Socket, Type};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::str::FromStr;
use std::time::Duration;

// ============================================================
// NETWORK CONFIGURATION
// ============================================================
//
// Holds all network-level options that go beyond what dicom-ul
// provides natively: interface binding and TTL control.
//
pub struct NetworkConfig {
    pub local_ip: Option<String>,
    pub ttl: u8,
    // Reserved for future use when dicom-ul supports connection timeouts
    #[allow(dead_code)]
    pub timeout_secs: u64,
}

impl NetworkConfig {
    pub fn new(local_ip: Option<String>, ttl: u8) -> Self {
        Self {
            local_ip,
            ttl,
            timeout_secs: 10,
        }
    }
}

// ============================================================
// CREATE A TCP STREAM BOUND TO A SPECIFIC LOCAL INTERFACE
// ============================================================
//
// By default, the OS chooses which network interface to use.
// This function forces the connection to go through a specific
// local IP address — useful when the machine has multiple
// network interfaces (eth0, eth1, VPN, etc.)
//
// Steps:
//   1. Create a raw TCP socket with socket2
//   2. Set SO_REUSEADDR (allows reuse of local address)
//   3. Optionally bind to a specific local IP (port 0 = OS picks the port)
//   4. Set the TTL on the socket
//   5. Connect to the remote address
//   6. Convert to std::net::TcpStream for use with dicom-ul
//
#[allow(dead_code)]
pub fn create_tcp_stream(
    remote_host: &str,
    remote_port: u16,
    config: &NetworkConfig,
) -> Result<TcpStream, Box<dyn std::error::Error>> {
    let remote_addr: SocketAddr = format!("{}:{}", remote_host, remote_port).parse()?;

    // Create a raw TCP socket — Domain::IPV4 for IPv4
    let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))?;

    // Allow reuse of local address — avoids "address already in use" errors
    socket.set_reuse_address(true)?;

    // Set TTL on the TCP socket
    socket.set_ttl(config.ttl as u32)?;
    println!("[NETWORK] TTL set to {}", config.ttl);

    // Optionally bind to a specific local network interface
    if let Some(ref local_ip) = config.local_ip {
        let ip: IpAddr = IpAddr::from_str(local_ip)?;
        // Port 0 means the OS will assign an available ephemeral port
        let local_addr = SocketAddr::new(ip, 0);
        socket.bind(&local_addr.into())?;
        println!("[NETWORK] Bound to local interface {}", local_ip);
    }

    // Set a connection timeout
    let timeout = Duration::from_secs(config.timeout_secs);
    socket.connect_timeout(&remote_addr.into(), timeout)?;
    println!("[NETWORK] TCP connected to {}:{}", remote_host, remote_port);

    // Convert socket2::Socket into std::net::TcpStream
    // dicom-ul works with std::net::TcpStream
    Ok(TcpStream::from(socket))
}
// ============================================================
// SEND RAW UDP PACKETS WITH CONTROLLED TTL
// ============================================================
//
// UDP mode does NOT implement the DICOM protocol — there is no
// association, no handshake, no response expected.
//
// The purpose is to test the network infrastructure itself:
//   - Does the packet reach the destination?
//   - Does the TTL expire before reaching the target?
//   - Are there firewalls or routers dropping the packets?
//
// We send a minimal DICOM-like payload (a fake A-ASSOCIATE-RQ
// header) so that network monitoring tools (Wireshark, etc.)
// can identify the traffic as DICOM-related.
//
// TTL behaviour:
//   TTL=1   → packet dies at the first router (local network only)
//   TTL=64  → standard Linux default, reaches most internet hosts
//   TTL=255 → maximum, crosses any number of hops
//
pub fn send_udp_packets(
    remote_host: &str,
    remote_port: u16,
    config: &NetworkConfig,
    count: u32,       // number of packets to send
    interval_ms: u64, // delay between packets in milliseconds
) -> Result<(), Box<dyn std::error::Error>> {
    let remote_addr: SocketAddr = format!("{}:{}", remote_host, remote_port).parse()?;

    // Create a raw UDP socket
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;

    // Set the TTL — this controls how many router hops the packet survives
    socket.set_ttl(config.ttl as u32)?;
    println!("[UDP] TTL set to {}", config.ttl);

    // Optionally bind to a specific local interface
    if let Some(ref local_ip) = config.local_ip {
        let ip: IpAddr = IpAddr::from_str(local_ip)?;
        let local_addr = SocketAddr::new(ip, 0);
        socket.bind(&local_addr.into())?;
        println!("[UDP] Bound to local interface {}", local_ip);
    }

    // Convert to std::net::UdpSocket for easier send operations
    let udp: std::net::UdpSocket = socket.into();

    println!(
        "[UDP] Sending {} packet(s) to {}:{} every {}ms",
        count, remote_host, remote_port, interval_ms
    );

    for i in 1..=count {
        let payload = build_fake_dicom_payload(i);
        udp.send_to(&payload, remote_addr)?;
        println!("[UDP] Packet #{} sent ({} bytes)", i, payload.len());

        if i < count {
            std::thread::sleep(Duration::from_millis(interval_ms));
        }
    }

    println!("[UDP] Done");
    Ok(())
}

// ============================================================
// BUILD A FAKE DICOM-LIKE UDP PAYLOAD
// ============================================================
//
// This is NOT a valid DICOM PDU — UDP has no association.
// We just build a recognizable header so Wireshark and other
// tools can flag this traffic as DICOM-related.
//
// Structure:
//   Byte 0    : PDU type (0x01 = A-ASSOCIATE-RQ)
//   Byte 1    : reserved (0x00)
//   Bytes 2-5 : payload length (big endian, as per DICOM spec)
//   Bytes 6+  : ASCII marker "DICOM-GEN" + packet number
//
fn build_fake_dicom_payload(packet_number: u32) -> Vec<u8> {
    let marker = format!("DICOM-GEN packet #{}", packet_number);
    let marker_bytes = marker.as_bytes();

    let mut payload: Vec<u8> = Vec::new();

    // PDU type 0x01 = A-ASSOCIATE-RQ (fake, just for identification)
    payload.push(0x01);
    payload.push(0x00); // reserved

    // Length in big endian (DICOM uses big endian for PDU length)
    let len = marker_bytes.len() as u32;
    payload.extend_from_slice(&len.to_be_bytes());

    // Payload content
    payload.extend_from_slice(marker_bytes);

    payload
}
