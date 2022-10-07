use std::collections::{HashMap, VecDeque};
use std::ops::Range;
use std::sync::Arc;

use array_tool::vec::Union;

use super::ast::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Backward,
}

impl Direction {
    pub fn is_forward(self) -> bool {
        self == Self::Forward
    }

    pub fn is_backward(self) -> bool {
        self == Self::Backward
    }
}

#[derive(Clone)]
pub struct State {
    input: &'static [u16],
    pub start_index: usize,
    pub end_index: usize,
    pub captures: Vec<Option<Range<usize>>>,
}

pub type MatchResult = Result<State, ()>;
pub type Matcher = Arc<dyn Fn(State, Cont) -> MatchResult>;
type Cont = Arc<dyn Fn(State) -> MatchResult + 'static>;
pub type RegExpMatcher = Arc<dyn Fn(&[u16], usize) -> Result<State, ()>>;

#[derive(Clone, Copy)]
pub struct RegExpRecord {
    pub ignore_case: bool,
    pub multiline: bool,
    pub dot_all: bool,
    pub unicode: bool,
    pub capturing_groups_count: usize,
}

pub fn create_matcher(
    pattern:&Pattern,
    record: RegExpRecord,
) -> Arc<dyn Fn(&[u16], usize) -> Result<State, ()>> {
    let m = pattern.disjunction.create_matcher(&pattern, record, Direction::Forward);

    Arc::new(move |input, index| {
        if index > input.len() {
            return Err(());
        }

        let mut cap = Vec::with_capacity(record.capturing_groups_count);
        cap.resize(record.capturing_groups_count, None);

        let state = State {
            input: unsafe { std::mem::transmute_copy(&input) },
            start_index: index,
            end_index: index,
            captures: cap,
        };

        m(state, Arc::new(|state: State| Ok(state)))
    })
}

impl Disjunction {
    pub fn create_matcher(
        &self,
        top: &Pattern,
        record: RegExpRecord,
        direction: Direction,
    ) -> Arc<dyn Fn(State, Arc<dyn Fn(State) -> MatchResult>) -> MatchResult> {
        let mut matchers = Vec::new();

        for i in &self.0 {
            matchers.push(i.create_matcher(top, record, direction));
        }

        Arc::new(move |state, cont| {
            let mut err;
            for m in &matchers {
                let r = m(state.clone(), cont.clone());
                match r {
                    Ok(v) => return Ok(v),
                    Err(e) => err = e,
                }
            }
            return Err(());
        })
    }
}

impl Alternative {
    pub fn create_matcher(
        &self,
        top: &Pattern,
        record: RegExpRecord,
        direction: Direction,
    ) -> Arc<dyn Fn(State, Arc<dyn Fn(State) -> MatchResult>) -> MatchResult> {
        if self.0.len() == 0 {
            return Arc::new(|state, cont| cont(state));
        } else {
            let mut matchers = VecDeque::new();

            if direction.is_forward() {
                for i in &self.0 {
                    matchers.push_front(i.create_matcher(top, record, direction));
                }

                let m1 = matchers.pop_back().unwrap();

                return Arc::new(move |state, cont: Cont| {
                    let mut d: Cont = cont;

                    for i in &matchers {
                        let r: Cont = d.clone();
                        let i = i.clone();

                        d = Arc::new(move |state| (i)(state, r.clone()));
                    }

                    m1(state, d)
                });
            } else {
                for i in &self.0 {
                    matchers.push_back(i.create_matcher(top, record, direction));
                }

                let m2 = matchers.pop_back().unwrap();

                return Arc::new(move |state, cont| {
                    let mut d: Cont = Arc::new(move |state| cont(state));

                    for i in &matchers {
                        let r = d.clone();
                        let i = i.clone();

                        d = Arc::new(move |state| i(state, r.clone()));
                    }

                    m2(state, d)
                });
            }
        }
    }
}

