use std::{
    hash::{Hash, Hasher},
    str::EncodeUtf16,
};

pub struct Pattern{
    pub disjunction:Disjunction,
    pub groups:Vec<Option<GroupSpecifier>>
}

pub struct Disjunction(pub Vec<Alternative>);

pub struct Alternative(pub Vec<Term>);

pub enum Term {
    Assertion(Assertion),
    Atom(Atom),
    AtomQuantifier { 
        paren_before:usize,
        atom: Atom, 
        quantifier: Quantifier 
    },
}

pub enum Assertion {
    /// ^
    BegAnchor,
    /// $
    EndAnchor,
    /// \\b
    WordAnchor,
    /// \\B
    NonWordAnchor,
    /// (?=Disjunction)
    PosLookahead(Disjunction),
    /// (?!Disjunction)
    NegLookahead(Disjunction),
    /// (?<=Disjunction)
    PosLookbehind(Disjunction),
    /// (?<!Disjunction)
    NegLookbehind(Disjunction),
}

pub enum Quantifier {
    /// QuantifierPrefix
    Quantifier(QuantifierPrefix),
    /// QuantifierPrefix ?
    QuantifierNonGreedy(QuantifierPrefix),
}

pub enum QuantifierPrefix {
    /// *
    Star,
    /// +
    Plus,
    /// ?
    Opt,
    /// { DecimalDigits }
    Loop1(DecimalDigits),
    /// { DecimalDigits , }
    Loop2(DecimalDigits),
    /// { DecimalDigits, DecimalDigits}
    Loop3(DecimalDigits, DecimalDigits),
}

pub enum Atom {
    /// PatternCharacter
    PatternChar(PatternCharacter),
    /// .
    Dot,
    /// \\ AtomEscape
    Escape(AtomEscape),
    /// CharacterClass
    CharClass(CharacterClass),
    /// ( GroupSpecifier [?U] Disjunction )
    Group {
        parent_before:usize,
        paren_within:usize,
        group: Option<GroupSpecifier>,
        disjunction: Disjunction,
    },
    /// ( ?: Disjunction )
    NonCaptGroup{
        paren_within:usize,
        disjunction:Disjunction
    },
}

/// ^ $ \ . * + ? ( ) [ ] { } |
#[derive(Clone, PartialEq)]
pub struct SyntaxCharacter(pub char);

/// SourceCharacter but not SyntaxCharacter
pub struct PatternCharacter(pub char);

pub enum AtomEscape {
    /// DecimalEscape
    Decimal(DecimalEscape),
    /// CharacterClassEscape [?U]
    CharacterClass(CharacterClassEscape),
    /// CharacterEscape [?U]
    Character(CharacterEscape),
    /// [+N] k GroupName [?U]
    GroupName(GroupName),
}

pub enum CharacterEscape {
    /// ControlEscape
    Control(ControlEscape),
    /// c AsciiLetter
    AsciiLetter(char),
    /// 0 \[ lookahead ∉ DecimalDigit ]
    DecimalDigits(Option<DecimalDigits>),
    /// HexEscapeSequence
    HexEscapeSequence(HexEscapeSequence),
    /// UnicodeEscapeSequence[?U]
    UnicodeEscapeSequence(UnicodeEscapeSequence),
    /// IdentityEscape[?U]
    IdentityEscape(IdentityEscape),
}

#[allow(non_camel_case_types)]
#[repr(u8)]
pub enum ControlEscape {
    f = 'f' as u8,
    n = 'n' as u8,
    r = 'r' as u8,
    t = 't' as u8,
    v = 'v' as u8,
}

/// ? GroupName [?U]
#[derive(Clone)]
pub struct GroupSpecifier{
    pub paren_before:usize,
    pub name:IdentifierName,
}

/// < RegExpIdentifierName [?U] >
#[derive(Clone)]
pub struct GroupName(pub IdentifierName);

#[derive(Clone, PartialEq, Hash)]
pub struct IdentifierName {
    pub code_points: Vec<u32>,
    pub start: IdentifierStart,
    pub parts: Vec<IdentifierPart>,
}

