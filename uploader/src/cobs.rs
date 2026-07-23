pub fn encode(data: &[u8], delimiter: u8) -> Vec<u8> {
    let mut out = vec![0];

    let mut codep = 0;
    let mut code = 1;

    for (i, &byte) in data.iter().enumerate() {
        if byte != delimiter {
            out.push(byte);
            code += 1;
        }

        if byte == delimiter || code == 0xFF {
            out[codep] = code;
            code = 1;
            codep = out.len();

            if byte == delimiter || i != data.len() - 1 {
                out.push(0);
            }
        }
    }

    if codep < out.len() {
        out[codep] = code;
    }
    out
}

pub fn decode(buffer: &[u8], delimiter: u8) -> Vec<u8> {
    let mut data = vec![];
    let mut i = 0;

    while i < buffer.len() {
        let code = buffer[i];
        i += 1;

        if code == 0 {
            break;
        }

        let copy_end = i + (code as usize).saturating_sub(1);
        let copy_end = usize::min(copy_end, buffer.len());
        data.extend_from_slice(&buffer[i..copy_end]);
        i = copy_end;

        if code < 255 && i < buffer.len() {
            data.push(delimiter);
        }
    }

    data
}

#[cfg(test)]
mod tests {
    use super::*;

    // tests were copied from these links:
    // - https://github.com/charlesnicholson/nanocobs/blob/main/tests/test_cobs_encode.cc
    // - https://github.com/charlesnicholson/nanocobs/blob/main/tests/test_cobs_decode.cc

    #[test]
    fn encode_empty() {
        assert_eq!(encode(&[], 0), vec![0x01]);
    }

    #[test]
    fn encode_1_nonzero() {
        assert_eq!(encode(&[0x34], 0), vec![0x02, 0x34]);
    }

    #[test]
    fn encode_2_nonzero() {
        assert_eq!(encode(&[0x34, 0x56], 0), vec![0x03, 0x34, 0x56]);
    }

    #[test]
    fn encode_8_nonzero() {
        assert_eq!(
            encode(&[0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xFF], 0),
            vec![0x09, 0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xFF]
        );
    }

    #[test]
    fn encode_1_zero() {
        assert_eq!(encode(&[0x00], 0), vec![0x01, 0x01]);
    }

    #[test]
    fn encode_2_zero() {
        assert_eq!(encode(&[0x00, 0x00], 0), vec![0x01, 0x01, 0x01]);
    }

    #[test]
    fn encode_8_zero() {
        assert_eq!(encode(&[0x00; 8], 0), vec![0x01; 9]);
    }

    #[test]
    fn encode_4_alternating_zero_nonzero() {
        assert_eq!(
            encode(&[0x00, 0x11, 0x00, 0x22], 0),
            vec![0x01, 0x02, 0x11, 0x02, 0x22]
        );
    }

    #[test]
    fn encode_4_alternating_nonzero_zero() {
        assert_eq!(
            encode(&[0x11, 0x00, 0x22, 0x00], 0),
            vec![0x02, 0x11, 0x02, 0x22, 0x01]
        );
    }

    #[test]
    fn encode_253_nonzero() {
        let enc = encode(&[0x42; 253], 0);
        assert_eq!(enc[0], 0xFE);
        assert_eq!(enc.len(), 254);
    }

    #[test]
    fn encode_254_nonzero_single_0xff_block() {
        let mut expected = vec![0xFF];
        expected.extend(std::iter::repeat_n(0x01, 254));
        assert_eq!(encode(&[0x01; 254], 0), expected);
    }

    #[test]
    fn encode_255_nonzero_two_code_blocks() {
        let mut expected = vec![0xFF];
        expected.extend(std::iter::repeat_n(0x01, 254));
        expected.push(0x02);
        expected.push(0x01);
        assert_eq!(encode(&[0x01; 255], 0), expected);
    }

    #[test]
    fn encode_256_nonzero() {
        let enc = encode(&[0x01; 256], 0);
        assert_eq!(enc[0], 0xFF);
    }

    #[test]
    fn encode_508_nonzero_two_full_0xff_blocks() {
        let mut expected = vec![0xFF];
        expected.extend(std::iter::repeat_n(0xAA, 254));
        expected.push(0xFF);
        expected.extend(std::iter::repeat_n(0xAA, 254));
        assert_eq!(encode(&[0xAA; 508], 0), expected);
    }

    // --- Wikipedia examples ---

    #[test]
    fn encode_wikipedia_example_1() {
        assert_eq!(encode(&[0x00], 0), vec![0x01, 0x01]);
    }

    #[test]
    fn encode_wikipedia_example_2() {
        assert_eq!(encode(&[0x00, 0x00], 0), vec![0x01, 0x01, 0x01]);
    }

    #[test]
    fn encode_wikipedia_example_3() {
        assert_eq!(encode(&[0x00, 0x11, 0x00], 0), vec![0x01, 0x02, 0x11, 0x01]);
    }