impl Term {
    pub fn create_matcher(
        &self,
        top: &Pattern,
        record: RegExpRecord,
        direction: Direction,
    ) -> Arc<dyn Fn(State, Arc<dyn Fn(State) -> MatchResult>) -> MatchResult> {
        match self {
            Term::Assertion(a) => a.create_matcher(top, record),
            Term::Atom(a) => a.create_matcher(top, record, direction),
            Term::AtomQuantifier { paren_before, atom, quantifier } => {
                let m = atom.create_matcher(top, record, direction);
                let (min, max, greedy) = quantifier.compile();

                let paren_index = *paren_before;
                let paren_count = atom.paren_within();

                return Arc::new(move |state, cont| {
                    RepeatMatcher(
                        m.clone(),
                        min,
                        max,
                        greedy,
                        state,
                        cont,
                        paren_index,
                        paren_count,
                    )
                });
            }
        }
    }
}

impl Assertion {
    pub fn create_matcher(&self, top: &Pattern, record: RegExpRecord) -> Matcher {
        match self {
            Assertion::BegAnchor => {
                return Arc::new(move |state, cont| {
                    let e = state.end_index;
                    if state.end_index == 0 {
                        cont(state)

                        // line termination
                    } else if record.multiline
                        && (state.input[e - 1] == 0x000A
                            || state.input[e - 1] == 0x000D
                            || state.input[e - 1] == 0x2028
                            || state.input[e - 1] == 0x2029)
                    {
                        cont(state)
                    } else {
                        Err(())
                    }
                });
            }
            Assertion::EndAnchor => {
                return Arc::new(move |state, cont| {
                    let e = state.end_index;
                    let input_length = state.input.len();

                    if e == input_length {
                        cont(state)
                    } else if record.multiline
                        && (state.input[e] == 0x000A
                            || state.input[e] == 0x000D
                            || state.input[e] == 0x2028
                            || state.input[e] == 0x2029)
                    {
                        cont(state)
                    } else {
                        Err(())
                    }
                })
            }
            Assertion::WordAnchor => {
                return Arc::new(move |state, cont| {
                    let e = state.end_index;
                    let a = IsWordChar(record, state.input, e as i64 - 1);
                    let b = IsWordChar(record, state.input, e as i64);
                    if a && b {
                        Err(())
                    } else {
                        cont(state)
                    }
                });
            }
            Assertion::NonWordAnchor => {
                return Arc::new(move |state, cont| {
                    let e = state.end_index;
                    let a = IsWordChar(record, state.input, e as i64 - 1);
                    let b = IsWordChar(record, state.input, e as i64);
                    if a != b {
                        Err(())
                    } else {
                        cont(state)
                    }
                });
            }
            Assertion::PosLookahead(d) => {
                let m = d.create_matcher(top, record, Direction::Forward);
                return Arc::new(move |state, cont| {
                    let d: Cont = Arc::new(move |s| Ok(s));
                    let input = state.input;
                    let e = state.end_index;
                    let y = m(state, d)?;

                    let z = State {
                        input: input,
                        start_index: y.start_index,
                        end_index: e,
                        captures: y.captures,
                    };
                    cont(z)
                });
            }
            Assertion::NegLookahead(d) => {
                let m = d.create_matcher(top, record, Direction::Forward);
                return Arc::new(move |state, cont| {
                    let d: Cont = Arc::new(|s| Ok(s));
                    let r = m(state.clone(), d);
                    if r.is_ok() {
                        return Err(());
                    }
                    return cont(state);
                });
            }
            Assertion::PosLookbehind(d) => {
                let m = d.create_matcher(top, record, Direction::Backward);
                return Arc::new(move |state, cont| {
                    let d: Cont = Arc::new(|s| Ok(s));

                    let input = state.input;
                    let e = state.end_index;
                    let y = m(state, d)?;

                    let z = State {
                        input: input,
                        start_index: y.start_index,
                        end_index: e,
                        captures: y.captures,
                    };
                    return cont(z);
                });
            }
            Assertion::NegLookbehind(d) => {
                let m = d.create_matcher(top, record, Direction::Backward);
                return Arc::new(move |state, cont| {
                    let d: Cont = Arc::new(|s| Ok(s));
                    let r = m(state.clone(), d);
                    if r.is_ok() {
                        return Err(());
                    }
                    return cont(state);
                });
            }
        }
    }
}

