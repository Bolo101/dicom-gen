# dicom-gen

A DICOM traffic generator written in Rust, designed for debugging and developing DICOM applications.

Supports **C-ECHO** (ping) and **C-STORE** (image transfer) over TCP, and raw **UDP** packet generation with TTL control.


## Features

- **C-ECHO** — DICOM ping to verify connectivity with a remote SCP
- **C-STORE** — Send DICOM images to a remote SCP (supports all Transfer Syntaxes)
- **UDP mode** — Raw packet generation with configurable TTL (no DICOM handshake)
- **--count** — Send multiple requests in sequence
- **--interval** — Configurable delay between requests
- **--inspect** — Inspect a local DICOM file and print its metadata
- Configurable AET, host, port, TTL via CLI

## Requirements

- Rust (installed via rustup)
- Debian Linux (tested on Debian 12)
- Orthanc (optional, for local testing)

## Installation

### 1 — Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

Verify:
```bash
rustc --version
cargo --version
```

### 2 — Clone and build

```bash
git clone https://github.com/Bolo101/dicom-gen.git
cd dicom-gen
cargo build --release
```

The binary will be available at `target/release/dicom-gen`.

### 3 — Install Orthanc (optional, for local testing)

Orthanc is an open-source DICOM server (SCP) used to test C-ECHO and C-STORE.
You can install Orthanc on a remote host, or directly on dicom-gen running machine to work on localhost 

```bash
sudo apt update
sudo apt install orthanc
```

#### Start and enable Orthanc

```bash
# Start the service
sudo systemctl start orthanc

# Enable at boot
sudo systemctl enable orthanc

# Check status
sudo systemctl status orthanc
```

#### Verify Orthanc is running

```bash
curl http://localhost:8042/system
```

You should see a JSON response with the Orthanc version.

- Web interface : `http://localhost:8042`
- DICOM port    : `4242`
- Default AET   : `ORTHANC`

#### Configure Orthanc to accept any SCP

By default Orthanc may reject unknown AETs. To allow all incoming connections:

```bash
sudo nano /etc/orthanc/orthanc.json
```

Make sure these values are set:

```json
"DicomAet" : "ORTHANC",
"DicomPort" : 4242,
"UnknownSopClassAccepted" : true
```

Then restart:

```bash
sudo systemctl restart orthanc
```

#### Test the connection with dicom-gen

```bash
# Verify connectivity
./dicom-gen --mode tcp --command echo --host 127.0.0.1 --port 4242

# Send a test image
./dicom-gen --mode tcp --command store \
  --host 127.0.0.1 --port 4242 \
  --file /path/to/image.dcm
```

The image should appear in the Orthanc web interface at `http://localhost:8042`.

## Usage

### Inspect a DICOM file

```bash
./dicom-gen --inspect --file /path/to/image.dcm
```

Output:
```
=== Inspection du fichier DICOM ===
Patient      : CompressedSamples^CT1
Patient ID   : 1CT1
Date étude   : 20040119
SOP Class    : 1.2.840.10008.5.1.4.1.1.2
Modalité     : CT
```

---

### C-ECHO — DICOM ping

```bash
# Single echo to localhost
./dicom-gen --mode tcp --command echo --host 127.0.0.1 --port 4242

# Single echo to a remote host
./dicom-gen --mode tcp --command echo --host 192.168.1.50 --port 4242

# 5 echos with 1 second interval
./dicom-gen --mode tcp --command echo --host 192.168.1.50 \
  --count 5 --interval 1000

# Custom AET names
./dicom-gen --mode tcp --command echo --host 192.168.1.50 \
  --calling-aet MY-SCU --called-aet MY-SCP
```

---

### C-STORE — Send a DICOM image

```bash
# Send a single image to localhost
./dicom-gen --mode tcp --command store \
  --host 127.0.0.1 --port 4242 \
  --file /path/to/image.dcm

# Send a single image to a remote host
./dicom-gen --mode tcp --command store \
  --host 192.168.1.50 --port 4242 \
  --file /path/to/image.dcm

# Custom AET names
./dicom-gen --mode tcp --command store \
  --host 192.168.1.50 --port 4242 \
  --file /path/to/image.dcm \
  --calling-aet MY-SCU --called-aet MY-SCP
```

---

### UDP — Raw packet generation with TTL control

UDP mode does **not** implement the DICOM protocol — it sends raw packets
without a handshake or response. Useful for testing network infrastructure
(firewalls, routing, QoS) rather than DICOM applications.

```bash
# Send 1 UDP packet with default TTL (64)
./dicom-gen --mode udp --host 192.168.1.50 --port 4242

# Send 10 packets with TTL=5 (dies after 5 router hops)
./dicom-gen --mode udp --host 192.168.1.50 --port 4242 \
  --count 10 --ttl 5

# Send 5 packets with 500ms interval between each
./dicom-gen --mode udp --host 192.168.1.50 --port 4242 \
  --count 5 --interval 500 --ttl 64

# TTL=1 : packet dies at the first router (local network only)
./dicom-gen --mode udp --host 192.168.1.50 --ttl 1

# TTL=255 : maximum, crosses any number of hops
./dicom-gen --mode udp --host 192.168.1.50 --ttl 255
```

---

### All CLI options

```
Usage: dicom-gen [OPTIONS]

Options:
      --mode <MODE>                Transport mode: tcp or udp [default: tcp]
      --host <HOST>                Target host (IP or hostname) [default: 127.0.0.1]
      --port <PORT>                Target DICOM port [default: 4242]
      --command <COMMAND>          DICOM command: echo or store [default: echo]
      --called-aet <CALLED_AET>    Called AET - the server's DICOM name [default: ORTHANC]
      --calling-aet <CALLING_AET>  Calling AET - our DICOM name [default: DICOM-GEN]
      --ttl <TTL>                  TTL for UDP mode [default: 64]
      --file <FILE>                Path to a DICOM file (--inspect or --command store)
      --inspect                    Inspect a DICOM file and print its metadata
      --local-ip <LOCAL_IP>        Local IP address to bind to (reserved, not yet active)
      --count <COUNT>              Number of requests/packets to send [default: 1]
      --interval <INTERVAL>        Delay between requests in milliseconds [default: 1000]
  -h, --help                       Print help
  -V, --version                    Print version
```

## Project structure

```
src/
├── main.rs       — entry point, CLI dispatch
├── cli.rs        — CLI argument definitions (clap)
├── echo.rs       — C-ECHO implementation
├── store.rs      — C-STORE implementation
├── inspect.rs    — DICOM file inspection
└── network.rs    — UDP packet generation, socket configuration
```

## Known limitations

- `--local-ip` interface binding is not yet active for TCP mode
  (waiting for dicom-ul to support pre-bound TcpStream injection)
- C-FIND and C-MOVE are not implemented as out-of-scope
