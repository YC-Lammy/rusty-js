pub mod data;
pub mod code_point;
pub mod utf16;

pub use code_point::CodePoint;

pub use unicode_id_start::{
    is_id_continue,
    is_id_start
};

pub const WhiteSpace: &'static [char] = &[
    '\u{0009}', '\u{000A}', '\u{000B}', '\u{000C}', '\u{000D}', 
    '\u{0020}',
    '\u{00A0}',
    '\u{1680}', 
    '\u{2000}', '\u{2001}', '\u{2002}', 
    '\u{2003}', '\u{2004}', '\u{2005}', 
    '\u{2006}', '\u{2007}', '\u{2008}', 
    '\u{2009}', '\u{200A}', 
    
    '\u{2028}', '\u{2029}', 
    '\u{202F}', '\u{205F}',
    '\u{205F}',
    '\u{3000}',
    '\u{FEFF}'
];

pub fn is_white_space(c:char) -> bool{
    WhiteSpace.contains(&c)
}

pub fn code_points_to_string<I, CP>(chars: I) -> Vec<char>
where
    I: IntoIterator<Item = CP>,
    CP: Into<CodePoint>,
{
    let mut result = Vec::new();
    for c in chars {
        let cp: CodePoint = c.into();
        let i = cp.0;
        if i <= '\u{FFFF}' {
            result.push(i);
        } else {
            let cu1 = ((i as u32 - 0x10000) / 0x400) + 0xD800;
            let cu2 = ((i as u32 - 0x10000) % 0x400) + 0xDC00;

            result.push(char::from_u32(cu1).unwrap());
            result.push(char::from_u32(cu2).unwrap());
        }
    }
    return result;
}

pub fn canonicalize(c:char, is_utf16:bool) -> char{
    let mut c = c as u32;
    if is_utf16{
        if c < 128{
            if c >= 'A' as u32 && c <= 'Z' as u32{
                c = c + 'a' as u32 - 'A' as u32;
            }
        } else{
            let res = case_fold(char::from_u32(c).unwrap());
            c = res[0] as u32;
        }
    } else{
        if c < 128{
            if c >= 'A' as u32 && c <= 'Z' as u32{
                c = c + 'a' as u32 - 'A' as u32;
            }
        } else{
            let res = char::from_u32(c).unwrap().to_uppercase().collect::<Vec<char>>();

            if res.len() == 1 && res[0] as u32 >= 128{
                c = res[0] as u32;
            }
        }
    }

    return char::from_u32(c).unwrap()
}

/// chars that are unused are null(\0) chars
pub fn case_fold(c:char) -> [char;3]{
    let re = data::CASE_FOLDING_TABLE.binary_search_by(|(x,_)|c.cmp(&c));
    match re{
        Err(_) => [c, '\0', '\0'],
        Ok(e) => {
            data::CASE_FOLDING_TABLE[e].1
        }
    }
}

pub fn MatchProperty(name: &str) -> Option<&'static str> {
    Some(match name {
        "gc" | "General_Category" => "General_Category",
        "sc" | "Script" => "Script",
        "scx" | "Script_Extensions" => "Script_Extensions",
        _ => unreachable!(),
    })
}