impl Quantifier {
    pub fn compile(&self) -> (f64, f64, bool) {
        match self {
            Quantifier::Quantifier(q) => {
                let (min, max) = q.compile();
                (min, max, true)
            }
            Quantifier::QuantifierNonGreedy(q) => {
                let (min, max) = q.compile();
                (min, max, false)
            }
        }
    }
}

impl QuantifierPrefix {
    pub fn compile(&self) -> (f64, f64) {
        match self {
            Self::Star => (0.0, f64::INFINITY),
            Self::Plus => (1.0, f64::INFINITY),
            Self::Opt => (0.0, 1.0),
            Self::Loop1(d) => {
                let i = d.MV() as f64;
                (i, i)
            }
            Self::Loop2(d) => {
                let i = d.MV() as f64;
                (i, f64::INFINITY)
            }
            Self::Loop3(d, d1) => {
                let i = d.MV() as f64;
                let j = d1.MV() as f64;
                (i, j)
            }
        }
    }
}

impl Atom {
    pub fn create_matcher(
        &self,
        top: &Pattern,
        record: RegExpRecord,
        direction: Direction,
    ) -> Matcher {
        match self {
            Atom::PatternChar(c) => CharacterSetMatcherVec(record, vec![c.0], false, direction),
            Atom::Dot => {
                if record.dot_all {
                    // Remove all characters corresponding to a code point on the right-hand side of the LineTerminator production.
                };
                CharacterSetMatcherRange(record, '\0'..char::MAX, false, direction)
            }
            Atom::CharClass(c) => {
                let (mut charset, invert) = c.compile(record);
                charset.sort();
                CharacterSetMatcherVec(record, charset, invert, direction)
            }
            Atom::Group { 
                parent_before,
                paren_within:_,
                group:_, 
                disjunction 
            } => {
                let m = disjunction.create_matcher(top, record, direction);

                let parenIndex = *parent_before;

                return Arc::new(move |state, cont| {
                    let Input = state.input;
                    let xe = state.end_index;

                    let d: Cont = Arc::new(move |y| {
                        let mut cap = y.captures.clone();
                        let ye = y.end_index;

                        let r = if direction.is_forward() {
                            assert!(xe <= ye);
                            xe..ye
                        } else {
                            assert!(ye <= xe);
                            ye..xe
                        };

                        cap[parenIndex as usize + 1] = Some(r);

                        let z = State {
                            input: Input,
                            start_index: y.start_index,
                            end_index: ye,
                            captures: cap,
                        };
                        return cont(z);
                    });

                    m(state, d)
                });
            }

            Atom::NonCaptGroup { paren_within:_, disjunction } => return disjunction.create_matcher(top, record, direction),

            Atom::Escape(a) => return a.create_matcher(top, record, direction),
        }
    }
}

impl AtomEscape {
    pub fn create_matcher(
        &self,
        top: &Pattern,
        record: RegExpRecord,
        direction: Direction,
    ) -> Matcher {
        match self {
            AtomEscape::Decimal(d) => {
                let n = d.CapturingGroupNumber() as usize;
                assert!(n <= record.capturing_groups_count);
                return BackreferenceMatcher(record, n, direction);
            }

            AtomEscape::Character(c) => {
                let cv = c.CharacterValue();
                let ch = char::from_u32(cv).unwrap();
                return CharacterSetMatcherVec(record, vec![ch], false, direction);
            }

            AtomEscape::CharacterClass(c) => {
                let mut a = c.compile_to_charset(record);
                a.sort();
                return CharacterSetMatcherVec(record, a, false, direction);
            }

            AtomEscape::GroupName(n) => {
                let mut ms = Vec::new();

                for i in &top.groups{
                    if let Some(gs) = i{
                         if gs.name.RegExpIdentifierCodePoints() == n.0.RegExpIdentifierCodePoints(){
                            ms.push(gs);
                        }
                    }
                    
                }
                assert!(ms.len() == 1);
                let paren_index = ms[0].paren_before;
                return BackreferenceMatcher(record, paren_index as usize, direction);
            }
        }
    }
}