#[derive(Clone, PartialEq, Hash)]
pub enum IdentifierStart {
    /// UnicodeIdStart or $ or _
    Char(char),
    /// / UnicodeEscapeSequence [+U]
    UnicodeEscapeSequence(UnicodeEscapeSequence),
    /// [~U] UnicodeLeadSurrogate UnicodeTrailSurrogate
    UnicodeLeadTrailSurrogate {
        lead: UnicodeLeadSurrogate,
        trail: UnicodeTrailSurrogate,
    },
}

#[derive(Clone, PartialEq, Hash)]
pub enum IdentifierPart {
    /// UnicodeIdContinue or $
    Char(char),
    /// <ZWNJ>
    ZWNJ,
    /// <ZWJ>
    ZWJ,
    /// / UnicodeEscapeSequence [+U]
    UnicodeEscapeSequence(UnicodeEscapeSequence),
    /// [~U] UnicodeLeadSurrogate UnicodeTrailSurrogate
    UnicodeLeadTrailSurrogate {
        lead: UnicodeLeadSurrogate,
        trail: UnicodeTrailSurrogate,
    },
}

#[derive(Clone, PartialEq, Hash)]
pub enum UnicodeEscapeSequence {
    /// [+U]u HexLeadSurrogate \u HexTrailSurrogate
    LeadTrail {
        lead: HexLeadSurrogate,
        trail: HexTrailSurrogate,
    },
    /// [+U]u HexLeadSurrogate
    Lead(HexLeadSurrogate),
    /// [+U]u HexTrailSurrogate
    Trail(HexTrailSurrogate),
    /// [+U]u HexNonSurrogate
    Non(HexNonSurrogate),
    /// [~U]u Hex4Digits
    Hex4Digits(Hex4Digits),
    /// [+U]u{CodePoint}
    CodePoint(Vec<HexDigit>),
}

#[derive(Clone, PartialEq, Hash)]
/// any unicode inclusive from 0xD800 to 0xDBFF
pub struct UnicodeLeadSurrogate(pub u16);

#[derive(Clone, PartialEq, Hash)]
/// any unicode inclusive from 0xDC00 to 0xDFFF
pub struct UnicodeTrailSurrogate(pub u16);

#[derive(Clone, PartialEq, Hash)]
/// MV of Hex4Digits is inclusive from 0xD800 to 0xDBFF
pub struct HexLeadSurrogate(pub u16);

#[derive(Clone, PartialEq, Hash)]
/// MV of Hex4Digits is inclusive from 0xDC00 to 0xDFFF
pub struct HexTrailSurrogate(pub u16);

#[derive(Clone, PartialEq, Hash)]
/// MV of Hex4Digits is inclusive from 0xD800 to 0xDFFF
pub struct HexNonSurrogate(pub u16);

#[derive(Clone, PartialEq)]
pub enum IdentityEscape {
    /// [+U] SyntaxCharacter
    SyntaxCharacter(SyntaxCharacter),
    /// [+U] /
    Slash,
    /// [~U] SourceCharacter but not UnicodeIDContinue
    Char(char),
}

#[derive(Clone, PartialEq)]
/// NonZeroDigit DecimalDigit sopt \[lookahead ∉ DecimalDigit]
pub struct DecimalEscape {
    pub non_zero: u8,
    pub digits: Option<DecimalDigits>,
}

#[derive(Clone, PartialEq)]
#[allow(non_camel_case_types)]
#[repr(u8)]
pub enum CharacterClassEscape {
    /// d
    d,
    /// D
    D,
    /// s
    s,
    /// S
    S,
    /// w
    w,
    /// W
    W,
    /// p{ UnicodePropertyValueExpression }
    p(UnicodePropertyValueExpression),
    /// P{ UnicodePropertyValueExpression }
    P(UnicodePropertyValueExpression),
}

#[derive(Clone, PartialEq)]
pub enum UnicodePropertyValueExpression {
    /// UnicodePropertyName = UnicodePropertyValue
    NameValue {
        name: UnicodePropertyNameCharacters,
        value: UnicodePropertyValueCharacters,
    },
    /// UnicodePropertyName
    NameOrValue(UnicodePropertyValueCharacters),
}

