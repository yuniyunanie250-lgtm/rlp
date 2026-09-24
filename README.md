# rlp

Recursive Length Prefix encoding and decoding, in about 160 lines with no
dependencies.

RLP is the serialisation Ethereum uses below the transaction layer: transaction
payloads, block headers, and the nodes of the state trie. It is deliberately
minimal — no type tags, no field names, just length-prefixed bytes and lists —
which is exactly why it is worth reading and reimplementing rather than treating
as a black box.

## The whole spec

- A single byte in `0x00..0x7f` is encoded as itself.
- A byte string of 0..55 bytes is `0x80 + len` followed by the bytes.
- A byte string of 56+ bytes is `0xb7 + len_of_len`, the length as big-endian,
  then the bytes.
- A list uses the same two rules with `0xc0` as the base offset.

## What this implementation enforces

- **Canonical encodings.** `0x81 0x05` is rejected because `0x05` should have
  been encoded as itself; a length that fits in the short form is rejected from
  the long form. Non-canonical RLP is a real source of consensus bugs, since two
  different byte strings can otherwise decode to the same value.
- **No trailing bytes.** `decode` requires the input to be exactly one item.
  Use `decode_at` when you are walking a stream.

## What it does not do

- **No integer type.** Ethereum integers are big-endian byte strings with no
  leading zeros; encoding that convention is a layer above this.
- **No streaming.** Input is a byte slice in memory.

## Development

```bash
cargo test
```

## License

MIT
