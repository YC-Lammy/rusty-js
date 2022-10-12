use std::ops::Range;

use crate::{util::DynamicBuffer, Flags, op::{Op, self}, error::Error};



const CAPTURE_COUNT_MAX:usize = 255;
const STACK_SIZE_MAX:usize = 255;

const CP_LS:u32 = 0x2028;
const CP_PS:u32 = 0x2029;

const TMP_BUF_SIZE:usize = 128;

const Escape_b:u32 = 8;
const Escape_f:u32 = 12;
const Escape_v:u32 = 11;

const char_range_d:[Range<u32>;1] = [0x0020..0x0039+1];
const char_range_s:[Range<u32>;10] = [
    0x0009..0x000D + 1,
    0x0020..0x0020 + 1,
    0x00A0..0x00A0 + 1,
    0x1680..0x1680 + 1,
    0x2000..0x200A + 1,
    /* 2028;LINE SEPARATOR;Zl;0;WS;;;;;N;;;;; */
    /* 2029;PARAGRAPH SEPARATOR;Zp;0;B;;;;;N;;;;; */
    0x2028..0x2029 + 1,
    0x202F..0x202F + 1,
    0x205F..0x205F + 1,
    0x3000..0x3000 + 1,
    /* FEFF;ZERO WIDTH NO-BREAK SPACE;Cf;0;BN;;;;;N;BYTE ORDER MARK;;;; */
    0xFEFF..0xFEFF + 1,
];

const char_range_w:[Range<u32>;4] = [
    0x0030..0x0039 + 1,
    0x0041..0x005A + 1,
    0x005F..0x005F + 1,
    0x0061..0x007A + 1,
];

const ClassRangeBase:u32 = 0x40000000;


#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CharRangeEnum{
    Char_Range_d,
    Char_Range_D,
    Char_Range_s,
    Char_Range_S,
    Char_Range_w,
    Char_Range_W,
}

pub struct ParseState<'a>{
    pub pattern:&'a [u8],
    pub pattern_ptr:&'a [u8],
    pub bytecode:DynamicBuffer,
    pub is_utf16:bool,
    pub flag:Flags,
    pub ignore_case:bool,
    pub capture_count:i32,
    pub total_capture_count:i32,
    pub has_named_captures:i32,
    pub opaque:*const (),
    pub group_names:Vec<Option<String>>,
    pub tmp_buf:Vec<char>,

    pub error:Option<Error>
}

pub fn from_hex(c:char) -> i32{
    if c >= '0' && c <= '9'{
        return c as i32 - '0' as i32
    } else if c >= 'A' && c <= 'F'{
        return c as i32 + 10 - 'A' as i32
    } else if c >= 'a' && c <= 'f'{
        return c as i32 + 10 - 'a' as i32
    } else{
        return -1
    }
}

const utf8_min_code:[u32;5] = [
    0x80, 0x800, 0x10000, 0x00200000, 0x04000000,
];

const utf8_first_code_mask:[u32;5] = [
    0x1f, 0xf, 0x7, 0x3, 0x1,
];

pub fn unicode_from_utf8<'a>(mut p:&'a [u8], max_len:usize, pp:&mut &'a [u8]) -> i32{
    let mut c = p[0] as u32;
    p = &p[1..];
    if c < 0x80{
        *pp = p;
        return c as i32;
    }

    let mut l = 0;
    match c{
        0xC0..=0xDF => l=0,
        0xE0..=0xEF => l=2,
        0xF0..=0xF7 => l=3,
        0xF8..=0xFB => l=4,
        0xFC..=0xFD => l=5,
        _ => return -1,
    };

    if l > max_len -1{
        return -1;
    }

    c &= utf8_first_code_mask[l-1];

    for i in 0..l{
        let b = p[0];
        p = &p[1..];
        if b < 0x80 || b >= 0xC0{
            return -1;
        }
        c = (c<<6) | (b as u32 & 0x3F);
    }

    if c < utf8_min_code[l-1]{
        return -1
    }
    *pp = p;
    return c as i32;
}