#[derive(Clone, PartialEq)]
/// (ContolLetter | _ )*
pub struct UnicodePropertyNameCharacters(pub String);

#[derive(Clone, PartialEq)]
/// (UnicodePropertyNameCharacter | DecimalDigit)*
pub struct UnicodePropertyValueCharacters(pub String);

pub enum CharacterClass {
    /// "[" [lookahead ≠ ^]ClassRanges[?U] "]"
    PosClass(ClassRanges),
    /// "[" ^ ClassRanges [?U] "]"
    NegClass(ClassRanges),
}

pub enum ClassRanges {
    ///
    Empty,
    /// NonemptyClassRanges[?U]
    NonemptyClassRangesNoDash(NonEmptyClassRanges),
}

pub enum NonEmptyClassRanges {
    /// ClassAtom - ClassAtom ClassRanges
    ClassCharRange {
        start: ClassAtom,
        end: ClassAtom,
        ranges: Box<ClassRanges>,
    },
    /// ClassAtom[?U] NonemptyClassRangesNoDash[?U]
    ClassCont {
        atom: ClassAtom,
        ranges: Box<NonEmptyClassRangesNoDash>,
    },
    /// ClassAtom[?U]
    ClassAtom(ClassAtom),
}

pub enum NonEmptyClassRangesNoDash {
    /// ClassAtomNoDash - ClassAtom ClassRanges
    ClassCharRange {
        start: ClassAtomNoDash,
        end: ClassAtom,
        ranges: Box<ClassRanges>,
    },
    /// ClassAtomNoDash[?U] NonemptyClassRangesNoDash[?U]
    ClassCont {
        atom: ClassAtomNoDash,
        ranges: Box<NonEmptyClassRangesNoDash>,
    },
    /// ClassAtom[?U]
    ClassAtom(ClassAtom),
}

pub enum ClassAtom {
    /// -
    Dash,
    /// ClassAtomNoDash
    NoDash(ClassAtomNoDash),
}

pub enum ClassAtomNoDash {
    /// not \ or ] or -
    Char(char),
    /// \ ClassEscape[?U]
    ClassEscape(ClassEscape),
}

#[allow(non_camel_case_types)]
pub enum ClassEscape {
    /// b
    b,
    /// [+U] -
    Dash,
    /// CharacterClassEscape [?U]
    CharacterClassEscape(CharacterClassEscape),
    /// CharacterEscape [?U]
    CharacterEscape(CharacterEscape),
    /// singal digit 0-7 range 0-255
    OctalDigit(u8),
}

#[derive(Clone, PartialEq)]
/// \[DecimalDigit]
pub struct DecimalDigits(pub Vec<DecimalDigit>);

#[derive(Clone, PartialEq)]
pub struct DecimalDigit(pub u8);

/// x HexDigit HexDigit
pub struct HexEscapeSequence(pub HexDigit, pub HexDigit);

/// DecimalDigit a b c d e f A B C D E F
#[derive(Clone, PartialEq, Hash)]
pub struct HexDigit(pub char);

#[derive(Clone, PartialEq, Hash)]
pub struct Hex4Digits(pub HexDigit, pub HexDigit, pub HexDigit, pub HexDigit);

impl HexDigit {
    pub fn MV(&self) -> i32 {
        match self.0 {
            'a' | 'A' => 10,
            'b' | 'B' => 11,
            'c' | 'C' => 12,
            'd' | 'D' => 13,
            'e' | 'E' => 14,
            'f' | 'F' => 15,
            '0' => 0,
            '1' => 1,
            '2' => 2,
            '3' => 3,
            '4' => 4,
            '5' => 5,
            '6' => 6,
            '7' => 7,
            '8' => 8,
            '9' => 9,
            _ => unreachable!(),
        }
    }
}

impl Hex4Digits {
    pub fn MV(&self) -> i32 {
        self.0.MV() * 16 * 16 * 16 + self.1.MV() * 16 * 16 + self.2.MV() * 16 + self.3.MV()
    }
}

impl HexEscapeSequence {
    pub fn MV(&self) -> i32 {
        self.0.MV() * 16 + self.1.MV()
    }
}

