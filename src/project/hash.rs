pub fn stable_hash_bytes(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

pub fn stable_hash_str(value: &str) -> u64 {
    stable_hash_bytes(value.as_bytes())
}
