//! Recursive Length Prefix encoding and decoding.
//!
//! RLP is how Ethereum serialises everything below the transaction layer:
//! transactions, blocks, and the leaves of the state trie. The whole spec is two
//! rules — a single byte below `0x80` is itself, and everything else is a length
//! prefix followed by the payload — which makes it worth having without pulling
//! in a dependency.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    Bytes(Vec<u8>),
    List(Vec<Item>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Ran out of input while reading a length or payload.
    UnexpectedEnd,
    /// Length prefix used a non-canonical encoding, which RLP forbids.
    NonCanonical,
    /// Trailing bytes after a complete item.
    TrailingBytes(usize),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::UnexpectedEnd => write!(f, "input ended mid-item"),
            Error::NonCanonical => write!(f, "non-canonical length encoding"),
            Error::TrailingBytes(n) => write!(f, "{} trailing byte(s)", n),
        }
    }
}

impl std::error::Error for Error {}

/// Encode a byte string.
pub fn encode_bytes(data: &[u8]) -> Vec<u8> {
    if data.len() == 1 && data[0] < 0x80 {
        return data.to_vec();
    }
    let mut out = encode_length(data.len(), 0x80);
    out.extend_from_slice(data);
    out
}

/// Encode a list of already-encoded items.
pub fn encode_list(items: &[Vec<u8>]) -> Vec<u8> {
    let payload: Vec<u8> = items.iter().flatten().copied().collect();
    let mut out = encode_length(payload.len(), 0xc0);
    out.extend_from_slice(&payload);
    out
}

/// Encode an arbitrary item tree.
pub fn encode(item: &Item) -> Vec<u8> {
    match item {
        Item::Bytes(b) => encode_bytes(b),
        Item::List(items) => encode_list(&items.iter().map(encode).collect::<Vec<_>>()),
    }
}

fn encode_length(len: usize, offset: u8) -> Vec<u8> {
    if len < 56 {
        return vec![offset + len as u8];
    }
    let be = len.to_be_bytes();
    let first = be.iter().position(|b| *b != 0).unwrap();
    let trimmed = &be[first..];
    let mut out = vec![offset + 55 + trimmed.len() as u8];
    out.extend_from_slice(trimmed);
    out
}

/// Decode exactly one item, rejecting trailing bytes.
pub fn decode(input: &[u8]) -> Result<Item, Error> {
    let (item, used) = decode_at(input, 0)?;
    if used != input.len() {
        return Err(Error::TrailingBytes(input.len() - used));
    }
    Ok(item)
}

/// Decode the item starting at `pos`; returns the item and the position after it.
pub fn decode_at(input: &[u8], pos: usize) -> Result<(Item, usize), Error> {
    let first = *input.get(pos).ok_or(Error::UnexpectedEnd)?;

    if first < 0x80 {
        return Ok((Item::Bytes(vec![first]), pos + 1));
    }

    if first <= 0xb7 {
        let len = (first - 0x80) as usize;
        if len == 1 && input.get(pos + 1).is_some_and(|b| *b < 0x80) {
            return Err(Error::NonCanonical);
        }
        let start = pos + 1;
        let end = start.checked_add(len).ok_or(Error::UnexpectedEnd)?;
        let payload = input.get(start..end).ok_or(Error::UnexpectedEnd)?;
        return Ok((Item::Bytes(payload.to_vec()), end));
    }

    if first <= 0xbf {
        let len_of_len = (first - 0xb7) as usize;
        let len = read_length(input, pos + 1, len_of_len)?;
        if len < 56 {
            return Err(Error::NonCanonical);
        }
        let start = pos + 1 + len_of_len;
        let end = start.checked_add(len).ok_or(Error::UnexpectedEnd)?;
        let payload = input.get(start..end).ok_or(Error::UnexpectedEnd)?;
        return Ok((Item::Bytes(payload.to_vec()), end));
    }

    let (payload_len, header) = if first <= 0xf7 {
        ((first - 0xc0) as usize, pos + 1)
    } else {
        let len_of_len = (first - 0xf7) as usize;
        let len = read_length(input, pos + 1, len_of_len)?;
        if len < 56 {
            return Err(Error::NonCanonical);
        }
        (len, pos + 1 + len_of_len)
    };

    let end = header
        .checked_add(payload_len)
        .ok_or(Error::UnexpectedEnd)?;
    if end > input.len() {
        return Err(Error::UnexpectedEnd);
    }
    let mut items = Vec::new();
    let mut cursor = header;
    while cursor < end {
        let (item, next) = decode_at(input, cursor)?;
        if next > end {
            return Err(Error::UnexpectedEnd);
        }
        items.push(item);
        cursor = next;
    }
    Ok((Item::List(items), end))
}

fn read_length(input: &[u8], pos: usize, len_of_len: usize) -> Result<usize, Error> {
    let bytes = input
        .get(pos..pos + len_of_len)
        .ok_or(Error::UnexpectedEnd)?;
    if bytes[0] == 0 {
        return Err(Error::NonCanonical);
    }
    let mut len = 0usize;
    for b in bytes {
        len = (len << 8) | *b as usize;
    }
    Ok(len)
}

/// Hex helpers, since RLP is normally inspected as hex.
pub fn to_hex(bytes: &[u8]) -> String {
    format!(
        "0x{}",
        bytes
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>()
    )
}

pub fn from_hex(input: &str) -> Result<Vec<u8>, String> {
    let s = input.strip_prefix("0x").unwrap_or(input);
    if s.len() % 2 != 0 {
        return Err("odd number of hex digits".into());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}