impl DecimalDigit {
    pub fn MV(&self) -> i32 {
        self.0 as i32
    }
}

impl DecimalDigits {
    pub fn MV(&self) -> i32 {
        let mut i = 0;
        for d in &self.0 {
            i = i * 10 + d.MV()
        }
        return i;
    }
}

impl Disjunction {
    
}

impl Atom{
    pub fn paren_within(&self) -> usize{
        match self{
            Self::Group { paren_within, .. } => *paren_within,
            Self::NonCaptGroup { paren_within, .. } => *paren_within,
            _ => 0
        }
    }
}

impl DecimalEscape {
    pub fn CapturingGroupNumber(&self) -> i32 {
        if let Some(d) = &self.digits {
            let n = d.0.len();
            self.non_zero as i32 * 10i32.pow(n as u32) + d.MV()
        } else {
            self.non_zero as i32
        }
    }
}

impl ClassAtom {
    /// https://tc39.es/ecma262/#sec-patterns-static-semantics-is-character-class
    pub fn IsCharacterClass(&self) -> bool {
        match self {
            ClassAtom::Dash => false,
            ClassAtom::NoDash(n) => n.IsCharacterClass(),
        }
    }

    /// https://tc39.es/ecma262/#sec-patterns-static-semantics-character-value
    pub fn CharacterValue(&self) -> u32 {
        match self {
            ClassAtom::Dash => 0x0002,
            ClassAtom::NoDash(n) => n.CharacterValue(),
        }
    }
}

impl ClassAtomNoDash {
    /// https://tc39.es/ecma262/#sec-patterns-static-semantics-is-character-class
    pub fn IsCharacterClass(&self) -> bool {
        match self {
            ClassAtomNoDash::Char(_) => false,
            ClassAtomNoDash::ClassEscape(c) => c.IsCharacterClass(),
        }
    }

    /// https://tc39.es/ecma262/#sec-patterns-static-semantics-character-value
    pub fn CharacterValue(&self) -> u32 {
        match self {
            ClassAtomNoDash::Char(c) => *c as u32,
            ClassAtomNoDash::ClassEscape(c) => c.CharacterValue(),
        }
    }
}

impl ClassEscape {
    /// https://tc39.es/ecma262/#sec-patterns-static-semantics-is-character-class
    pub fn IsCharacterClass(&self) -> bool {
        match self {
            ClassEscape::b | ClassEscape::Dash => false,
            ClassEscape::OctalDigit(_) => false,
            ClassEscape::CharacterEscape(_) => false,
            ClassEscape::CharacterClassEscape(_) => true,
        }
    }

    /// https://tc39.es/ecma262/#sec-patterns-static-semantics-character-value
    pub fn CharacterValue(&self) -> u32 {
        match self {
            ClassEscape::b => 0x0008,
            ClassEscape::Dash => 0x0002,
            ClassEscape::CharacterEscape(c) => c.CharacterValue(),
            ClassEscape::CharacterClassEscape(c) => {
                unimplemented!()
            }
            ClassEscape::OctalDigit(o) => todo!(),
        }
    }
}

impl CharacterEscape {
    /// https://tc39.es/ecma262/#sec-patterns-static-semantics-character-value
    pub fn CharacterValue(&self) -> u32 {
        match self {
            CharacterEscape::Control(c) => match c {
                ControlEscape::t => 9,
                ControlEscape::n => 10,
                ControlEscape::v => 11,
                ControlEscape::f => 12,
                ControlEscape::r => 13,
            },
            CharacterEscape::AsciiLetter(c) => *c as u32 % 32,
            CharacterEscape::DecimalDigits(_) => 0,
            CharacterEscape::HexEscapeSequence(h) => h.MV() as u32,
            CharacterEscape::UnicodeEscapeSequence(u) => u.CharacterValue(),
            CharacterEscape::IdentityEscape(i) => i.CharacterValue(),
        }
    }
}

impl CharacterClassEscape {}

