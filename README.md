# dicom-gen

A DICOM traffic generator written in Rust, designed for debugging and 
developing DICOM applications.

## Features (planned)

- C-ECHO — DICOM ping to verify connectivity
- C-STORE — Send DICOM images to a remote SCP
- C-FIND — Query a DICOM server
- C-MOVE — Trigger image transfers between SCPs
- Dual mode: TCP (full DICOM handshake) or UDP (raw, TTL-controlled)
- CLI interface with configurable AET, host, port, TTL

## Target platform

Debian Linux

## Status

🚧 Work in progress — early development stage

## Requirements

- Rust (installed via rustup)
- Orthanc (optional, for local testing)

## Usage

_Coming soon_

## License

MIT