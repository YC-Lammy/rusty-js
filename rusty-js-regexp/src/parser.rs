use super::ast::*;
pub use regex::disjunction as parse;

pub struct CharSlice(pub Vec<char>);

impl peg::Parse for CharSlice {
    type PositionRepr = <[char] as peg::Parse>::PositionRepr;
    fn start<'input>(&'input self) -> usize {
        self.0.start()
    }

    fn is_eof<'input>(&'input self, p: usize) -> bool {
        self.0.is_eof(p)
    }

    fn position_repr<'input>(&'input self, p: usize) -> Self::PositionRepr {
        self.0.position_repr(p)
    }
}

impl<'input> peg::ParseElem<'input> for CharSlice {
    type Element = <[char] as peg::ParseElem<'input>>::Element;
    fn parse_elem(&'input self, pos: usize) -> peg::RuleResult<Self::Element> {
        self.0.parse_elem(pos)
    }
}

impl peg::ParseLiteral for CharSlice {
    fn parse_string_literal(&self, pos: usize, literal: &str) -> peg::RuleResult<()> {
        let l = literal.len();
        let lit = literal.chars().collect::<Vec<char>>();
        if self.0.len() >= pos + l && &self.0[pos..pos + l] == &lit {
            peg::RuleResult::Matched(pos + l, ())
        } else {
            peg::RuleResult::Failed
        }
    }
}

impl<'input> peg::ParseSlice<'input> for CharSlice {
    type Slice = <[char] as peg::ParseSlice<'input>>::Slice;
    fn parse_slice(&'input self, p1: usize, p2: usize) -> Self::Slice {
        self.0.parse_slice(p1, p2)
    }
}