    #[test]
    fn encode_wikipedia_example_4() {
        assert_eq!(
            encode(&[0x11, 0x22, 0x00, 0x33], 0),
            vec![0x03, 0x11, 0x22, 0x02, 0x33]
        );
    }

    #[test]
    fn encode_wikipedia_example_5() {
        assert_eq!(
            encode(&[0x11, 0x22, 0x33, 0x44], 0),
            vec![0x05, 0x11, 0x22, 0x33, 0x44]
        );
    }

    #[test]
    fn encode_wikipedia_example_6() {
        assert_eq!(
            encode(&[0x11, 0x00, 0x00, 0x00], 0),
            vec![0x02, 0x11, 0x01, 0x01, 0x01]
        );
    }

    #[test]
    fn encode_wikipedia_example_7() {
        let dec: Vec<u8> = (1..=254).collect();
        let mut expected = vec![0xFF];
        expected.extend(1..=254);
        assert_eq!(encode(&dec, 0), expected);
    }

    #[test]
    fn encode_wikipedia_example_8() {
        let dec: Vec<u8> = (0..=254).collect();
        let mut expected = vec![0x01, 0xFF];
        expected.extend(1..=254);
        assert_eq!(encode(&dec, 0), expected);
    }

    #[test]
    fn encode_wikipedia_example_9() {
        let dec: Vec<u8> = (1..=255).collect();
        let mut expected = vec![0xFF];
        expected.extend(1..=254);
        expected.push(0x02);
        expected.push(0xFF);
        assert_eq!(encode(&dec, 0), expected);
    }

    // --- COBS paper: Figure 3 IP header ---

    #[test]
    fn encode_cobs_paper_figure_3() {
        let dec = vec![
            0x45, 0x00, 0x00, 0x2C, 0x4C, 0x79, 0x00, 0x00, 0x40, 0x06, 0x4F, 0x37,
        ];
        let expected = vec![
            0x02, 0x45, 0x01, 0x04, 0x2C, 0x4C, 0x79, 0x01, 0x05, 0x40, 0x06, 0x4F, 0x37,
        ];
        assert_eq!(encode(&dec, 0), expected);
    }

    // --- Longer payload encodings ---

    #[test]
    fn encode_255_zero_bytes() {
        assert_eq!(encode(&[0x00; 255], 0), vec![0x01; 256]);
    }

    #[test]
    fn encode_1024_nonzero_bytes() {
        let mut expected: Vec<u8> = Vec::new();
        for _ in 0..1024 / 254 {
            expected.push(0xFF);
            expected.extend(std::iter::repeat_n(b'!', 254));
        }
        expected.push((1024 % 254 + 1) as u8);
        expected.extend(std::iter::repeat_n(b'!', 1024 % 254));
        assert_eq!(encode(&[b'!'; 1024], 0), expected);
    }

    #[test]
    fn encode_1024_zero_bytes() {
        assert_eq!(encode(&[0x00; 1024], 0), vec![0x01; 1025]);
    }

    #[test]
    fn encode_1024_every_other_zero() {
        let dec: Vec<u8> = (0..1024).map(|i| i as u8 & 1).collect();
        let expected: Vec<u8> = (0..=1024).map(|i| if i & 1 == 1 { 2 } else { 1 }).collect();
        assert_eq!(encode(&dec, 0), expected);
    }

    #[test]
    fn encode_single_byte_values() {
        for b in 0..=0xFFu8 {
            let enc = encode(&[b], 0);
            assert!(
                enc.iter().all(|&x| x != 0),
                "zero found in encoding of byte {:#04x}",
                b
            );
        }
    }

    // --- Decode: known vectors (from nanocobs decode tests) ---

    #[test]
    fn decode_empty() {
        assert_eq!(decode(&[0x01], 0), vec![]);
    }

    #[test]
    fn decode_1_nonzero() {
        assert_eq!(decode(&[0x02, 0x34], 0), vec![0x34]);
    }

    #[test]
    fn decode_2_nonzero() {
        assert_eq!(decode(&[0x03, 0x34, 0x56], 0), vec![0x34, 0x56]);
    }

    #[test]
    fn decode_8_nonzero() {
        assert_eq!(
            decode(&[0x09, 0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xFF], 0),
            vec![0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xFF]
        );
    }

    #[test]
    fn decode_1_zero() {
        assert_eq!(decode(&[0x01, 0x01], 0), vec![0x00]);
    }

    #[test]
    fn decode_2_zero() {
        assert_eq!(decode(&[0x01, 0x01, 0x01], 0), vec![0x00, 0x00]);
    }

    #[test]
    fn decode_8_zero() {
        assert_eq!(decode(&[0x01; 9], 0), vec![0x00; 8]);
    }

    #[test]
    fn decode_4_alternating_zero_nonzero() {
        assert_eq!(
            decode(&[0x01, 0x02, 0x11, 0x02, 0x22], 0),
            vec![0x00, 0x11, 0x00, 0x22]
        );
    }

