use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RegExpFlags(u8);

#[allow(non_upper_case_globals)]
impl RegExpFlags{
    const Empty:Self = Self(0);
    const HasIndices:Self = Self(0b00000001);
    const Global:Self = Self(0b00000010);
    const IgnoreCase:Self = Self(0b00000100);
    const Multiline:Self = Self(0b00001000);
    const DotAll:Self = Self(0b00010000);
    const Unicode:Self = Self(0b00100000);
    const Sticky:Self = Self(0b01000000);

    pub fn dotall(self) -> bool{
        self & Self::DotAll
    }

    pub fn ignore_case(self) -> bool{
        self & Self::IgnoreCase
    }

    pub fn has_indices(self) -> bool{
        self & Self::HasIndices
    }

    pub fn global(self) -> bool{
        self & Self::Global
    }

    pub fn multiline(self) -> bool{
        self & Self::Multiline
    }

    pub fn unicode(self) -> bool{
        self & Self::Unicode
    }

    pub fn sticky(self) -> bool{
        self & Self::Sticky
    }
}

impl std::ops::BitOr for RegExpFlags{
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for RegExpFlags{
    fn bitor_assign(&mut self, rhs: Self) {
        *self = Self(self.0 | rhs.0)
    }
}

impl std::ops::BitAnd for RegExpFlags{
    type Output = bool;
    fn bitand(self, rhs: Self) -> Self::Output {
        (self.0 & rhs.0) != 0
    }
}

pub struct RegExp{
    pub flags:RegExpFlags,
    pub last_index:usize,
    pub matcher:regress::Regex,
}

pub struct Match{
    pub substr:Range<usize>,
    pub captures:Vec<Option<Range<usize>>>,
    pub named_groups:Vec<(String, Range<usize>)>,
}

impl RegExp{
    pub fn with_flags(pattern:&str, flags:&str) -> Result<Self, String>{
        let mut f = RegExpFlags::Empty;
        if flags.contains("d"){
            f |= RegExpFlags::HasIndices;
        };

        if flags.contains("g"){
            f |= RegExpFlags::Global;
        }

        if flags.contains("i"){
            f |= RegExpFlags::IgnoreCase;
        }

        if flags.contains("m"){
            f |= RegExpFlags::Multiline;
        }

        if flags.contains("s"){
            f |= RegExpFlags::DotAll;
        }

        if flags.contains("u"){
            f |= RegExpFlags::Unicode;
        }

        if flags.contains("y"){
            f |= RegExpFlags::Sticky;
        }

        let r = regress::Regex::with_flags(pattern, flags);

        let m = match r{
            Ok(v) => v,
            Err(e) => return Err(e.to_string())
        };
        return Ok(Self { 
            flags:f, 
            last_index: 0, 
            matcher: m
        });
    }

    pub fn exec(&mut self, text:&str) -> Option<Match>{

        let length = text.len();
        let mut last_index = self.last_index;
        let global = self.flags.global();
        let sticky = self.flags.sticky();
        let unicode = self.flags.unicode();

        if !global && !sticky{
            last_index = 0;
        }

        let match_value = loop{
            if last_index > length{
                if global || sticky{
                    self.last_index = 0;
                }

                return None;
            }

            let last_byte_index = last_index;

            let r = self.matcher.find_from(text, last_byte_index).next();

            match r{
                None => {
                    if sticky{
                        self.last_index = 0;
                        return None
                    }

                    last_index = advance_string_index(text, last_index, unicode);
                },
                Some(m) => {
                    if m.start() != last_index{
                        if sticky{
                            self.last_index = 0;
                            return None
                        }

                        last_index = advance_string_index(text, last_index, unicode);
                    } else{
                        break m;
                    }
                }
            }
        };

        let mut e = match_value.end();

        // 14. If fullUnicode is true, then
        // TODO: disabled for now until we have UTF-16 support
        if unicode{
            // e is an index into the Input character list, derived from S, matched by matcher.
            // Let eUTF be the smallest index into S that corresponds to the character at element e of Input.
            // If e is greater than or equal to the number of elements in Input, then eUTF is the number of code units in S.
            // b. Set e to eUTF.
            e = e;
        };

        if global || sticky{
            self.last_index = text[..e].len();
        };

        let mut groups = Vec::new();
        for (name, r) in match_value.named_groups(){
            if let Some(r) = r{
                groups.push((name.to_string(), r));
            }
        };

        return Some(Match{
            substr:match_value.range,
            captures:match_value.captures,
            named_groups:groups,
        })
    }
}

fn advance_string_index(s: &str, index: usize, unicode: bool) -> usize {
    // Regress only works with utf8, so this function differs from the spec.

    // 1. Assert: index ≤ 2^53 - 1.

    // 2. If unicode is false, return index + 1.
    if !unicode {
        return index + 1;
    }

    // 3. Let length be the number of code units in S.
    let length = s.len();

    // 4. If index + 1 ≥ length, return index + 1.
    if index + 1 > length {
        return index + 1;
    }

    // 5. Let cp be ! CodePointAt(S, index).
    let code_point = s.chars().nth(index).unwrap();

    index + code_point.len_utf8()
}