// the N param is get by position!()
peg::parser!(
    grammar regex() for CharSlice {
        rule _()
        = quiet!{ [' ' | '\u{0009}' | '\u{000B}' | '\u{000C}' | '\u{FEFF}']* }

        pub rule disjunction(unicode:bool, gs:&mut Vec<Option<GroupSpecifier>>) -> Disjunction
        = alts:(_ alt:alternative(unicode, gs) _ {alt})**['|'] {Disjunction(alts)}

        rule alternative(unicode:bool, gs:&mut Vec<Option<GroupSpecifier>>) -> Alternative
        = terms:(terms:(_ t:term(unicode, gs) _ {t})* {terms})? {
            if terms.is_none(){
                Alternative(Vec::new())
            } else{
                Alternative(terms.unwrap())
            }
        }

        rule term(unicode:bool, gs:&mut Vec<Option<GroupSpecifier>>) -> Term
        = a:assertion(unicode, gs) {
            Term::Assertion(a)
        }
        / before:({gs.len()}) a:atom(unicode, gs) _ q:quantifier()?{
            if q.is_some(){
                Term::AtomQuantifier{
                    paren_before:before,
                    atom:a,
                    quantifier:q.unwrap()
                }
            } else{
                Term::Atom(a)
            }
        }

        rule assertion(u:bool, gs:&mut Vec<Option<GroupSpecifier>>) -> Assertion
        = "^" {Assertion::BegAnchor}
        / "$" {Assertion::EndAnchor}
        / "\\" _ "b" {Assertion::WordAnchor}
        / "\\" _ "B" {Assertion::NonWordAnchor}
        / "(" _ "?" _ op:$("=" / "!" / "<=" / "<!") _ d:disjunction(u, gs) _ ")" {
            if op == &['=']{
                Assertion::PosLookahead(d)
            } else if op == &['!']{
                Assertion::NegLookahead(d)
            } else if op == &['<', '=']{
                Assertion::PosLookbehind(d)
            } else if op == &['<', '!']{
                Assertion::NegLookbehind(d)
            } else{
                unreachable!()
            }
        }

        rule quantifier() -> Quantifier
        = q:quantifier_prefex() _ opt:$("?")? {
            if opt.is_some(){
                Quantifier::QuantifierNonGreedy(q)
            } else{
                Quantifier::Quantifier(q)
            }
        }

        rule quantifier_prefex() -> QuantifierPrefix
        = "*" {QuantifierPrefix::Star}
        / "+" {QuantifierPrefix::Plus}
        / "?" {QuantifierPrefix::Opt}
        / "{" _ d1:decimal_digits() _ comma:$(",")? _ d2:decimal_digits()? _ "}" {
            if comma.is_some() && d2.is_some(){
                QuantifierPrefix::Loop3(d1, d2.unwrap())
            } else if comma.is_some(){
                QuantifierPrefix::Loop2(d1)
            } else{
                QuantifierPrefix::Loop1(d1)
            }
        }

        rule atom(u:bool, gs:&mut Vec<Option<GroupSpecifier>>) -> Atom
        = p:pattern_character() {Atom::PatternChar(p)}
        / "." {Atom::Dot}
        / "\\" _ a:atom_escape(u) {Atom::Escape(a)}
        / c:character_class(u) {Atom::CharClass(c)}
        / s:({gs.len()}) "(" _ g:group_specifier(u, gs)? _ d:disjunction(u, gs) _ ")" {
            gs.push(g.clone());
            Atom::Group { 
                parent_before:s,
                paren_within:gs.len() - s,
                group: g, 
                disjunction: d 
            }
        }
        / s:({gs.len()}) "(" _ "?" _ ":" _ d:disjunction(u, gs) _ ")" {
            Atom::NonCaptGroup{
                paren_within:gs.len() -s,
                disjunction:d,
            }
        }

        rule syntax_character() -> SyntaxCharacter
        = c:$(['^' | '$' | '\\' | '.' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '|']) {
            SyntaxCharacter(c[0])
        }

        rule pattern_character() -> PatternCharacter
        = c:$([_]) {?{
            let c = c[0];
            if c=='^' || c=='$' || c=='\\' || c=='.' || c=='*' || c=='+' || c=='('||c==')'||c=='['||c==']'||c=='{'||c=='}'||c=='|'{
                Err("PatternCharacter must not be SyntaxCharacter")
            } else{
                Ok(PatternCharacter(c))
            }
        }}

        rule atom_escape(u:bool) -> AtomEscape
        = d:decimal_escape() {AtomEscape::Decimal(d)}
        / c:character_class_escape(u) {AtomEscape::CharacterClass(c)}
        / c:character_escape(u) {AtomEscape::Character(c)}
        / N:position!() "k" _ g:group_name(u) {?
            if N == 0{
                Err("AtomEscape with k GroupName must have a position larger then zero.")
            } else{
                Ok(AtomEscape::GroupName(g))
            }
        }

        rule character_escape(u:bool) -> CharacterEscape
        = c:control_escape() {CharacterEscape::Control(c)}
        / "c" c:$(['a'..='z' | 'A'..='Z']) {CharacterEscape::AsciiLetter(c[0])}
        / "0" ds:decimal_digit()* {
            if ds.len() == 0{
                CharacterEscape::DecimalDigits(None)
            } else{
                CharacterEscape::DecimalDigits(Some(DecimalDigits(ds)))
            }
        }
        / h:hex_escape_sequence() {CharacterEscape::HexEscapeSequence(h)}
        / u:unicode_escape_sequence(u) {CharacterEscape::UnicodeEscapeSequence(u)}
        / i:identity_escape(u) {CharacterEscape::IdentityEscape(i)}

        rule control_escape() -> ControlEscape
        = "f" {ControlEscape::f}
        / "n" {ControlEscape::n}
        / "r" {ControlEscape::r}
        / "t" {ControlEscape::t}
        / "v" {ControlEscape::v}

        rule group_specifier(u:bool, gs:&mut Vec<Option<GroupSpecifier>>) -> GroupSpecifier
        = s:({gs.len()}) "?" g:group_name(u) {
            GroupSpecifier{
                paren_before:s,
                name:g.0,
            }
        }

        rule group_name(u:bool) -> GroupName
        = "<" _ i:identifier_name(u) _ ">" {GroupName(i)}

        rule identifier_name(u:bool) -> IdentifierName
        = start:identifier_start(u) part:identifier_parts(u){
            IdentifierName{
                code_points:Vec::new(),
                start:start,
                parts:part
            }
        }

        pub rule identifier_start(u:bool) -> IdentifierStart
        = "$" {IdentifierStart::Char('$')}
        / "_" {IdentifierStart::Char('_')}
        /  c:($[_]) {?{
            let c = c[0];
            if unicode_id_start::is_id_start(c){
                Ok(IdentifierStart::Char(c))
            } else{
                Err("invalide identifier")
            }
        }}
        / lead:unicode_lead_surrogate() trail:unicode_trail_surrogate() {?
            if u{
                Err("surrogate not avaliable in unicode mode")
            } else{
                Ok(IdentifierStart::UnicodeLeadTrailSurrogate { lead, trail})
            }
        }
        / "\\" u:unicode_escape_sequence(u) {IdentifierStart::UnicodeEscapeSequence(u)}

        pub rule identifier_parts(u:bool) -> Vec<IdentifierPart>
        = p:identifier_part(u)* {p}

        rule identifier_part(u:bool) -> IdentifierPart
        = "$" {IdentifierPart::Char('$')}
        / "\u{200C}" {IdentifierPart::ZWNJ}
        / "\u{200D}" {IdentifierPart::ZWJ}
        / c:$([_]) {?{
            let c = c[0];
            if unicode_id_start::is_id_continue(c){
                Ok(IdentifierPart::Char(c))
            } else{
                Err("invalid identifier")
            }
        }}
        / lead:unicode_lead_surrogate() trail:unicode_trail_surrogate() {?
            if u{
                Err("surrogate not avaliable in unicode mode")
            } else{
                Ok(IdentifierPart::UnicodeLeadTrailSurrogate { lead, trail})
            }
        }
        / "\\" u:unicode_escape_sequence(u) {IdentifierPart::UnicodeEscapeSequence(u)}

        rule unicode_escape_sequence(u:bool) -> UnicodeEscapeSequence
        = "u" lead:hex_lead_surrogate() trail:("\\u" trail:hex_trail_surrogate(){trail})? {
            ?if !u{
                // not unicode
                Err("")
            } else{
                if trail.is_some(){
                    Ok(UnicodeEscapeSequence::LeadTrail { lead, trail:trail.unwrap()})
                } else{
                    Ok(UnicodeEscapeSequence::Lead(lead))
                }
            }
        }
        / "u" trail:hex_trail_surrogate() {
            ?if !u{
                Err("")
            } else{
                Ok(UnicodeEscapeSequence::Trail(trail))
            }
        }
        / "u" h:hex_non_surrogate() {?
            if !u{
                Err("")
            } else{
                Ok(UnicodeEscapeSequence::Non(h))
            }
        }
        / "u" h:hex4digits() {?
            if u{
                // not unicode mode
                Err("")
            } else{
                Ok(UnicodeEscapeSequence::Hex4Digits(h))
            }
        }
        / "u{" ds:hex_digit()* "}" {?
            if !u{
                Err("")
            } else{
                Ok(UnicodeEscapeSequence::CodePoint(ds))
            }
        }

        rule unicode_lead_surrogate() -> UnicodeLeadSurrogate
        = "\\u" a:$(['0'..='9' | 'a'..='f' | 'A'..='F']['0'..='9' | 'a'..='f' | 'A'..='F']['0'..='9' | 'a'..='f' | 'A'..='F']['0'..='9' | 'a'..='f' | 'A'..='F']){
            ?{
                let mut v = 0;
                for i in a{
                    let n = match *i{
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
                        'a' |'A' => 10,
                        'b'|'B' => 11,
                        'c'|'C' => 12,
                        'd'|'D' => 13,
                        'e'|'E' => 14,
                        'f'|'F' => 15,
                        _ => unreachable!()
                    };
                    v = v*16 + n;
                };
                if v < 0xD800 || v > 0xDBFF{
                    Err("UnicodeLeadSurrogate must be in the range U+D800 to U+DBFF")
                } else{
                    Ok(UnicodeLeadSurrogate(v))
                }
            }
        }

        rule unicode_trail_surrogate() -> UnicodeTrailSurrogate
        = "\\u" a:$(['0'..='9' | 'a'..='f' | 'A'..='F']['0'..='9' | 'a'..='f' | 'A'..='F']['0'..='9' | 'a'..='f' | 'A'..='F']['0'..='9' | 'a'..='f' | 'A'..='F']){
            ?{
                let mut v = 0;
                for i in a{
                    let n = match *i{
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
                        'a' |'A' => 10,
                        'b'|'B' => 11,
                        'c'|'C' => 12,
                        'd'|'D' => 13,
                        'e'|'E' => 14,
                        'f'|'F' => 15,
                        _ => unreachable!()
                    };
                    v = v*16 + n;
                };
                if v < 0xDC00 || v > 0xDFFF{
                    Err("UnicodeTrailSurrogate must be in the range U+DC00 to U+DFFF")
                } else{
                    Ok(UnicodeTrailSurrogate(v))
                }
            }
        }

        rule hex_lead_surrogate() -> HexLeadSurrogate
        = a:$(['0'..='9' | 'a'..='f' | 'A'..='F']['0'..='9' | 'a'..='f' | 'A'..='F']['0'..='9' | 'a'..='f' | 'A'..='F']['0'..='9' | 'a'..='f' | 'A'..='F']){
            ?{
                let mut v = 0;
                for i in a{
                    let n = match *i{
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
                        'a' |'A' => 10,
                        'b'|'B' => 11,
                        'c'|'C' => 12,
                        'd'|'D' => 13,
                        'e'|'E' => 14,
                        'f'|'F' => 15,
                        _ => unreachable!()
                    };
                    v = v*16 + n;
                };
                if v < 0xD800 || v > 0xDBFF{
                    Err("HexLeadSurrogate must be in the range U+D800 to U+DBFF")
                } else{
                    Ok(HexLeadSurrogate(v))
                }
            }
        }

        rule hex_trail_surrogate() -> HexTrailSurrogate
        = a:$(['0'..='9' | 'a'..='f' | 'A'..='F']['0'..='9' | 'a'..='f' | 'A'..='F']['0'..='9' | 'a'..='f' | 'A'..='F']['0'..='9' | 'a'..='f' | 'A'..='F']){
            ?{
                let mut v = 0;
                for i in a{
                    let n = match *i{
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
                        'a' |'A' => 10,
                        'b'|'B' => 11,
                        'c'|'C' => 12,
                        'd'|'D' => 13,
                        'e'|'E' => 14,
                        'f'|'F' => 15,
                        _ => unreachable!()
                    };
                    v = v*16 + n;
                };
                if v < 0xDC00 || v > 0xDFFF{
                    Err("HexTrailSurrogate must be in the range U+DC00 to U+DFFF")
                } else{
                    Ok(HexTrailSurrogate(v))
                }
            }
        }

        rule hex_non_surrogate() -> HexNonSurrogate
        = a:$(['0'..='9' | 'a'..='f' | 'A'..='F']['0'..='9' | 'a'..='f' | 'A'..='F']['0'..='9' | 'a'..='f' | 'A'..='F']['0'..='9' | 'a'..='f' | 'A'..='F']){
            ?{
                let mut v = 0;
                for i in a{
                    let n = match *i{
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
                        'a' |'A' => 10,
                        'b'|'B' => 11,
                        'c'|'C' => 12,
                        'd'|'D' => 13,
                        'e'|'E' => 14,
                        'f'|'F' => 15,
                        _ => unreachable!()
                    };
                    v = v*16 + n;
                };
                if v >= 0xD800 && v <= 0xDFFF{
                    Err("HexNonSurrogate must not be in the range U+D800 to U+DFFF")
                } else{
                    Ok(HexNonSurrogate(v))
                }
            }
        }

        rule identity_escape(u:bool) -> IdentityEscape
        = s:syntax_character() {
            ?if !u{
                Err("")
            } else{
                Ok(IdentityEscape::SyntaxCharacter(s))
            }
        }
        / "/" {
            ?if !u{
                Err("")
            } else{
                Ok(IdentityEscape::Slash)
            }
        }
        / c:$([_]) {
            ?if u{
                Err("")
            } else{
                let c = c[0];
                if unicode_id_start::is_id_continue(c){
                    Err("")
                } else{
                    Ok(IdentityEscape::Char(c))
                }
            }
        }

        rule decimal_escape() -> DecimalEscape
        = n:$(['1'..='9']) d:decimal_digits()?{
            let n = match n[0]{
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
                _ => unreachable!()
            };

            DecimalEscape{
                non_zero:n,
                digits:d
            }
        }

        rule character_class_escape(u:bool) -> CharacterClassEscape
        = "d" {CharacterClassEscape::d}
        / "D" {CharacterClassEscape::D}
        / "s" {CharacterClassEscape::s}
        / "S" {CharacterClassEscape::S}
        / "w" {CharacterClassEscape::w}
        / "W" {CharacterClassEscape::W}
        / "p{" un:unicode_property_value_expression() "}" {
            ? if !u{
                Err("")
            } else{
                Ok(CharacterClassEscape::p(un))
            }
        }
        / "P{" un:unicode_property_value_expression() "}" {
            ? if !u{
                Err("")
            } else{
                Ok(CharacterClassEscape::P(un))
            }
        }

        rule unicode_property_value_expression() -> UnicodePropertyValueExpression
        = name:unicode_property_name() "=" value:unicode_property_value() {
            UnicodePropertyValueExpression::NameValue { name, value}
        }
        / u:unicode_property_value() {
            UnicodePropertyValueExpression::NameOrValue(u)
        }

        rule unicode_property_name() -> UnicodePropertyNameCharacters
        = c:$(['a'..='z' | 'A'..='Z' | '_']*) {
            UnicodePropertyNameCharacters(c.iter().collect::<String>())
        }

        rule unicode_property_value() -> UnicodePropertyValueCharacters
        = c:$(['a'..='z' | 'A'..='Z' | '_' | '0'..='9']*) {
            UnicodePropertyValueCharacters(c.iter().collect::<String>())
        }

        rule character_class(u:bool) -> CharacterClass
        = "[" n:$("^")? c:class_ranges(u) "]" {
            if n.is_some(){
                CharacterClass::NegClass(c)
            } else{
                CharacterClass::PosClass(c)
            }
        }

        rule class_ranges(u:bool) -> ClassRanges
        = n:non_empty_class_ranges(u) {
            ClassRanges::NonemptyClassRangesNoDash(n)
        }
        / {ClassRanges::Empty}

        rule non_empty_class_ranges(u:bool) -> NonEmptyClassRanges
        = c:class_atom(u) "-" c1:class_atom(u) r:class_ranges(u) {
            NonEmptyClassRanges::ClassCharRange { start: c, end: c1, ranges: Box::new(r) }
        }
        / c:class_atom(u) r:non_empty_class_ranges_no_dash(u)? {
            if r.is_some(){
                NonEmptyClassRanges::ClassCont { atom: c, ranges: Box::new(r.unwrap()) }
            } else{
                NonEmptyClassRanges::ClassAtom(c)
            }
        }

        rule non_empty_class_ranges_no_dash(u:bool) -> NonEmptyClassRangesNoDash
        = c:class_atom_no_dash(u) "-" c1:class_atom(u) r:class_ranges(u) {
            NonEmptyClassRangesNoDash::ClassCharRange { start: c, end: c1, ranges: Box::new(r) }
        }
        / c:class_atom_no_dash(u) r:non_empty_class_ranges_no_dash(u) {
            NonEmptyClassRangesNoDash::ClassCont { atom: c, ranges: Box::new(r) }
        }
        / c:class_atom(u) {
            NonEmptyClassRangesNoDash::ClassAtom(c)
        }

        rule class_atom(u:bool) -> ClassAtom
        = "-" {ClassAtom::Dash}
        / c:class_atom_no_dash(u) {ClassAtom::NoDash(c)}

        rule class_atom_no_dash(u:bool) -> ClassAtomNoDash
        = c:$([_]) {
            ?{
                let c = c[0];
                if c == '\\' || c == ']' || c=='-'{
                    Err("")
                } else{
                    Ok(ClassAtomNoDash::Char(c))
                }
            }
        }
        / "\\" c:class_escape(u) {
            ClassAtomNoDash::ClassEscape(c)
        }

        rule class_escape(u:bool) -> ClassEscape
        = "b" {ClassEscape::b}
        / "-" {
            ?if !u{
                Err("")
            } else{
                Ok(ClassEscape::Dash)
            }
        }
        / c:character_class_escape(u) {
            ClassEscape::CharacterClassEscape(c)
        }
        / c:character_escape(u) {
            ClassEscape::CharacterEscape(c)
        }

        rule decimal_digits() -> DecimalDigits
        = d:decimal_digit()* {DecimalDigits(d)}

        rule decimal_digit() -> DecimalDigit
        = d:$(['0'..='9']) {
            let n = match d[0]{
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
                _ => unreachable!()
            };
            DecimalDigit(n)
        }

        rule hex4digits() -> Hex4Digits
        = a:hex_digit() b:hex_digit() c:hex_digit() d:hex_digit() {
            Hex4Digits(a, b, c, d)
        }

        rule hex_digit() -> HexDigit
        = d:$(['0'..='9' | 'a'..='f' | 'A'..='F']) {
            HexDigit(d[0])
        }

        rule hex_escape_sequence() -> HexEscapeSequence
        = "x" a:hex_digit() b:hex_digit() {
            HexEscapeSequence(a, b)
        }
    }

);
