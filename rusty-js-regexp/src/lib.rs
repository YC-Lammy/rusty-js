use ast::{Pattern, GroupSpecifier};

use self::{
    abstraction::{RegExpRecord, State},
    parser::CharSlice,
};

mod abstraction;
mod ast;
mod parser;
mod builder;

#[derive(Debug, Clone, Copy)]
pub struct RegExpFlag(u8);

impl RegExpFlag {
    pub const DotAll: Self = Self(0b00000001);
    pub const IgnoreCase: Self = Self(0b00000010);
    pub const Multiline: Self = Self(0b00000100);
    pub const Unicode: Self = Self(0b00001000);
    pub const None: Self = Self(0);

    pub fn is_dot_all(self) -> bool {
        (self & Self::DotAll).0 != 0
    }

    pub fn is_ignore_case(self) -> bool {
        (self & Self::IgnoreCase).0 != 0
    }

    pub fn is_multiline(self) -> bool {
        (self & Self::Multiline).0 != 0
    }

    pub fn is_unicode(self) -> bool {
        (self & Self::Unicode).0 != 0
    }

    pub fn is_none(self) -> bool {
        self.0 == 0
    }
}

impl std::ops::BitOr for RegExpFlag {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitAnd for RegExpFlag {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

#[derive(Clone)]
pub struct RegExp {
    regex: abstraction::RegExpMatcher,
    groups:Vec<Option<GroupSpecifier>>,

    global:bool,
    sticky:bool,

    pub source: String,
    pub record: abstraction::RegExpRecord,
    pub last_index: usize,
}

impl RegExp {
    pub fn new(pattern: String, flags: &str) -> Result<Self, peg::error::ParseError<usize>> {
        let mut record = RegExpRecord {
            ignore_case: false,
            multiline: false,
            dot_all: false,
            unicode: false,
            capturing_groups_count: 0,
        };

        let mut global = false;
        let mut sticky = false;

        if flags.contains("i") {
            record.ignore_case = true;
        }
        if flags.contains("m") {
            record.multiline = true;
        }
        if flags.contains("s") {
            record.dot_all = true;
        }
        if flags.contains("u") {
            record.unicode = true;
        }

        if flags.contains("g") {
            global = true;
        }

        if flags.contains("y") {
            sticky = true;
        }

        let input = CharSlice(pattern.chars().collect());
        let mut groups = Vec::new();
        let re = parser::parse(&input, record.unicode, &mut groups)?;

        record.capturing_groups_count = groups.len();

        let pat = Pattern{
            disjunction:re,
            groups:groups
        };

        let m = abstraction::create_matcher(&pat, record);

        Ok(Self {
            regex: m,
            groups:pat.groups,
            global,
            sticky,
            source: pattern,
            record: record,
            last_index: 0,
        })
    }

    pub fn get_group_name(&self, index:usize) -> Option<String>{
        if self.groups.len() <= index{
            return None
        }

        if let Some(g) = &self.groups[index]{
            let mut str = String::new();
            let s = g.name.RegExpIdentifierCodePoints();
            s.iter().for_each(|v|{
                let c = char::from_u32(*v).unwrap();
                str.push(c);
            });
            return Some(str);
        } else{
            return None
        }
        
    }

    pub fn exec(&mut self, s: &[u16]) -> Option<State> {
        let length = s.len();
        let mut last_index = self.last_index;

        let global = self.global;
        let sticky = self.sticky;

        if !global && !sticky {
            last_index = 0;
        }

        let r = loop {
            if last_index > length {
                if global || sticky {
                    self.last_index = 0;
                }
                return None;
            };

            let re = (self.regex)(s, last_index as usize);
            if re.is_err() {
                if sticky {
                    self.last_index = 0;
                    return None;
                }

                last_index += 1;
            } else {
                let r = re.unwrap();
                break r;
            }
        };

        if global || sticky {
            self.last_index = r.end_index;
        };

        return Some(r);
    }
}

#[test]
pub fn t() {
    let mut r = RegExp::new("\\s[i-j][0-9]".to_string(), "u").unwrap();
    let s = "p    i9".encode_utf16().collect::<Vec<u16>>();
    let ins = std::time::Instant::now();
    let s = r.exec(&s);
    println!("{}", ins.elapsed().as_nanos());

    if let Some(s) = s {
        println!("{}..{}", s.start_index, s.end_index);
    }

    let re = regress::Regex::new("\\s[i-j][0-9]").unwrap();
    let ins = std::time::Instant::now();
    let m = re.find("p    i9").unwrap();
    println!("{}", ins.elapsed().as_nanos());
    println!("{}..{}", m.start(), m.end());
}
