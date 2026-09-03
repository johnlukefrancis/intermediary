// Path: src-tauri/src/lib/agent/supervisor/websocket_frame.rs
// Description: Minimal RFC 6455 client framing used by the supervisor's graceful-shutdown request

//! The supervisor speaks exactly one request/response exchange to an agent, on
//! a socket it opens and closes itself. That is far less than a websocket
//! library provides, and the app deliberately carries no websocket dependency
//! (the identity probe next door is raw HTTP too), so the two frame shapes it
//! needs live here: a masked client text frame out, and server frames in.

use std::io::Read;

pub(super) const OPCODE_CONTINUATION: u8 = 0x0;
pub(super) const OPCODE_TEXT: u8 = 0x1;
pub(super) const OPCODE_CLOSE: u8 = 0x8;
pub(super) const OPCODE_PING: u8 = 0x9;

const FIN_BIT: u8 = 0x80;
const MASK_BIT: u8 = 0x80;
const LENGTH_MASK: u8 = 0x7f;
const LENGTH_16BIT: u8 = 126;
const LENGTH_64BIT: u8 = 127;
/// One inbound frame bound. The agent's replies are small; a larger frame is a
/// protocol surprise, not something to buffer.
const MAX_FRAME_PAYLOAD: u64 = 1024 * 1024;

pub(super) struct Frame {
    pub fin: bool,
    pub opcode: u8,
    pub payload: Vec<u8>,
}

/// Encodes one complete masked text frame. Every client frame must be masked,
/// and the mask must not be constant across frames, so it is derived from the
/// caller's nonce rather than hard-coded.
pub(super) fn encode_client_text_frame(payload: &str, mask_nonce: u32) -> Vec<u8> {
    let bytes = payload.as_bytes();
    let mask = mask_nonce.to_be_bytes();
    let mut frame = Vec::with_capacity(bytes.len() + 14);
    frame.push(FIN_BIT | OPCODE_TEXT);

    let length = bytes.len();
    if length < LENGTH_16BIT as usize {
        frame.push(MASK_BIT | (length as u8));
    } else if let Ok(short) = u16::try_from(length) {
        frame.push(MASK_BIT | LENGTH_16BIT);
        frame.extend_from_slice(&short.to_be_bytes());
    } else {
        frame.push(MASK_BIT | LENGTH_64BIT);
        frame.extend_from_slice(&(length as u64).to_be_bytes());
    }

    frame.extend_from_slice(&mask);
    frame.extend(
        bytes
            .iter()
            .enumerate()
            .map(|(index, byte)| byte ^ mask[index % 4]),
    );
    frame
}

/// Reads one frame. A server frame is normally unmasked; a masked one is
/// unmasked anyway rather than rejected, because the exchange's value is the
/// payload, not policing the peer.
pub(super) fn read_frame(reader: &mut impl Read) -> Result<Frame, String> {
    let mut header = [0_u8; 2];
    read_exact(reader, &mut header)?;
    let fin = header[0] & FIN_BIT != 0;
    let opcode = header[0] & 0x0f;
    let masked = header[1] & MASK_BIT != 0;

    let length = match header[1] & LENGTH_MASK {
        LENGTH_16BIT => {
            let mut extended = [0_u8; 2];
            read_exact(reader, &mut extended)?;
            u64::from(u16::from_be_bytes(extended))
        }
        LENGTH_64BIT => {
            let mut extended = [0_u8; 8];
            read_exact(reader, &mut extended)?;
            u64::from_be_bytes(extended)
        }
        short => u64::from(short),
    };
    if length > MAX_FRAME_PAYLOAD {
        return Err(format!(
            "Agent websocket frame of {length} bytes exceeds the read bound"
        ));
    }

    let mut mask = [0_u8; 4];
    if masked {
        read_exact(reader, &mut mask)?;
    }

    let mut payload = vec![0_u8; usize::try_from(length).unwrap_or(0)];
    read_exact(reader, &mut payload)?;
    if masked {
        for (index, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[index % 4];
        }
    }

    Ok(Frame {
        fin,
        opcode,
        payload,
    })
}

fn read_exact(reader: &mut impl Read, buffer: &mut [u8]) -> Result<(), String> {
    reader
        .read_exact(buffer)
        .map_err(|err| format!("Agent websocket read failed: {err}"))
}

#[cfg(test)]
mod tests {
    use super::{encode_client_text_frame, read_frame, OPCODE_TEXT};

    #[test]
    fn a_short_client_frame_is_final_masked_and_recoverable() {
        let frame = encode_client_text_frame("hi", 0x01020304);
        assert_eq!(frame[0], 0x81);
        assert_eq!(frame[1], 0x82);
        assert_eq!(&frame[2..6], &[0x01, 0x02, 0x03, 0x04]);

        let unmasked: Vec<u8> = frame[6..]
            .iter()
            .enumerate()
            .map(|(index, byte)| byte ^ frame[2 + index % 4])
            .collect();
        assert_eq!(unmasked, b"hi");
    }

    #[test]
    fn a_long_client_frame_uses_the_extended_length() {
        let payload = "x".repeat(200);
        let frame = encode_client_text_frame(&payload, 7);
        assert_eq!(frame[1], 0xfe);
        assert_eq!(u16::from_be_bytes([frame[2], frame[3]]), 200);
        assert_eq!(frame.len(), 4 + 4 + 200);
    }

    #[test]
    fn a_server_text_frame_is_read_back_whole() {
        let mut bytes = vec![0x81, 0x03];
        bytes.extend_from_slice(b"abc");
        let mut cursor = bytes.as_slice();
        let frame = read_frame(&mut cursor).expect("frame");
        assert!(frame.fin);
        assert_eq!(frame.opcode, OPCODE_TEXT);
        assert_eq!(frame.payload, b"abc");
    }

    #[test]
    fn a_masked_extended_server_frame_is_unmasked() {
        let payload = vec![b'z'; 300];
        let mask = [0x0a, 0x0b, 0x0c, 0x0d];
        let mut bytes = vec![0x81, 0xfe, 0x01, 0x2c];
        bytes.extend_from_slice(&mask);
        bytes.extend(
            payload
                .iter()
                .enumerate()
                .map(|(index, byte)| byte ^ mask[index % 4]),
        );
        let mut cursor = bytes.as_slice();
        let frame = read_frame(&mut cursor).expect("frame");
        assert_eq!(frame.payload, payload);
    }
}
