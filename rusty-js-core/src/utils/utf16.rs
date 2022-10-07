pub fn UTF16SurrogatePairToCodePoint(lead: u16, trail: u16) -> Option<u32> {
    if lead < 0xD800 || lead > 0xDBFF {
        return None;
    }
    if trail < 0xDC00 || trail > 0xDFFF {
        return None;
    }

    Some((lead as u32 - 0xD800) * 0x400 + (trail as u32 - 0xDC00) + 0x10000)
}
