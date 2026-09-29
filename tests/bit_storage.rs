use rustj::{Error, bit_storage::BitStorage};

#[test]
fn arbitrary_offsets_and_tail_masks_match_byte_reference() {
    let bytes: Vec<u8> = (0..260).map(|i| (i % 3 == 0 || i % 7 == 0) as u8).collect();
    let bits = BitStorage::from_bytes(&bytes).unwrap();
    for start in 0..130 {
        for len in [0, 1, 2, 31, 63, 64, 65, 127, 129] {
            let view = bits.slice(start..start + len).unwrap();
            assert_eq!(view.to_bytes(len).unwrap(), bytes[start..start + len]);
            assert_eq!(
                view.count_ones(),
                bytes[start..start + len]
                    .iter()
                    .map(|&x| x as usize)
                    .sum::<usize>()
            );
            assert_eq!(view.get(len), None);
            assert_eq!(
                view.slice(0..len).unwrap().to_bytes(len).unwrap(),
                view.to_bytes(len).unwrap()
            );
        }
    }
    assert_eq!(bits.backing_bytes(), 40);
    assert_eq!(bits.slice(17..18).unwrap().backing_bytes(), 40);
}

#[test]
fn boolean_word_operations_match_bytes_for_different_offsets() {
    let a: Vec<u8> = (0..300).map(|i| (i % 3 == 0) as u8).collect();
    let b: Vec<u8> = (0..300).map(|i| (i % 5 < 2) as u8).collect();
    let left = BitStorage::from_bytes(&a).unwrap();
    let right = BitStorage::from_bytes(&b).unwrap();
    for offset in [0, 1, 31, 63, 64, 65] {
        for len in [0, 1, 63, 64, 65, 129] {
            let x = left.slice(offset..offset + len).unwrap();
            let y = right.slice(67..67 + len).unwrap();
            for (op, result) in [(0, x.and(&y)), (1, x.or(&y)), (2, x.xor(&y))] {
                let expected: Vec<u8> = (0..len)
                    .map(|i| match op {
                        0 => a[offset + i] & b[67 + i],
                        1 => a[offset + i] | b[67 + i],
                        _ => a[offset + i] ^ b[67 + i],
                    })
                    .collect();
                assert_eq!(result.unwrap().to_bytes(len).unwrap(), expected);
            }
        }
    }
}

#[test]
fn invalid_bytes_ranges_and_expansion_are_rejected() {
    assert!(matches!(
        BitStorage::from_bytes(&[0, 2]),
        Err(Error::Domain)
    ));
    let bits = BitStorage::from_bytes(&[1, 0, 1]).unwrap();
    assert!(matches!(bits.slice(0..4), Err(Error::Index)));
    assert!(matches!(bits.to_bytes(2), Err(Error::Limit)));
    assert!(matches!(
        bits.and(&bits.slice(0..1).unwrap()),
        Err(Error::Length)
    ));
    assert_eq!(BitStorage::from_bytes(&[]).unwrap().count_ones(), 0);
}