impl UnicodeEscapeSequence {
    /// https://tc39.es/ecma262/#sec-patterns-static-semantics-character-value
    pub fn CharacterValue(&self) -> u32 {
        match self {
            UnicodeEscapeSequence::LeadTrail { lead, trail } => {
                let lead = lead.CharacterValue() as u16;
                let trail = trail.CharacterValue() as u16;
                UTF16SurrogatePairToCodePoint(lead, trail).unwrap()
            }
            UnicodeEscapeSequence::Hex4Digits(h) => h.MV() as u32,
            UnicodeEscapeSequence::CodePoint(c) => {
                let mut i = 0;
                for d in c {
                    i = i * 16 + d.MV() as u32;
                }
                return i;
            }
            UnicodeEscapeSequence::Lead(l) => l.CharacterValue(),
            UnicodeEscapeSequence::Trail(t) => t.CharacterValue(),
            UnicodeEscapeSequence::Non(n) => n.CharacterValue(),
        }
    }
}

impl HexLeadSurrogate {
    /// https://tc39.es/ecma262/#sec-patterns-static-semantics-character-value
    pub fn CharacterValue(&self) -> u32 {
        self.0 as u32
    }
}

impl HexTrailSurrogate {
    /// https://tc39.es/ecma262/#sec-patterns-static-semantics-character-value
    pub fn CharacterValue(&self) -> u32 {
        self.0 as u32
    }
}

impl HexNonSurrogate {
    /// https://tc39.es/ecma262/#sec-patterns-static-semantics-character-value
    pub fn CharacterValue(&self) -> u32 {
        self.0 as u32
    }
}

impl IdentityEscape {
    /// https://tc39.es/ecma262/#sec-patterns-static-semantics-character-value
    pub fn CharacterValue(&self) -> u32 {
        match self {
            IdentityEscape::Char(c) => *c as u32,
            IdentityEscape::Slash => '/' as u32,
            IdentityEscape::SyntaxCharacter(c) => c.0 as u32,
        }
    }
}

impl GroupName {
    // https://tc39.es/ecma262/#sec-static-semantics-capturinggroupname
    //pub fn CapturingGroupName(&mut self) -> &str{
    //let points = self.0.RegExpIdentifierCodePoints();

    //}
}

impl IdentifierName {
    pub fn RegExpIdentifierCodePoints(&self) -> &[u32] {
        if self.code_points.len() > 0 {
            return &self.code_points;
        }
        let mut v = vec![self.start.RegExpIdentifierCodePoint()];
        for p in &self.parts {
            v.push(p.RegExpIdentifierCodePoint());
        }

        unsafe {
            (self as *const Self as *mut Self)
                .as_mut()
                .unwrap()
                .code_points = v;
        }
        return &self.code_points;
    }
}

impl IdentifierStart {
    pub fn RegExpIdentifierCodePoint(&self) -> u32 {
        match self {
            IdentifierStart::Char(c) => *c as u32,
            IdentifierStart::UnicodeEscapeSequence(u) => u.CharacterValue(),
            IdentifierStart::UnicodeLeadTrailSurrogate { lead, trail } => {
                let lead = lead.0;
                let trail = trail.0;
                UTF16SurrogatePairToCodePoint(lead, trail).unwrap()
            }
        }
    }
}

impl IdentifierPart {
    pub fn RegExpIdentifierCodePoint(&self) -> u32 {
        match self {
            IdentifierPart::Char(c) => *c as u32,
            IdentifierPart::UnicodeEscapeSequence(u) => u.CharacterValue(),
            IdentifierPart::ZWNJ => 0x200C,
            IdentifierPart::ZWJ => 0x200D,
            IdentifierPart::UnicodeLeadTrailSurrogate { lead, trail } => {
                let lead = lead.0;
                let trail = trail.0;
                UTF16SurrogatePairToCodePoint(lead, trail).unwrap()
            }
        }
    }
}

pub fn UTF16SurrogatePairToCodePoint(lead: u16, trail: u16) -> Option<u32> {
    if lead < 0xD800 || lead > 0xDBFF {
        return None;
    }
    if trail < 0xDC00 || trail > 0xDFFF {
        return None;
    }

    Some((lead as u32 - 0xD800) * 0x400 + (trail as u32 - 0xDC00) + 0x10000)
}