impl CharacterClass {
    pub fn compile(&self, record: RegExpRecord) -> (Vec<char>, bool) {
        match self {
            Self::PosClass(c) => (c.compile_to_charset(record), false),
            Self::NegClass(c) => (c.compile_to_charset(record), true),
        }
    }
}

impl ClassRanges {
    pub fn compile_to_charset(&self, record: RegExpRecord) -> Vec<char> {
        match self {
            ClassRanges::Empty => Vec::new(),
            ClassRanges::NonemptyClassRangesNoDash(n) => n.compile_to_charset(record),
        }
    }
}

impl NonEmptyClassRanges {
    pub fn compile_to_charset(&self, record: RegExpRecord) -> Vec<char> {
        match self {
            NonEmptyClassRanges::ClassAtom(a) => a.compile_to_charset(record),
            NonEmptyClassRanges::ClassCharRange { start, end, ranges } => {
                let a = start.compile_to_charset(record);
                let b = end.compile_to_charset(record);
                let c = ranges.compile_to_charset(record);
                let d = CharacterRange(a, b);
                d.union(c)
            }
            NonEmptyClassRanges::ClassCont { atom, ranges } => {
                let a = atom.compile_to_charset(record);
                let b = ranges.compile_to_charset(record);
                a.union(b)
            }
        }
    }
}

impl NonEmptyClassRangesNoDash {
    pub fn compile_to_charset(&self, record: RegExpRecord) -> Vec<char> {
        match self {
            NonEmptyClassRangesNoDash::ClassAtom(a) => a.compile_to_charset(record),
            NonEmptyClassRangesNoDash::ClassCharRange { start, end, ranges } => {
                let a = start.compile_to_charset(record);
                let b = end.compile_to_charset(record);
                let c = ranges.compile_to_charset(record);
                let d = CharacterRange(a, b);
                d.union(c)
            }
            NonEmptyClassRangesNoDash::ClassCont { atom, ranges } => {
                let a = atom.compile_to_charset(record);
                let b = ranges.compile_to_charset(record);
                a.union(b)
            }
        }
    }
}

impl ClassAtom {
    pub fn compile_to_charset(&self, record: RegExpRecord) -> Vec<char> {
        match self {
            ClassAtom::Dash => vec!['\u{002D}'],
            ClassAtom::NoDash(n) => n.compile_to_charset(record),
        }
    }
}

impl ClassAtomNoDash {
    pub fn compile_to_charset(&self, record: RegExpRecord) -> Vec<char> {
        match self {
            ClassAtomNoDash::Char(c) => vec![*c],
            ClassAtomNoDash::ClassEscape(c) => c.compile_to_charset(record),
        }
    }
}

impl ClassEscape {
    pub fn compile_to_charset(&self, record: RegExpRecord) -> Vec<char> {
        match self {
            ClassEscape::b | ClassEscape::Dash => {
                vec![char::from_u32(self.CharacterValue()).unwrap()]
            }
            ClassEscape::CharacterEscape(c) => vec![char::from_u32(self.CharacterValue()).unwrap()],
            ClassEscape::CharacterClassEscape(c) => c.compile_to_charset(record),
            ClassEscape::OctalDigit(o) => unimplemented!(),
        }
    }
}

