//! CRC used by the MoYoung JieLi watch-face transfer protocol.
//!
//! Width 16, polynomial 0x1021, init 0xFEEA, no reflection or final XOR.

pub fn crc16(data: &[u8]) -> u16 {
    let mut crc = 0xFEEAu16;
    for &byte in data {
        crc ^= u16::from(byte) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}

/// Extract a CRC only from the observed JieLi completion-frame format.
/// Other response formats are left to the existing uploader.
pub fn reported_crc(frame: &[u8]) -> Option<u16> {
    match frame {
        [0xfe, 0xea, 0x20, 0x09, 0x74, 0xff, 0xff, hi, lo] => {
            Some(u16::from_be_bytes([*hi, *lo]))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_vectors() {
        assert_eq!(crc16(b""), 0xFEEA);
        assert_eq!(crc16(b"123456789"), 0xB798);
        let bytes: Vec<u8> = (0..=255).collect();
        assert_eq!(crc16(&bytes), 0x4D93);
        assert_eq!(crc16(&[0xfe, 0xea]), 0x0000);
    }

    #[test]
    fn completion_frame() {
        assert_eq!(reported_crc(&[0xfe, 0xea, 0x20, 9, 0x74, 0xff, 0xff, 0xe6, 0xf2]), Some(0xE6F2));
        assert_eq!(reported_crc(&[0xfe, 0xea, 0x20, 9, 0x74, 0xff, 0xff, 0, 0]), Some(0));
        assert_eq!(reported_crc(&[0xfe, 0xea, 0x20, 9, 0x74, 0, 0, 0, 0]), None);
        assert_eq!(reported_crc(&[0xfe, 0xea, 0x20, 7, 0x74, 0, 0]), None);
        assert_eq!(reported_crc(&[]), None);
    }
}
