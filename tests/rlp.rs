use rlp::{decode, encode, encode_bytes, encode_list, from_hex, to_hex, Error, Item};

#[test]
fn single_low_byte_encodes_as_itself() {
    assert_eq!(encode_bytes(&[0x7f]), vec![0x7f]);
    assert_eq!(encode_bytes(&[0x00]), vec![0x00]);
}

#[test]
fn single_high_byte_gets_a_prefix() {
    assert_eq!(encode_bytes(&[0x80]), vec![0x81, 0x80]);
    assert_eq!(encode_bytes(&[0xff]), vec![0x81, 0xff]);
}

#[test]
fn empty_values_are_the_classic_reference_vectors() {
    assert_eq!(to_hex(&encode_bytes(b"")), "0x80");
    assert_eq!(to_hex(&encode_bytes(b"dog")), "0x83646f67");
    assert_eq!(
        to_hex(&encode_list(&[encode_bytes(b"cat"), encode_bytes(b"dog")])),
        "0xc88363617483646f67"
    );
}

#[test]
fn empty_list_and_empty_string_differ() {
    assert_eq!(to_hex(&encode_list(&[])), "0xc0");
    assert_eq!(to_hex(&encode_bytes(b"")), "0x80");
}

#[test]
fn strings_longer_than_55_bytes_use_the_long_form() {
    let payload = vec![b'a'; 56];
    let encoded = encode_bytes(&payload);
    assert_eq!(encoded[0], 0xb8); // 0x80 + 55 + 1 length byte
    assert_eq!(encoded[1], 56);
    assert_eq!(encoded.len(), 58);
}

#[test]
fn round_trips_a_nested_structure() {
    let item = Item::List(vec![
        Item::Bytes(b"cat".to_vec()),
        Item::List(vec![Item::Bytes(b"puppy".to_vec())]),
        Item::Bytes(vec![]),
    ]);
    let encoded = encode(&item);
    assert_eq!(decode(&encoded).unwrap(), item);
}

#[test]
fn reference_vector_for_the_nested_case() {
    let item = Item::List(vec![
        Item::Bytes(b"cat".to_vec()),
        Item::Bytes(b"dog".to_vec()),
    ]);
    assert_eq!(to_hex(&encode(&item)), "0xc88363617483646f67");
}

#[test]
fn rejects_trailing_bytes() {
    let mut encoded = encode_bytes(b"dog");
    encoded.push(0x00);
    assert!(matches!(decode(&encoded), Err(Error::TrailingBytes(1))));
}

#[test]
fn rejects_truncated_input() {
    assert!(matches!(decode(&[0x83, 0x64]), Err(Error::UnexpectedEnd)));
}

#[test]
fn rejects_non_canonical_single_byte() {
    // 0x81 0x05 should have been encoded as the bare byte 0x05
    assert_eq!(decode(&[0x81, 0x05]), Err(Error::NonCanonical));
}

#[test]
fn hex_helpers_round_trip() {
    let bytes = from_hex("0x83646f67").unwrap();
    assert_eq!(bytes, b"\x83dog");
    assert_eq!(to_hex(&bytes), "0x83646f67");
    assert!(from_hex("0xabc").is_err());
}