impl CharacterClassEscape {
    pub fn compile_to_charset(&self, record: RegExpRecord) -> Vec<char> {
        match self {
            CharacterClassEscape::d => ('0'..='9').collect(),
            CharacterClassEscape::D => ('\0'..=char::MAX)
                .filter(|x| !('0'..='9').contains(x))
                .collect(),
            CharacterClassEscape::s => rusty_js_unicode::WhiteSpace.to_vec(),
            CharacterClassEscape::S => ('\0'..=char::MAX)
                .filter(|x| !rusty_js_unicode::WhiteSpace.contains(x))
                .collect(),
            CharacterClassEscape::w => WordCharacters(record),
            CharacterClassEscape::W => {
                let w = WordCharacters(record);
                ('\0'..=char::MAX).filter(|x| !w.contains(x)).collect()
            }
            CharacterClassEscape::p(expr) => expr.compile_to_charset(record),
            CharacterClassEscape::P(expr) => {
                let s = expr.compile_to_charset(record);
                ('\0'..=char::MAX).filter(|x| !s.contains(x)).collect()
            }
        }
    }
}

impl UnicodePropertyValueExpression {
    pub fn compile_to_charset(&self, record: RegExpRecord) -> Vec<char> {
        match self {
            UnicodePropertyValueExpression::NameValue { name, value } => {
                let ps = &name.0;
                let name = rusty_js_unicode::MatchProperty(&ps).unwrap();
                todo!()
            }
            UnicodePropertyValueExpression::NameOrValue(v) => {
                todo!()
            }
        }
    }
}

pub fn RepeatMatcher(
    m: Matcher,
    min: f64,
    max: f64,
    greedy: bool,
    state: State,
    cont: Cont,
    paren_index: usize,
    paren_count: usize,
) -> MatchResult {
    if max == 0.0 {
        return cont(state);
    }

    let mm = m.clone();
    let c = cont.clone();
    let d: Cont = Arc::new(move |y| {
        if min == 0.0 && y.end_index == state.end_index {
            return Err(());
        }

        let min2 = if min == 0.0 { 0.0 } else { -1.0 };

        let max2 = if max == f64::INFINITY {
            f64::INFINITY
        } else {
            -1.0
        };

        RepeatMatcher(
            mm.clone(),
            min2,
            max2,
            greedy,
            y,
            c.clone(),
            paren_index,
            paren_count,
        )
    });

    let mut cap = state.captures.clone();

    for k in 0..cap.len() {
        if k < paren_index && k <= paren_index + paren_count {
            cap[k] = None;
        }
    }

    let xr = State {
        start_index: state.start_index,
        input: state.input,
        end_index: state.end_index,
        captures: cap,
    };

    if min != 0.0 {
        return m(xr, d);
    }

    let z = m(xr, d);
    if z.is_ok() {
        return z;
    }

    return cont(state);
}

pub fn IsWordChar(rer: RegExpRecord, input: &[u16], e: i64) -> bool {
    let len = input.len();

    if e == -1 || e == len as i64 {
        return false;
    }

    let c = input[e as usize];

    if ('a' as u16..='z' as u16).contains(&c) {
        return true;
    }

    if ('A' as u16..='Z' as u16).contains(&c) {
        return true;
    }

    if ('0' as u16..='9' as u16).contains(&c) {
        return true;
    }

    if c == '_' as u16 {
        return true;
    }

    if rer.unicode && rer.ignore_case {
        todo!("extra word set")
    }

    return false;
}

#[inline]
pub fn CharacterSetMatcherVec(
    record: RegExpRecord,
    charset: Vec<char>,
    invert: bool,
    direction: Direction,
) -> Matcher {
    return Arc::new(move |state, cont| {
        let e = state.end_index;

        let f = if direction.is_forward() {
            e as i64 + 1
        } else {
            e as i64 - 1
        };

        let input_length = state.input.len();
        if f < 0 || f > input_length as i64 {
            return Err(());
        }

        let index = e.min(f as usize);
        let ch = char::from_u32(state.input[index] as u32).unwrap();
        let cc = Canonicalize(record, ch);

        let re = charset.binary_search_by(|v|{
            Canonicalize(record, *v).cmp(&cc)
        });
        let found = re.is_ok();

        if invert == found {
            return Err(());
        }
        let y = State {
            input: state.input,
            start_index: state.start_index,
            end_index: f as usize,
            captures: state.captures,
        };
        cont(y)
    });
}