    #[test]
    fn decode_4_alternating_nonzero_zero() {
        assert_eq!(
            decode(&[0x02, 0x11, 0x02, 0x22, 0x01], 0),
            vec![0x11, 0x00, 0x22, 0x00]
        );
    }

    #[test]
    fn decode_254_nonzero_single_0xff_block() {
        let mut enc = vec![0xFF];
        enc.extend(std::iter::repeat_n(0x01, 254));
        assert_eq!(decode(&enc, 0), vec![0x01; 254]);
    }

    #[test]
    fn decode_255_nonzero_two_code_blocks() {
        let mut enc = vec![0xFF];
        enc.extend(std::iter::repeat_n(0x01, 254));
        enc.push(0x02);
        enc.push(0x01);
        assert_eq!(decode(&enc, 0), vec![0x01; 255]);
    }

    #[test]
    fn decode_508_nonzero_two_full_0xff_blocks() {
        let mut enc = vec![0xFF];
        enc.extend(std::iter::repeat_n(0xAA, 254));
        enc.push(0xFF);
        enc.extend(std::iter::repeat_n(0xAA, 254));
        assert_eq!(decode(&enc, 0), vec![0xAA; 508]);
    }

    #[test]
    fn decode_cobs_paper_figure_3() {
        let enc = vec![
            0x02, 0x45, 0x01, 0x04, 0x2C, 0x4C, 0x79, 0x01, 0x05, 0x40, 0x06, 0x4F, 0x37,
        ];
        let expected = vec![
            0x45, 0x00, 0x00, 0x2C, 0x4C, 0x79, 0x00, 0x00, 0x40, 0x06, 0x4F, 0x37,
        ];
        assert_eq!(decode(&enc, 0), expected);
    }

    #[test]
    fn decode_255_zero_bytes() {
        assert_eq!(decode(&[0x01; 256], 0), vec![0x00; 255]);
    }

    #[test]
    fn decode_1024_zero_bytes() {
        assert_eq!(decode(&[0x01; 1025], 0), vec![0x00; 1024]);
    }

    #[test]
    fn decode_wikipedia_example_7() {
        let mut enc = vec![0xFF];
        enc.extend(1..=254);
        let expected: Vec<u8> = (1..=254).collect();
        assert_eq!(decode(&enc, 0), expected);
    }

    #[test]
    fn decode_wikipedia_example_8() {
        let mut enc = vec![0x01, 0xFF];
        enc.extend(1..=254);
        let expected: Vec<u8> = (0..=254).collect();
        assert_eq!(decode(&enc, 0), expected);
    }

    #[test]
    fn decode_wikipedia_example_9() {
        let mut enc = vec![0xFF];
        enc.extend(1..=254);
        enc.push(0x02);
        enc.push(0xFF);
        let expected: Vec<u8> = (1..=255).collect();
        assert_eq!(decode(&enc, 0), expected);
    }

    #[test]
    fn round_trip_single_byte_values() {
        for b in 0..=0xFFu8 {
            let dec = vec![b];
            assert_eq!(
                decode(&encode(&dec, 0), 0),
                dec,
                "round-trip failed for byte {:#04x}",
                b
            );
        }
    }

    #[test]
    fn round_trip_two_byte_with_zeros() {
        for b in 0..=0xFFu8 {
            let dec = vec![0x00, b];
            assert_eq!(
                decode(&encode(&dec, 0), 0),
                dec,
                "round-trip failed for [0x00, {:#04x}]",
                b
            );
            let dec = vec![b, 0x00];
            assert_eq!(
                decode(&encode(&dec, 0), 0),
                dec,
                "round-trip failed for [{:#04x}, 0x00]",
                b
            );
        }
    }

    #[test]
    fn round_trip_253_nonzero() {
        assert_eq!(decode(&encode(&[0x42; 253], 0), 0), vec![0x42; 253]);
    }

    #[test]
    fn round_trip_254_nonzero() {
        assert_eq!(decode(&encode(&[0x01; 254], 0), 0), vec![0x01; 254]);
    }

    #[test]
    fn round_trip_255_nonzero() {
        assert_eq!(decode(&encode(&[0x01; 255], 0), 0), vec![0x01; 255]);
    }

    #[test]
    fn round_trip_254_nonzero_then_zero() {
        let mut dec = vec![0x01; 254];
        dec.push(0x00);
        assert_eq!(decode(&encode(&dec, 0), 0), dec);
    }

    #[test]
    fn round_trip_ascending_pattern() {
        let dec: Vec<u8> = (0..512).map(|i| i as u8).collect();
        assert_eq!(decode(&encode(&dec, 0), 0), dec);
    }

    #[test]
    fn round_trip_zero_at_every_nth_position() {
        for n in [1u8, 2, 127, 253, 254, 255] {
            let mut dec = vec![0x42; 1024];
            for i in (0..dec.len()).step_by(n as usize) {
                dec[i] = 0x00;
            }
            assert_eq!(
                decode(&encode(&dec, 0), 0),
                dec,
                "round-trip failed for n={}",
                n
            );
        }
    }
}
