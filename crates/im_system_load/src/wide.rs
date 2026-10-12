// Path: crates/im_system_load/src/wide.rs
// Description: UTF-16 conversions shared by the Windows FFI modules

/// Nul-terminated UTF-16 copy of `text` for `PCWSTR` parameters.
pub fn to_wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Replaces `out` with `units`, decoding lossily and stopping at the first nul.
pub fn decode_into(out: &mut String, units: &[u16]) {
    out.clear();
    let end = units
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(units.len());
    out.extend(
        char::decode_utf16(units[..end].iter().copied())
            .map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER)),
    );
}

/// Length of a nul-terminated UTF-16 string, scanning at most `limit` units.
/// Safety: `ptr` must be readable for `limit` units or up to and including a nul.
pub unsafe fn wide_len(ptr: *const u16, limit: usize) -> usize {
    let mut len = 0;
    while len < limit && unsafe { ptr.add(len).read_unaligned() } != 0 {
        len += 1;
    }
    len
}