pub fn CharacterSetMatcherRange(
    record: RegExpRecord,
    charset: Range<char>,
    invert: bool,
    direction: Direction,
) -> Matcher {
    return Arc::new(move |state, cont| {
        let e = state.end_index;

        let f = if direction.is_forward() {
            e as i64 + 1
        } else {
            e as i64 - 1
        };

        let input_length = state.input.len();
        if f < 0 || f > input_length as i64 {
            return Err(());
        }

        let index = e.min(f as usize);
        let ch = char::from_u32(state.input[index] as u32).unwrap();
        let cc = Canonicalize(record, ch);

        let mut found = false;
        for i in charset.clone() {
            if Canonicalize(record, i) == cc {
                found = true;
                break;
            }
        }

        if invert == found {
            return Err(());
        }
        let y = State {
            input: state.input,
            start_index: state.start_index,
            end_index: f as usize,
            captures: state.captures,
        };
        cont(y)
    });
}

pub fn BackreferenceMatcher(record: RegExpRecord, n: usize, direction: Direction) -> Matcher {
    return Arc::new(move |state, cont| {
        let r = state.captures[n].clone();
        if r.is_none() {
            return cont(state);
        }

        let r = r.unwrap();
        let e = state.end_index;
        let rs = r.start;
        let re = r.end;
        let len = re - rs;
        let f = if direction.is_forward() {
            e as i64 + len as i64
        } else {
            e as i64 - len as i64
        };
        let input_length = state.input.len();
        if f < 0 || f > input_length as i64 {
            return Err(());
        }

        let f = f as usize;
        let g = e.min(f);

        let y = State {
            input: state.input,
            start_index: state.start_index,
            end_index: f,
            captures: state.captures,
        };
        cont(y)
    });
}

pub fn Canonicalize(record: RegExpRecord, ch: char) -> char {
    if record.unicode && record.ignore_case {
        let re = rusty_js_unicode::case_folding_data::CASE_FOLDING_TABLE
            .binary_search_by(|x| (x.0.cmp(&ch)));
        match re {
            Ok(i) => {
                let chars = rusty_js_unicode::case_folding_data::CASE_FOLDING_TABLE[i].1;
                if chars[1] == '\0' {
                    return chars[0];
                } else {
                    ch
                }
            }
            Err(_) => ch,
        }
    } else if record.ignore_case {
        return ch;
    } else {
        let v = char::from_u32(ch as u32).unwrap();
        let c = rusty_js_unicode::code_points_to_string(v.to_uppercase());
        if c.len() == 0 {
            return ch;
        } else {
            let cu = c[0];
            if ch as u32 >= 128 && (cu as u32) < 128 {
                return ch;
            }
            return cu;
        }
    }
}

pub fn CharacterRange(a: Vec<char>, b: Vec<char>) -> Vec<char> {
    assert!(a.len() == 1 && b.len() == 1);
    (a[0]..=b[0]).into_iter().collect()
}

pub fn WordCharacters(record: RegExpRecord) -> Vec<char> {
    let mut basic = ('a'..='z').collect::<Vec<_>>();
    basic.extend('A'..='Z');
    basic.extend('0'..='9');
    basic.push('_');

    if record.unicode && record.ignore_case {
        for c in '\0'..=char::MAX {
            if !basic.contains(&c) {
                if basic.contains(&Canonicalize(record, c)) {
                    basic.push(c)
                }
            }
        }
    }

    return basic;
}