pub fn canonicalize(mut c:u32, is_utf16:bool) -> u32{
    if is_utf16{
        if c < 128{
            if c >= 'A' as u32 && c <= 'Z' as u32{
                c = c + 'a' as u32 - 'A' as u32;
            }
        } else{
            let res = rusty_js_unicode::case_fold(char::from_u32(c).unwrap());
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

    return c;
}

fn is_ident_first(c:u32) -> bool{
    const start_table:[u32;4] = [0x0, 0x0010, 0x87FFFFFE, 0x07FFFFFE];
    
    if c < 128{
        let b = start_table[c as usize >> 5] >> (c as usize & 31) &1;
        b != 0
    } else{
        rusty_js_unicode::is_id_start(unsafe{char::from_u32_unchecked(c)})
    }
}

fn is_ident_next(c:u32) -> bool{
    const continue_table_ascii:[u32;4] = [
        /* $ 0-9 A-Z _ a-z */
        0x00000000, 0x03FF0010, 0x87FFFFFE, 0x07FFFFFE
    ];

    if c < 128{
        let b = continue_table_ascii[c as usize >> 5] >> (c as usize &31) &1;
        b != 0
    } else{
        c == 0x200c || c == 0x200D || 
        rusty_js_unicode::is_id_continue(unsafe{char::from_u32_unchecked(c)})
    }
}



impl<'a> ParseState<'a>{
    fn parse_error(&mut self, error:&str, pp:&[u8]){

    }

    fn parse_digits(&mut self, patern:&mut &[u8], allow_overflow:bool) -> i32{
        let mut p = *patern;
        let mut v = 0;
        let mut c;

        loop{
            c = p[0];
            if c < '0' as u8 || c > '9' as u8{
                break;
            }

            v = v * 10 + c as u64 - '0' as u64;
            if v > i32::MAX as u64{
                if allow_overflow{
                    v = i32::MAX as u64
                } else{
                    self.parse_error("integer overflow", p);
                    return -1
                }
            }
            p = &p[1..];
        };
        *patern = p;
        return v as i32;
    }

    fn parse_expect(&mut self, pp:&mut &[u8], c:u32) -> i32{
        let p = *pp;
        if p[0] as u32 != c{
            self.parse_error(&format!("expecting '{}'", char::from_u32(c).unwrap()), p);
            return -1;
        }
        *pp = &p[1..];
        return 0
    }

    /// Parse an escape sequence, *pp points after the '\':
    /// allow_utf16 value:
    /// 0 : no UTF-16 escapes allowed
    /// 1 : UTF-16 escapes allowed
    /// 2 : UTF-16 escapes allowed and escapes of surrogate pairs are
    /// converted to a unicode character (unicode regexp case).
    /// Return the unicode char and update *pp if recognized,
    /// return -1 if malformed escape,
    /// return -2 otherwise. */
    fn parse_escape(&mut self, pp:&mut &[u8], allow_utf16:i8) -> i32{
        let mut p = *pp;
        let mut c = p[0] as u32;
        p = &p[1..];

        match char::from_u32(c).unwrap(){
            'b' => c = Escape_b,
            'f' => c = Escape_f,
            'n' => c = '\n' as u32,
            'r' => c = '\r' as u32,
            't' => c = '\t' as u32,
            'v' => c = Escape_v,
            'x' | 'u' => {
                if (p[0] as char == '{') && allow_utf16 !=0{
                    p = &p[1..];
                    c = 0;

                    loop{
                        let h = from_hex(p[0] as char);
                        p = &p[1..];

                        // not a hex digit
                        if h < 0{
                            return -1;
                        }

                        c = (c <<4) | h as u32;

                        // not a unicode character
                        if c > char::MAX as u32{
                            return -1
                        }

                        if p[0] as char == '}'{
                            break;
                        }
                    };
                    p = &p[1..];

                } else{
                    let mut n= 4;

                    if c == 'x' as u32{
                        n = 2
                    }

                    c = 0;
                    for i in 0..n{
                        let h = from_hex(p[0] as char);
                        p = &p[1..];

                        if h < 0{
                            return -1;
                        }

                        c = (c << 4) | h as u32;
                    }

                    if c >= 0xD800 && c < 0xDC00 && allow_utf16 == 2 
                    && p[0] as char == '\\' && p[1] as char == 'u'{
                        let mut c1 = 0;

                        let mut i = 0;
                        for a in 0..4{
                            i = a;
                            let h = from_hex(p[2 + i] as char);
                            // not a hex digit
                            if h < 0{
                                break;
                            }
                            c1 = (c1<<4) | h as u32;
                        }

                        if i == 4 && c1 >= 0xDC00 && c1 < 0xE000{
                            p = &p[6..];
                            c = (((c & 0x3FF) << 10) | (c1 & 0x3FF)) + 0x10000;
                        }
                    }
                }
            },
            '0'..='7' => {
                c -= '0' as u32;

                if allow_utf16 == 2{
                    if c != 0 || (p[0] as char).is_digit(10){
                        return -1;
                    }
                } else{
                    let mut v = p[0] as u32 - '0' as u32;

                    if v > 7{
                        // does nothing
                    } else{
                        c = (c<<3) | v;
                        p = &p[1..];

                        if c >= 32{
                            // does nothing
                        } else{
                            v = p[0] as u32 - '0' as u32;

                            if v > 7{
                                // does nothing
                            } else{
                                c = (c<<3) | v;
                                p = &p[1..];
                            }
                        }
                    }
                }
            },

            _ => return -2
        };

        *pp = p;
        return c as i32
    }

    fn parse_unicode_property(&mut self, range:&mut Option<&[Range<u32>]>, pp:&mut &[u8], is_inv:bool) -> i32{
        let mut p = *pp;

        if p[0] as char != '{'{
            self.parse_error("expecting '{' after \\p", p);
            return -1;
        }

        p = &p[1..];
        let mut name:[u8;64] = [0u8;64];
        let mut q:&mut [u8] = &mut name;

        loop{
            if !p[0].is_ascii(){
                break;
            }
            if q.len() >= 64{
                self.parse_error("unknown unicode property name", &(*pp)[1..]);
                return -1;
            }
            q[0] = p[0];
            q = &mut q[1..];
            p = &p[1..];
        };
        
        let l = q.len();
        let name = std::str::from_utf8(&name[0..name.len() - l]).unwrap();

        let mut value = [0u8;64];
        q = &mut value;

        if p[0] as char == '='{
            p = &p[1..];
            loop{
                if !p[0].is_ascii(){
                    break;
                }
                if q.len() >= 64{
                    self.parse_error("unknown unicode property value", &(*pp)[name.len() + 1..]);
                    return -1;
                }
                q[0] = p[0];
                q = &mut q[1..];
                p = &p[1..];
            };
        }

        let value = std::str::from_utf8(q).unwrap();

        if p[0] as char != '}'{
            self.parse_error("unknown unicode property value", p);
            return -1;
        }

        p = &p[1..];

        let mut script_ext = false;

        if name == "Script" || name == "sc" {
            script_ext = false;
            todo!()
        } else if name == "Script_Extensions" || name == "scx" {
            script_ext = true;
            todo!()

        } else if name == "General_Catagory" || name == "gc"{
            todo!()

        } else if value.len() == 0{
            todo!()
        } else{
            self.parse_error("unknown unicode property name", &(*pp)[1..]);
            return -1;
        }

        if is_inv{
            todo!()
        }

        *pp = p;
        return 0;
    }

    fn get_class_atom(&mut self, pp:&mut &[u8], inclass:bool, crange:&mut Option<&[Range<u32>]>, range_exclusive:&mut bool) -> i32{
        let mut p = *pp;

        
        let mut c = p[0] as u32;

        match char::from_u32(c).unwrap(){
            '\\' => {
                p = &p[1..];
                if p.len() == 0{
                    self.parse_error("unexpected end", *pp);
                    return 0;
                };

                c = p[0] as u32;
                p = &p[1..];

                
                match char::from_u32(c).unwrap(){
                    'd' => {
                        *crange = Some(&char_range_w);
                        c = ClassRangeBase;
                    },
                    'D' => {
                        *crange = Some(&char_range_d);
                        *range_exclusive = true;
                        c = ClassRangeBase;
                    },
                    's' => {
                        *crange = Some(&char_range_s);
                        c = ClassRangeBase;
                    },
                    'S' => {
                        *crange = Some(&char_range_s);
                        *range_exclusive = true;
                        c = ClassRangeBase;
                    },
                    'w' => {
                        *crange = Some(&char_range_w);
                        c = ClassRangeBase;
                    },
                    'W' => {
                        *crange = Some(&char_range_w);
                        *range_exclusive = true;
                        c = ClassRangeBase;
                    },
                    'c' => {
                        c = p[0] as u32;

                        if (c >= 'a' as u32 && c <= 'z' as u32) ||
                            (c >= 'A' as u32 && c <= 'Z' as u32) ||
                            (((c >= '0' as u32 && c <= '9' as u32) || 
                            c == '_' as u32) && inclass && !self.is_utf16) {
                                c &= 0x1F;
                                p = &p[1..];
                        } else if self.is_utf16{
                            self.parse_error("invalid escape sequence in regular expression", p);
                            return -1;
                        } else{
                            p = &(*pp)[1..];
                            c = '\\' as u32;
                        }
                    },
                    'p' | 'P' => {
                        if self.is_utf16{
                            if self.parse_unicode_property(crange, pp, (c=='P' as u32)) != 0{
                                return -1;
                            }
                            c = ClassRangeBase;
                        }
                    },
                    _ => {
                        // reset p
                        p = &(*pp)[1..];

                        let ret = self.parse_escape(pp, self.is_utf16 as i8 *2);

                        if ret >= 0{
                            c = ret as u32;
                        } else{
                            if ret == -2 && p[0] != '\0' as u8 && "^$\\.*+?()[]{}|/".contains(p[0] as char) {
                                // valid to contain these characters
                                // normal char
                                if c >= 128{
                                    c = unicode_from_utf8(p, 6, &mut p) as u32;
                                    if c > char::MAX as u32 && !self.is_utf16{
                                        self.parse_error("malformed unicode char", p);
                                        return -1;
                                    }
                                } else{
                                    p = &p[1..];
                                }
                            } else if self.is_utf16{
                                self.parse_error("invalid escape sequence in regular expression", p);

                            } else{
                                // normal char
                                if c >= 128{
                                    c = unicode_from_utf8(p, 6, &mut p) as u32;
                                    if c > char::MAX as u32 && !self.is_utf16{
                                        self.parse_error("malformed unicode char", p);
                                        return -1;
                                    }
                                } else{
                                    p = &p[1..];
                                }
                            }
                        }
                    }
                }
            },

            '\0' => {
                if p.len() == 0{
                    self.parse_error("unexpected end", p);
                }
            },
            _ => {
                // normal char
                if c >= 128{
                    c = unicode_from_utf8(p, 6, &mut p) as u32;
                    if c > char::MAX as u32 && !self.is_utf16{
                        // todo: should handle non BMP-1 code points
                        self.parse_error("malformed unicode char", p);
                        return -1;
                    }
                } else{
                    p = &p[1..];
                }
            }
        };

        *pp = p;
        return c as i32;
    }

    fn parse_char_class(&mut self, pp:&mut &[u8]) -> i32{
        let mut p = *pp;
        // skip '['
        p = &p[1..];

        let mut invert = false;

        if p[0] == '^' as u8{
            p = &p[1..];
            invert = true;
        }

        // (range, exclusive)
        let mut cr:Vec<(Range<u32>, bool)> = Vec::new();
        let mut c1 = 0;
        let mut c2 = 0;

        let mut cr1_tmp:[Range<u32>;1];
        let mut cr1 = None;
        let mut cr1_exclusive = false;
        loop{
            if p[0] == ']' as u8{
                break;
            }

            c1 = self.get_class_atom(&mut p, true, &mut cr1, &mut cr1_exclusive);
            if c1 < 0{
                return -1;
            }

            let mut class_atom = false;
            if p[0] == '-' as u8 && p[1] != ']' as u8 {

                // skip '-'
                let mut p0 = &p[1..];

                if c1 as u32 >= ClassRangeBase{
                    if self.is_utf16{
                        self.parse_error("invalid class range", p);
                        return -1;
                    }
                    class_atom = true;

                } else{
                    // parse another atom
                    c2 = self.get_class_atom(&mut p0, true, &mut cr1, &mut cr1_exclusive);
                    if c2 < 0{
                        return -1
                    }

                    if c2 as u32 >= ClassRangeBase{
                        if self.is_utf16{
                            self.parse_error("invalid class range", p);
                            return -1;
                        }
                        class_atom = true;
                    } else{
                        p = p0;
                        if c2 < c1{
                            self.parse_error("invalid class range", p);
                            return -1
                        }

                        cr1_tmp = [c1 as u32..c2 as u32+1];
                        cr1 = Some(&cr1_tmp)
                    }
                }
            } else{
                class_atom = true;
            }

            if class_atom{
                if c1 as u32 >= ClassRangeBase{
                    for i in cr1{
                        for i in i{
                            cr.push((i.clone(), cr1_exclusive));
                        }
                    }
                } else{
                    cr.push((c1 as u32..c1 as u32+1, false));
                }
            }
        };

        if self.ignore_case{
            let mut v = Vec::new();
            for i in &cr{
                let r = i.0.clone();

                if r.start >= 'a' as u32 && r.start <= 'z' as u32{
                    let s = r.start + 'A' as u32 - 'a' as u32;
                    let e = r.end + 'A' as u32 - 'a' as u32;
                    v.push((s..e, i.1));
                }
            }
            cr.extend(v);
        }

        if invert{
            for i in &mut cr{
                *i = (i.0.clone(), !i.1);
            };
        };
        self.emit_ranges(&cr);

        p = &p[1..];
        *pp = p;
        return 0;
    }

    fn emit_ranges(&mut self, ranges:&[(Range<u32>, bool)]){

        if ranges.len() == 0{
            self.bytecode.push_u8(Op::Char32 as u8);
            self.bytecode.push_u32(char::MAX as u32 +1);
            return;
        }

        self.bytecode.push_u8(Op::Ranges as u8);
        self.bytecode.push_u16(ranges.len() as u16);

        for (r, exclusive) in ranges{
            self.bytecode.push_u32(r.start);
            self.bytecode.push_u32(r.end);
            self.bytecode.push_bool(*exclusive);
        };
    }

    /// Return:
    /// 1 if the opcodes in bc_buf[] always advance the character pointer.
    /// 0 if the character pointer may not be advanced.
    /// -1 if the code may depend on side effects of its previous execution (backreference)
    fn check_advence(&self, offset:usize) -> i32{
        let mut ret = -2;

        let mut iter = self.bytecode.iter();

        for i in 0..offset{
            iter.get_next_u8().unwrap();
        }

        let mut has_back_reference = false;

        let mut capture_bitmap:Vec<u8> = Vec::new();

        loop{
            let opcode = match iter.get_next_u8(){
                Some(v) => unsafe{std::mem::transmute::<u8,Op>(v)},
                None => break,
            };

            match opcode{
                Op::Ranges => {
                    Op::Ranges.consume_iter(&mut iter);
                    if ret == -2{
                        ret = 1;
                    }
                },
                Op::Char |
                Op::Char32 |
                Op::Dot |
                Op::Any => {
                    if ret == 2{
                        ret =1;
                    }
                },
                Op::LineStart |
                Op::LineEnd |
                Op::PushU32 |
                Op::PushCharPos |
                Op::Drop |
                Op::WordBoundary |
                Op::NotWordBoundary |
                Op::Prev => {},
                Op::SaveStart |
                Op::SaveEnd => {
                    let val = iter.get_next_u8().unwrap();
                    if capture_bitmap.len() -1 < val as usize{
                        capture_bitmap.resize(val as usize + 1, 0);
                    }
                    capture_bitmap[val as usize] |= 1;
                },
                Op::SaveReset => {
                    let mut val = iter.get_next_u8().unwrap();
                    let last = iter.get_next_u8().unwrap();

                    while val < last{
                        if capture_bitmap.len() -1 < val as usize{
                            capture_bitmap.resize(val as usize + 1, 0);
                        }
                        capture_bitmap[val as usize] |= 1;
                        val += 1;
                    }
                },
                Op::BackReference |
                Op::BackwardBackReference => {
                    let val = iter.get_next_u8().unwrap();
                    if capture_bitmap.len() -1 < val as usize{
                        capture_bitmap.resize(val as usize + 1, 0);
                    }
                    capture_bitmap[val as usize] |= 2;
                    has_back_reference = true;
                },
                o => {
                    o.consume_iter(&mut iter);
                    if ret == -2{
                        ret = 0;
                    }
                }
            }
        };

        if has_back_reference{
            for i in 0..capture_bitmap.len(){
                if capture_bitmap[i] == 3{
                    return -1;
                }
            };
        }
        if ret == -2{
            ret = 0;
        }
        return ret
    }

    fn is_simple_quantifier(&mut self, offset:usize) -> i32{
        let mut count = 0;
        let mut pos = 0;

        let mut iter = self.bytecode.iter();

        for i in 0..offset{
            iter.get_next_u8().unwrap();
        }

        loop{
            let opcode = match iter.get_next_u8(){
                Some(v) => unsafe{std::mem::transmute::<u8,Op>(v)},
                None => break,
            };

            match opcode{
                Op::Ranges => {
                    Op::Ranges.consume_iter(&mut iter);
                    count += 1;
                },
                Op::Char |
                Op::Char32 |
                Op::Dot |
                Op::Any => {
                    count += 1;
                },
                Op::LineStart |
                Op::LineEnd |
                Op::WordBoundary |
                Op::NotWordBoundary => {},
                _ => {
                    return -1;
                }
            };
        };

        return count
    }

    fn parse_group_name(&mut self, buf:&mut String, pp:&mut &[u8], is_utf16:bool) -> i32{
        let mut p = *pp;
        let mut c:u32;

        let q = buf.len();

        loop{
            c = p[0] as u32;

            if c == '\\' as u32{
                p = &p[1..];
                if p[0] != b'u'{
                    return -1;
                }
                c = self.parse_escape(&mut p, is_utf16 as i8 *2) as u32;

            } else if c == '>' as u32{
                break;
            } else if c >= 128{
                c = unicode_from_utf8(p, 6, &mut p) as u32;
            } else{
                p = &p[1..];
            }

            if c > 0x10FFFF{
                return -1
            }

            if q == buf.len(){
                if !is_ident_first(c){
                    return -1;
                }
            } else{
                if !is_ident_next(c){
                    return -1
                }
            };

            buf.push(char::from_u32(c).unwrap());
        };

        if q == buf.len(){
            return -1;
        }

        p = &p[1..];
        *pp = p;
        return 0
    }

    /// if capture_name = NONE: return the number of captures + 1.
    /// Otherwise, return the capture index corresponding to capture_name
    /// or -1 if none
    fn parse_captures(&mut self, phas_named_captures:&mut i32, capture_name:Option<&str>) -> i32{

        let mut p = self.pattern;
        let mut name = String::new();
        let mut capture_index = 1;

        let mut i = 0;
        for a in 0..p.len(){
            i += 1;
            match p[i]{
                b'(' => {
                    if p[i+1] == b'?'{
                        if p[i+2] == b'<' && p[i+3] != b'=' && p[i+3] != b'!'{
                            *phas_named_captures = 1;

                            if capture_name.is_some() {
                                i += 3;
                                if self.parse_group_name(&mut name, &mut p, self.is_utf16) == 0{
                                    match &capture_name{
                                        None => return capture_index,
                                        Some(s) => {
                                            if s.eq(&name){
                                                return capture_index
                                            }
                                        }
                                    };
                                    
                                }
                            };

                            capture_index += 1;
                            if capture_index >= CAPTURE_COUNT_MAX as i32{
                                if capture_name.is_some(){
                                    return -1;
                                } else{
                                    return capture_index
                                }
                            }
                        }
                    } else{
                        capture_index += 1;
                        if capture_index >= CAPTURE_COUNT_MAX as i32{
                            if capture_name.is_some(){
                                return -1;
                            } else{
                                return capture_index
                            }
                        }
                    };
                },
                b'\\' => {
                    i += 1;
                },
                b'[' => {
                    if p[i] == b']'{
                        i += 1;
                    }
                    i += 1;
                    loop{
                        
                        if i >= self.pattern.len(){
                            break
                        }
                        if p[i] == b']'{
                            break;
                        }

                        if p[i] == b'\\'{
                            i += 1;
                        }
                        i += 1;
                    };
                },
                
                _ => {
                    // TODO
                }
            };
        };
        
        if capture_name.is_some(){
            return -1;
        } else{
            return capture_index
        }
    }

    fn count_captures(&mut self) -> i32{
        if self.total_capture_count < 0{
            let mut phas = self.has_named_captures;
            self.total_capture_count = self.parse_captures(&mut phas, None);
            self.has_named_captures = phas;
        }

        return self.total_capture_count
    }

    fn has_named_captures(&mut self) -> bool{
        if self.has_named_captures < 0{
            self.count_captures();
        }
        return self.has_named_captures != 0;
    }

    fn find_group_name(&mut self, name:&str) -> i32{
        let mut i = 0;
        for s in &self.group_names{
            if let Some(s) = s{
                if name == s{
                    return i;
                }
            }
            
            i += 1;
        };
        return -1;
    }

    fn parse_term(&mut self, is_backward:bool) -> i32{
        let mut p = self.pattern_ptr;
        let mut c = p[0] as u32;
        
        let mut last_atom_start = -1;
        let mut last_capture_count = 0;

        let mut cr = None;
        let mut cr_exclusive = false;

        let mut is_neg = false;
        let mut is_backward_lookahead = false;

        let mut i = 0;
        while i < 1{
        match c as u8{
            b'^' => {
                p = &p[1..];
                self.bytecode.push_u8(Op::LineStart as u8);
            },
            b'$' => {
                p = &p[1..];
                self.bytecode.push_u8(Op::LineEnd as u8);
            },
            b'.' => {
                p = &p[1..];
                last_atom_start = self.bytecode.len() as i32;
                last_capture_count = self.capture_count;

                if is_backward{
                    self.bytecode.push_u8(Op::Prev as u8);
                }

                if self.flag.dotall(){
                    self.bytecode.push_u8(Op::Any as u8);
                } else{
                    self.bytecode.push_u8(Op::Dot as u8);
                }

                if is_backward{
                    self.bytecode.push_u8(Op::Prev as u8);
                }
                
            },
            b'{' => {
                if self.is_utf16{
                    self.parse_error("syntax error", p);
                    return -1;
                } else if !p[1].is_ascii_digit(){
                    let r = self.get_class_atom(&mut p, false, &mut cr, &mut cr_exclusive);
                    if r < 0{
                        return -1;
                    }
                    c = r as u32;
                } else{
                    let mut p1 = &p[1..];
                    self.parse_digits(&mut p1, true);

                    if p1[0] == b','{
                        p1 = &p1[1..];
                        if p1[0].is_ascii_digit(){
                            self.parse_digits(&mut p1, true);
                        }
                    }

                    if p1[0] != b'}'{
                        let r = self.get_class_atom(&mut p, false, &mut cr, &mut cr_exclusive);
                        if r < 0{
                            return -1;
                        }
                        c = r as u32;
                    }
                };
                // fall through
                i = 0;
            },

            b'*' |
            b'+' |
            b'?' => {
                self.parse_error("nothing to repeat", p);
                return -1;
            },
            b'(' => {
                if p[1] == b'?' {
                    if p[2] == b':' {
                        p = &p[3..];
                        last_atom_start = self.bytecode.len() as i32;
                        last_capture_count = self.capture_count;
                        self.pattern_ptr = p;

                        if self.parse_disjunction(is_backward) != 0{
                            return -1;
                        }

                        p = self.pattern_ptr;

                        if self.parse_expect(&mut p, ')' as u32) != 0{
                            return -1
                        }

                    } else if p[2] == b'=' || p[2] == b'!'{
                        is_neg = p[2] == b'!';
                        is_backward_lookahead = false;
                        p = &p[3..];

                        if !self.is_utf16 && !is_backward_lookahead{
                            last_atom_start = self.bytecode.len() as i32;
                            last_capture_count = self.capture_count;
                        }

                        if is_neg{
                            self.bytecode.push_u8(Op::NegativeLookahead as u8);
                        } else{
                            self.bytecode.push_u8(Op::Lookahead as u8);
                        }
                        
                        let pos = self.bytecode.len();
                        self.bytecode.push_u32(0);
                        self.pattern_ptr = p;

                        if self.parse_disjunction(is_backward_lookahead) != 0{
                            return -1
                        }
                        p = self.pattern_ptr;
                        
                        if self.parse_expect(&mut p, ')' as u32) != 0{
                            return -1;
                        }
                        self.bytecode.push_u8(Op::Match as u8);

                        self.bytecode.replace_u32(pos, (self.bytecode.len() - pos +4) as u32);

                    } else if p[2] == b'<' && (p[3] == b'=' || p[3] == b'!') {
                        is_neg = p[3] == b'!';
                        is_backward_lookahead = true;
                        p = &p[4..];

                        if !self.is_utf16 && !is_backward_lookahead{
                            last_atom_start = self.bytecode.len() as i32;
                            last_capture_count = self.capture_count;
                        }

                        if is_neg{
                            self.bytecode.push_u8(Op::NegativeLookahead as u8);
                        } else{
                            self.bytecode.push_u8(Op::Lookahead as u8);
                        }
                        
                        let pos = self.bytecode.len();
                        self.bytecode.push_u32(0);

                        self.pattern_ptr = p;

                        if self.parse_disjunction(is_backward_lookahead) != 0{
                            return -1
                        }
                        p = self.pattern_ptr;
                        
                        if self.parse_expect(&mut p, ')' as u32) != 0{
                            return -1;
                        }
                        self.bytecode.push_u8(Op::Match as u8);

                        self.bytecode.replace_u32(pos, (self.bytecode.len() - pos +4) as u32);

                    } else if p[2] == b'<'{
                        p = &p[3..];
                        let mut s = String::new();
                        if self.parse_group_name(&mut s, &mut p, self.is_utf16) != 0 {
                            self.parse_error("invalid group name", p);
                            return -1;
                        }

                        if self.find_group_name(&s) > 0{
                            self.parse_error("invalid group name", p);
                        }

                        self.group_names.push(Some(s));

                        self.has_named_captures = 1;

                        // parse capture
                        if self.capture_count as usize >= CAPTURE_COUNT_MAX{
                            self.parse_error("too many captures", p);
                            return -1;
                        }
    
                        last_atom_start = self.bytecode.len() as i32;
                        last_capture_count = self.capture_count;
    
                        let capture_index = self.capture_count;
                        self.capture_count += 1;
    
                        if is_backward{
                            self.bytecode.push_u8(Op::SaveEnd as u8);
                        } else{
                            self.bytecode.push_u8(Op::SaveStart as u8);
                        }
                        self.bytecode.push_u8(capture_index as u8);
    
                        self.pattern_ptr = p;
                        
                        if self.parse_disjunction(is_backward) != 0{
                            return -1;
                        }
    
                        p = self.pattern_ptr;
    
                        if is_backward{
                            self.bytecode.push_u8(Op::SaveStart as u8);
                        } else{
                            self.bytecode.push_u8(Op::SaveEnd as u8);
                        }
                        self.bytecode.push_u8(capture_index as u8);
                        
                        if self.parse_expect(&mut p, ')' as u32) != 0{
                            return -1;
                        }

                    } else {
                        self.parse_error("invalid group", p);
                        return -1;
                    };
                } else{
                    let mut capture_index = 0;
                    p = &p[1..];

                    self.group_names.push(None);

                    // parse capture

                    if self.capture_count as usize >= CAPTURE_COUNT_MAX{
                        self.parse_error("too many captures", p);
                        return -1;
                    }

                    last_atom_start = self.bytecode.len() as i32;
                    last_capture_count = self.capture_count;

                    capture_index = self.capture_count;
                    self.capture_count += 1;

                    if is_backward{
                        self.bytecode.push_u8(Op::SaveEnd as u8);
                    } else{
                        self.bytecode.push_u8(Op::SaveStart as u8);
                    }
                    self.bytecode.push_u8(capture_index as u8);

                    self.pattern_ptr = p;
                    
                    if self.parse_disjunction(is_backward) != 0{
                        return -1;
                    }

                    p = self.pattern_ptr;

                    if is_backward{
                        self.bytecode.push_u8(Op::SaveStart as u8);
                    } else{
                        self.bytecode.push_u8(Op::SaveEnd as u8);
                    }
                    self.bytecode.push_u8(capture_index as u8);
                    
                    if self.parse_expect(&mut p, ')' as u32) != 0{
                        return -1;
                    }
                }
            },

            b'\\' => {
                match p[1]{
                    b'b' => {
                        self.bytecode.push_u8(Op::WordBoundary as u8);
                        p = &p[2..];
                    },
                    b'B' => {
                        self.bytecode.push_u8(Op::NotWordBoundary as u8);
                        p = &p[2..];
                    },
                    b'k' => {
                        let mut p1 = p;
                        let mut dummy = 0;

                        if p1[2] != b'<' {
                            if self.is_utf16 || self.has_named_captures(){
                                self.parse_error("expecting group name", &p1[1..]);
                                return -1
                            } else{
                                // parse class atom
                                let r = self.get_class_atom(&mut p, false, &mut cr, &mut cr_exclusive);
                                if r < 0{
                                    return -1;
                                }
                                c = r as u32;
                            }
                        };

                        p1 = &p1[3..];

                        let mut s = String::new();
                        if self.parse_group_name(&mut s, &mut p1, self.is_utf16) != 0{

                            if self.is_utf16 || self.has_named_captures(){
                                self.parse_error("invalid group name", p1);

                            } else{
                                // parse class atom
                                let r = self.get_class_atom(&mut p, false, &mut cr, &mut cr_exclusive);
                                if r < 0{
                                    return -1;
                                }
                                c = r as u32;
                            }
                        };

                        let r = self.find_group_name(&s) as u32;
                        if r < 0{
                            let r = self.parse_captures(&mut dummy, Some(&s));

                            if r <0{
                                if self.is_utf16 || self.has_named_captures(){
                                    self.parse_error("group name not defined", p1);
                                    return -1;
                                } else{

                                    // parse class atom
                                    let r = self.get_class_atom(&mut p, false, &mut cr, &mut cr_exclusive);
                                    if r < 0{
                                        return -1;
                                    }
                                    c = r as u32;
                                
                                }
                            }
                            c = r as u32;

                        }
                        c = r as u32;
                        p = p1;
                        
                        // emit back reference
                        last_atom_start = self.bytecode.len() as i32;
                        last_capture_count = self.capture_count;

                        if is_backward{
                            self.bytecode.push_u8(Op::BackwardBackReference as u8);
                        } else{
                            self.bytecode.push_u8(Op::BackReference as u8);
                        }

                        self.bytecode.push_u8(c as u8);
                    },

                    b'0' => {
                        p = &p[2..];
                        c = 0;
        
                        if self.is_utf16{
                            if p[0].is_ascii_digit(){
                                self.parse_error("invalid decimal escape in regular expression", p);
                                return -1;
                            }
        
                        } else{
                            if p[0] >= b'0' && p[0] <= b'7'{
                                c = (c<<3) + p[0] as u32 - '0' as u32;
                                p = &p[1..];
                            }
                        };
        
                        // normal char
                        last_atom_start = self.bytecode.len() as i32;
                        last_capture_count = self.capture_count;
        
                        if is_backward{
                            self.bytecode.push_u8(Op::Prev as u8);
                        }
        
                        if c >= ClassRangeBase{
                            if let Some(cr) = cr{
                                let r = cr.iter().map(|v|(v.clone(), cr_exclusive)).collect::<Vec<_>>();
                                self.emit_ranges(&r);
                            }
                        } else{
                            if self.flag.ignore_case(){
                                c = canonicalize(c, self.is_utf16);
                            }
        
                            if c <= 0xffff{
                                self.bytecode.push_u8(Op::Char as u8);
                                self.bytecode.push_u16(c as u16);
                            } else{
                                self.bytecode.push_u8(Op::Char32 as u8);
                                self.bytecode.push_u32(c);
                            }
                        }
        
                        if is_backward{
                            self.bytecode.push_u8(Op::Prev as u8);
                        }

                        break;

                        // end normal char
                    },
        
                    b'1'..= b'9' => {
                        p = &p[1..];
                        let mut q = p;
        
                        let r = self.parse_digits(&mut p, false);
        
                        if r < 0 || (r >= self.capture_count && r >= self.count_captures()){
                            if !self.is_utf16{
                                p = q;
        
                                if p[0] <= b'7'{
                                    c =0;
                                    if p[0] <= b'3'{
                                        c = p[0] as u32 - '0' as u32;
                                        p = &p[1..];
                                    }
        
                                    if p[0] >= b'0' && p[0] <= b'7'{
                                        c = (c<<3) + p[0] as u32 - '0' as u32;
        
                                        p = &p[1..];
        
                                        if p[0] >= b'0' && p[0] <= b'7'{
                                            c = (c<<3) + p[0] as u32 - '0' as u32;
            
                                            p = &p[1..];
                                        }
                                    }
                                } else{
                                    c = p[0] as u32;
                                    p = &p[1..];
                                };
        
                                // normal char
                                last_atom_start = self.bytecode.len() as i32;
                                last_capture_count = self.capture_count;
        
                                if is_backward{
                                    self.bytecode.push_u8(Op::Prev as u8);
                                }
        
                                if c >= ClassRangeBase{
                                    if let Some(cr) = cr{
                                        let r = cr.iter().map(|v|(v.clone(), cr_exclusive)).collect::<Vec<_>>();
                                        self.emit_ranges(&r);
                                    }
                                } else{
                                    if self.flag.ignore_case(){
                                        c = canonicalize(c, self.is_utf16);
                                    }
        
                                    if c <= 0xffff{
                                        self.bytecode.push_u8(Op::Char as u8);
                                        self.bytecode.push_u16(c as u16);
                                    } else{
                                        self.bytecode.push_u8(Op::Char32 as u8);
                                        self.bytecode.push_u32(c);
                                    }
                                }
        
                                if is_backward{
                                    self.bytecode.push_u8(Op::Prev as u8);
                                };

                                break;
                                // end normal char

                            } else{
                                self.parse_error("back reference out of range in regular expression", p);
                                return -1;
                            }
                        };
        
                        c = r as u32;
        
                        // emit back reference
                        last_atom_start = self.bytecode.len() as i32;
                        last_capture_count = self.capture_count;
                        
                        if is_backward{
                            self.bytecode.push_u8(Op::BackwardBackReference as u8);
                        } else{
                            self.bytecode.push_u8(Op::BackReference as u8);
                        }
        
                        self.bytecode.push_u8(c as u8);
                    },
        
                    _ => {
                        // parse class atom
                        let r = self.get_class_atom(&mut p, false, &mut cr, &mut cr_exclusive);
                        if r < 0{
                            return -1;
                        }
                        c = r as u32;
                    }

                };
                
                
            },

            b'[' => {
                last_atom_start = self.bytecode.len() as i32;
                last_capture_count = self.capture_count;

                if is_backward{
                    self.bytecode.push_u8(Op::Prev as u8);
                }

                if self.parse_char_class(&mut p) != 0{
                    return -1;
                }

                if is_backward{
                    self.bytecode.push_u8(Op::Prev as u8);
                }
            },

            b']' |
            b'}' => {
                if self.is_utf16{
                    self.parse_error("syntax error", p);
                    return -1;
                }

                // parse class atom
                let r = self.get_class_atom(&mut p, false, &mut cr, &mut cr_exclusive);
                if r < 0{
                    return -1;
                }
                c = r as u32;
            },

            _ => {
                // parse class atom
                let r = self.get_class_atom(&mut p, false, &mut cr, &mut cr_exclusive);
                if r < 0{
                    return -1;
                }
                c = r as u32;

                // normal char
                last_atom_start = self.bytecode.len() as i32;
                last_capture_count = self.capture_count;

                if is_backward{
                    self.bytecode.push_u8(Op::Prev as u8);
                }

                if c >= ClassRangeBase{
                    if let Some(cr) = cr{
                        let r = cr.iter().map(|v|(v.clone(), cr_exclusive)).collect::<Vec<_>>();
                        self.emit_ranges(&r);
                    }
                } else{
                    if self.flag.ignore_case(){
                        c = canonicalize(c, self.is_utf16);
                    }

                    if c <= 0xffff{
                        self.bytecode.push_u8(Op::Char as u8);
                        self.bytecode.push_u16(c as u16);
                    } else{
                        self.bytecode.push_u8(Op::Char32 as u8);
                        self.bytecode.push_u32(c);
                    }
                }

                if is_backward{
                    self.bytecode.push_u8(Op::Prev as u8);
                }

                break;

                // end normal char
            }
            
        };
        i += 1;
        };

        let mut quantifier = false;
        let mut qmin = 0;
        let mut qmax = 0;

        // quantifier
        if last_atom_start >= 0{
            c = p[0] as u32;

            match p[0]{
                b'*' => {
                    p = &p[1..];
                    qmin = 0;
                    qmax = i32::MAX;
                    quantifier = true;
                },
                b'+' => {
                    p = &p[1..];
                    qmin = 1;
                    qmax = i32::MAX;
                    quantifier = true;
                },
                b'?' => {
                    p = &p[1..];
                    qmin = 0;
                    qmax = 1;
                    quantifier = true;
                },
                b'{' => {
                    let mut p1 = p;

                    if !p[1].is_ascii_digit(){
                        if self.is_utf16{
                            self.parse_error("invalid repetition count", p);
                            return -1;
                        }

                    } else{
                        p = &p[1..];

                        qmin = self.parse_digits(&mut p, true);
                        qmax = qmin;

                        if p[0] == b',' {
                            p = &p[1..];
                            if p[0].is_ascii_digit(){
                                qmax = self.parse_digits(&mut p, true);

                                if qmax < qmin{
                                    self.parse_error("invalid repetition count", p);
                                    return -1;
                                }
                            }
                            
                        }

                        // Annex B: normal atom if invalid '{' syntax
                        if p[0] != b'}' && !self.is_utf16{
                            p = p1;
                        } else if self.parse_expect(&mut p, '}' as u32) != 0{
                            return -1;
                        };

                        quantifier = true
                    };
                },
                _ => {}
            };

            if quantifier{
                let mut add_zero_advence_check = false;

                let mut greedy = true;

                if p[0] == b'?'{
                    p = &p[1..];
                    greedy = false;
                }

                if last_atom_start < 0{
                    self.parse_error("nothing to repeat", p);
                    return -1;
                }

                if greedy{
                    let mut len = 0;
                    let mut pos = 0;

                    if qmax > 0{
                        len = self.is_simple_quantifier(last_atom_start as usize);

                        if len > 0{
                            self.bytecode.push_u8(Op::Match as u8);

                            pos = last_atom_start;

                            let mut v = [0u8;17];
                            v[0] = Op::SimpleGreedyQuant as u8;

                            let v1:&mut [u32;4] = unsafe{std::mem::transmute_copy(&&mut v)};
                            v1[0] = self.bytecode.len() as u32 - last_atom_start as u32 - 17;
                            v1[1] = qmin as u32;
                            v1[2] = qmax as u32;
                            v1[3] = len as u32;

                            self.bytecode.insert_bytes(last_atom_start as usize, &v);

                            pos += 17;
                            self.pattern_ptr = p;
                            return 0;
                        }  
                    }
                    
                    add_zero_advence_check = self.check_advence(last_atom_start as usize) == 0;
                    
                } else{
                    add_zero_advence_check = false;
                };

                let mut len = 0;
                let mut pos = 0;

                len = self.bytecode.len() as i32 - last_atom_start;

                if qmin == 0{
                    if last_capture_count != self.capture_count{

                    }

                    if qmax == 0{
                        self.bytecode.resize(last_atom_start as usize, 0);

                    } else if qmax == 1{
                        let o = if greedy{
                            Op::SplitGotoFirst
                        } else{
                            Op::SplitNextFirst
                        };

                        self.bytecode.replace_u8(last_atom_start as usize, o as u8);
                        self.bytecode.replace_u32(last_atom_start as usize + 1, len as u32 + 5 + add_zero_advence_check as u32);

                    } else if qmax == i32::MAX{
                        let o = if greedy{
                            Op::SplitGotoFirst
                        } else{
                            Op::SplitNextFirst
                        };

                        self.bytecode.replace_u8(last_atom_start as usize, o as u8);
                        self.bytecode.replace_u32(last_atom_start as usize + 1, len as u32 + 5 + add_zero_advence_check as u32);

                        if add_zero_advence_check{
                            /* avoid infinite loop by stoping the
                               recursion if no advance was made in the
                               atom (only works if the atom has no
                               side effect) */

                            self.bytecode.replace_u8(last_atom_start as usize + 1 + 4, Op::PushCharPos as u8);

                            self.bytecode.push_u8(Op::BneCharPos as u8);
                            self.bytecode.push_u32(last_atom_start as u32);

                        } else{
                            self.bytecode.push_u8(Op::Goto as u8);
                            self.bytecode.push_u32(last_atom_start as u32);
                        };
                    } else{
                        let o = if greedy{
                            Op::SplitGotoFirst
                        } else{
                            Op::SplitNextFirst
                        };

                        pos = last_atom_start;

                        self.bytecode.replace_u8(pos as usize, Op::PushU32 as u8);
                        pos += 1;
                        self.bytecode.replace_u32(pos as usize, qmax as u32);
                        pos += 4;
                        self.bytecode.replace_u8(pos as usize, o as u8);
                        pos += 1;
                        self.bytecode.replace_u32(pos as usize, len as u32 +5);

                        self.bytecode.push_u8(Op::Loop as u8);
                        self.bytecode.push_u32(last_atom_start as u32 + 5);
                        self.bytecode.push_u8(Op::Drop as u8);
                    };

                } else if qmin == 1 && qmax == i32::MAX && !add_zero_advence_check{

                    if greedy{
                        self.bytecode.push_u8(Op::SplitGotoFirst as u8);
                    } else{
                        self.bytecode.push_u8(Op::SplitNextFirst as u8);
                    };

                    self.bytecode.push_u32(last_atom_start as u32);
                    
                } else{
                    if qmin == 1{
                        // nothing to add
                    } else{
                        self.bytecode.replace_u8(last_atom_start as usize, Op::PushU32 as u8);
                        self.bytecode.replace_u32(last_atom_start as usize + 1, qmin as u32);
                        last_atom_start += 5;
                        self.bytecode.push_u8(Op::Loop as u8);
                        self.bytecode.push_u32(last_atom_start as u32);

                        self.bytecode.push_u8(Op::Drop as u8);
                    };

                    if qmax == i32::MAX{
                        pos = self.bytecode.len() as i32;

                        let o = if greedy{
                            Op::SplitNextFirst
                        } else{
                            Op::SplitGotoFirst
                        };

                        self.bytecode.push_u8(o as u8);
                        self.bytecode.push_u32(len as u32 + 5 + add_zero_advence_check as u32);

                        if add_zero_advence_check{
                            self.bytecode.push_u8(Op::PushCharPos as u8)
                        }

                        let old = self.bytecode.len();
                        self.bytecode.resize(len as usize, 0);
                        self.bytecode.as_mut_bytes().copy_within(last_atom_start as usize..old, old);

                        if add_zero_advence_check{
                            self.bytecode.push_u8(Op::BneCharPos as u8);
                            self.bytecode.push_u32(pos as u32);
                        } else{
                            self.bytecode.push_u8(Op::Goto as u8);
                            self.bytecode.push_u32(pos as u32);

                        };
                    } else if qmax > qmin{
                        self.bytecode.push_u8(Op::PushU32 as u8);
                        self.bytecode.push_u32((qmax - qmin) as u32);
                        pos = self.bytecode.len() as i32;

                        let o = if greedy{
                            Op::SplitNextFirst
                        } else{
                            Op::SplitGotoFirst
                        };

                        self.bytecode.push_u8(o as u8);
                        self.bytecode.push_u32(len as u32 +5);

                        // copy the atom
                        let old = self.bytecode.len();
                        self.bytecode.resize(len as usize, 0);
                        self.bytecode.as_mut_bytes().copy_within(last_atom_start as usize..old, old);

                        self.bytecode.push_u8(Op::Loop as u8);
                        self.bytecode.push_u32(pos as u32);

                        self.bytecode.push_u8(Op::Drop as u8);
                    }
                }
                last_atom_start = -1;
            };           
        };
        // end quantifier

        // done
        self.pattern_ptr = p;
        return 0;
    }

    fn parse_alternative(&mut self, is_backward:bool) -> i32{
        let start = self.bytecode.len();

        loop{
            let p = self.pattern_ptr;
            if p.len() == 0{
                break;
            }

            if p[0] == b'|' || p[0] == b')'{
                break;
            }
            let term_start = self.bytecode.len();
            let ret = self.parse_term(is_backward);
            if ret != 0{
                return ret;
            }

            if is_backward{
                let end = self.bytecode.len();
                let term_size = end - term_start;
                self.bytecode.resize(end + term_size, 0);
                self.bytecode.as_mut_bytes().copy_within(start..end, start + term_size);
                self.bytecode.as_mut_bytes().copy_within(end..end + term_size, start);
            }
        };

        return 0;
    }

    pub fn parse_disjunction(&mut self, is_backward:bool) -> i32{
        let start = self.bytecode.len();

        if self.parse_alternative(is_backward) != 0{
            return -1;
        }

        while self.pattern_ptr[0] == b'|'{
            self.pattern_ptr = &self.pattern_ptr[1..];

            let mut len = self.bytecode.len() - start;
            self.bytecode.insert_u8(start, Op::SplitNextFirst as u8);
            self.bytecode.insert_u32(start+1, len as u32 +5);

            self.bytecode.push_u8(Op::Goto as u8);
            self.bytecode.push_u32(0);

            let pos = self.bytecode.len() - 4;

            if self.parse_alternative(is_backward) != 0{
                return -1;
            }

            len = self.bytecode.len() - (pos + 4);
            self.bytecode.replace_u32(pos, len as u32);
        };

        return 0;
    }

    pub fn compute_stack_size(&self) -> usize{
        let mut stack_size = 0;
        let mut stack_size_max = 0;

        let mut iter = self.bytecode.iter();

        loop{
            let op:Op = if let Some(p) = iter.get_next_u8(){
                unsafe{std::mem::transmute(p)}
            } else{
                break;
            };
        
            op.consume_iter(&mut iter);

            match op{
                Op::PushU32 |
                Op::PushCharPos => {
                    stack_size += 1;

                    if stack_size >  stack_size_max{
                        stack_size_max += 1;
                    }
                },
                Op::Drop |
                Op::BneCharPos => {
                    assert!(stack_size > 0);
                    stack_size -= 1;
                },
                _ => {}
            }
        };

        return stack_size_max
    }
}