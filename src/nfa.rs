use crate::automaton::{Automata, Automaton, Buildable};
use crate::dfa::{ToDfa, DFA};
use crate::regex::{Regex, ToRegex};
use crate::utils::*;
use std::cmp::{Ordering, Ordering::*, PartialEq, PartialOrd};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::iter::{repeat, FromIterator};
use std::ops::{Add, Bound::*, Mul, Neg, Not, RangeBounds, Sub};
use std::str::FromStr;

/// https://en.wikipedia.org/wiki/Nondeterministic_finite_automaton
#[derive(Debug, Clone)]
pub struct NFA<V: Eq + Hash + Display + Copy + Clone + Debug> {
    pub(crate) alphabet: HashSet<V>,
    pub(crate) initials: HashSet<usize>,
    pub(crate) finals: HashSet<usize>,
    pub(crate) transitions: Vec<HashMap<V, Vec<usize>>>,
}

/// An interface for structs that can be converted into a NFA.
pub trait ToNfa<V: Eq + Hash + Display + Copy + Clone + Debug> {
    fn to_nfa(&self) -> NFA<V>;
}

/* IMPLEMENTATION OF NFA */

impl<V: Eq + Hash + Display + Copy + Clone + Debug> NFA<V> {
    /// Returns an NFA that accepts a word if and only if this word is accepted by both `self` and `other`.
    pub fn intersect(self, other: NFA<V>) -> NFA<V> {
        self.negate().unite(other.negate()).negate().to_nfa()
    }

    /// A contains B if and only if for each `word` w, if B `accepts` w then A `accepts` w.
    pub fn contains(&self, other: &NFA<V>) -> bool {
        self.clone().negate().intersect(other.clone()).is_empty()
    }

    fn small_to_dfa(&self) -> DFA<V> {
        let mut map = HashMap::new();
        let mut stack = VecDeque::new();

        let mut dfa = DFA {
            alphabet: self.alphabet.clone(),
            initial: 0,
            finals: HashSet::new(),
            transitions: vec![HashMap::new()],
        };

        let i: u128 = self.initials.iter().fold(0, |acc, x| acc | (1 << *x));
        if self.initials.iter().any(|x| self.finals.contains(x)) {
            dfa.finals.insert(0);
        }

        map.insert(i, 0);
        stack.push_back((i, HashSet::from_iter(self.initials.clone().into_iter())));

        while let Some((elem, iter)) = stack.pop_front() {
            let elem_num = *map.get(&elem).unwrap();
            for v in &self.alphabet {
                let mut it = HashSet::new();
                for state in &iter {
                    if let Some(transitions) = self.transitions[*state].get(&v) {
                        for t in transitions {
                            it.insert(*t);
                        }
                    }
                }
                if it.is_empty() {
                    continue;
                }

                let other = it.iter().fold(0, |acc, x| acc | 1 << *x);
                if !map.contains_key(&other) {
                    let l = dfa.transitions.len();
                    map.insert(other, l);
                    if it.iter().any(|x| self.finals.contains(x)) {
                        dfa.finals.insert(l);
                    }
                    stack.push_back((other, it));
                    dfa.transitions.push(HashMap::new());
                }
                dfa.transitions[elem_num].insert(*v, *map.get(&other).unwrap());
            }
        }

        dfa
    }

    fn big_to_dfa(&self) -> DFA<V> {
        unimplemented!()
    }

    /// Export to dotfile in dots/automaton/i.dot
    pub fn write_dot(&self, i: u8) -> Result<(), std::io::Error> {
        use std::fs::File;
        use std::io::Write;
        use std::path::Path;

        let mut name = "dots/automaton".to_string();
        name.push_str(&i.to_string());
        name.push_str(".dot");
        let name = Path::new(&name);

        let mut file = File::create(&name)?;
        file.write(b"digraph {\n")?;

        if !self.finals.is_empty() {
            file.write(b"    node [shape = doublecircle];")?;
            for e in &self.finals {
                write!(file, " S_{}", e)?;
            }
            file.write(b";\n")?;
        }

        if !self.initials.is_empty() {
            file.write(b"    node [shape = point];")?;
            for e in &self.initials {
                write!(file, " I_{}", e)?;
            }
            file.write(b";\n")?;
        }

        file.write(b"    node [shape = circle];\n")?;
        let mut tmp_map = HashMap::new();
        for (i, map) in self.transitions.iter().enumerate() {
            if map.is_empty() {
                write!(file, "    S_{};\n", i)?;
            }
            for (k, v) in map {
                for e in v {
                    tmp_map.entry(e).or_insert(Vec::new()).push(k);
                }
            }
            for (e, v) in tmp_map.drain() {
                let mut vs = v.into_iter().fold(String::new(), |mut acc, x| {
                    acc.push_str(&x.to_string());
                    acc.push_str(", ");
                    acc
                });
                vs.pop();
                vs.pop();
                write!(file, "    S_{} -> S_{} [label = \"{}\"];\n", i, e, vs)?;
            }
        }

        for e in &self.initials {
            write!(file, "    I_{} -> S_{};\n", e, e)?;
        }

        file.write(b"}")?;

        Ok(())
    }

    /// Returns an empty NFA.
    pub fn new_empty(alphabet: HashSet<V>) -> NFA<V> {
        NFA {
            alphabet,
            initials: HashSet::new(),
            finals: HashSet::new(),
            transitions: Vec::new(),
        }
    }

    /// Returns a full NFA.
    pub fn new_full(alphabet: HashSet<V>) -> NFA<V> {
        NFA {
            transitions: vec![alphabet.iter().map(|v| (*v, vec![0])).collect()],
            alphabet,
            initials: (0..=0).collect(),
            finals: (0..=0).collect(),
        }
    }

    /// Returns a NFA that accepts all words of the given length.
    pub fn new_length(alphabet: HashSet<V>, len: usize) -> NFA<V> {
        let mut transitions: Vec<_> = repeat(HashMap::new()).take(len).collect();
        for (i, map) in transitions.iter_mut().enumerate() {
            for v in &alphabet {
                map.insert(*v, vec![i + 1]);
            }
        }

        transitions.push(HashMap::new());

        NFA {
            alphabet,
            initials: (0..=0).collect(),
            finals: (len..=len).collect(),
            transitions,
        }
    }

    /// Returns a NFA that accepts only the given word.
    pub fn new_matching(alphabet: HashSet<V>, word: &Vec<V>) -> NFA<V> {
        let l = word.len();
        let mut nfa = NFA {
            alphabet,
            initials: (0..=0).collect(),
            finals: (l..=l).collect(),
            transitions: repeat(HashMap::new()).take(l + 1).collect(),
        };

        for (i, l) in word.into_iter().enumerate() {
            nfa.transitions[i].insert(*l, vec![i + 1]);
        }

        nfa
    }

    /// Returns a NFA that accepts only the empty word.
    pub fn new_empty_word(alphabet: HashSet<V>) -> NFA<V> {
        NFA {
            alphabet,
            initials: (0..=0).collect(),
            finals: (0..=0).collect(),
            transitions: vec![HashMap::new()],
        }
    }
}

impl<V: Eq + Hash + Display + Copy + Clone + Debug> ToDfa<V> for NFA<V> {
    fn to_dfa(&self) -> DFA<V> {
        if self.is_empty() {
            DFA {
                alphabet: self.alphabet.clone(),
                initial: 0,
                finals: HashSet::new(),
                transitions: vec![HashMap::new()],
            }
        } else if self.transitions.len() < 128 {
            self.small_to_dfa()
        } else {
            self.big_to_dfa()
        }
    }
}

impl<V: Eq + Hash + Display + Copy + Clone + Debug> ToNfa<V> for NFA<V> {
    fn to_nfa(&self) -> NFA<V> {
        self.clone()
    }
}

impl<V: Eq + Hash + Display + Copy + Clone + Debug> ToRegex<V> for NFA<V> {
    fn to_regex(&self) -> Regex<V> {
        unimplemented!()
    }
}

impl<V: Eq + Hash + Display + Copy + Clone + Debug> Automata<V> for NFA<V> {
    fn run(&self, v: &Vec<V>) -> bool {
        if self.initials.is_empty() {
            return false;
        }

        let mut actuals = self.initials.clone();
        let mut next = HashSet::new();

        for l in v {
            for st in &actuals {
                if let Some(tr) = self.transitions[*st].get(l) {
                    for t in tr {
                        next.insert(*t);
                    }
                }
            }

            std::mem::swap(&mut actuals, &mut next);
            if actuals.is_empty() {
                return false;
            }
            next.clear();
        }

        return actuals.iter().any(|x| self.finals.contains(x));
    }

    fn is_complete(&self) -> bool {
        if self.initials.is_empty() {
            return false;
        }

        for m in &self.transitions {
            for v in &self.alphabet {
                if match m.get(v) {
                    None => true,
                    Some(v) => v.is_empty(),
                } {
                    return false;
                }
            }
        }
        return true;
    }

    fn is_reachable(&self) -> bool {
        let mut acc: HashSet<usize> = self.initials.clone().into_iter().collect();
        let mut stack: Vec<usize> = self.initials.iter().cloned().collect();
        while let Some(e) = stack.pop() {
            for (_, v) in &self.transitions[e] {
                for t in v {
                    if !acc.contains(t) {
                        acc.insert(*t);
                        stack.push(*t);
                    }
                }
            }
        }
        acc.len() == self.transitions.len()
    }

    fn is_coreachable(&self) -> bool {
        self.clone().reverse().is_reachable()
    }

    fn is_trimmed(&self) -> bool {
        self.is_reachable() && self.is_coreachable()
    }

    fn is_empty(&self) -> bool {
        if !self.initials.is_disjoint(&self.finals) {
            return false;
        }

        let mut acc: HashSet<usize> = self.initials.clone().into_iter().collect();
        let mut stack: Vec<usize> = self.initials.clone().into_iter().collect();

        while let Some(e) = stack.pop() {
            for (_, v) in &self.transitions[e] {
                for t in v {
                    if self.finals.contains(t) {
                        return false;
                    }
                    if !acc.contains(t) {
                        acc.insert(*t);
                        stack.push(*t);
                    }
                }
            }
        }
        return true;
    }

    fn is_full(&self) -> bool {
        if self.initials.is_disjoint(&self.finals) {
            return false;
        }

        let mut acc: HashSet<usize> = self.initials.clone().into_iter().collect();
        let mut stack: Vec<usize> = self.initials.clone().into_iter().collect();

        while let Some(e) = stack.pop() {
            for (_, v) in &self.transitions[e] {
                for t in v {
                    if !self.finals.contains(t) {
                        return false;
                    }
                    if !acc.contains(t) {
                        acc.insert(*t);
                        stack.push(*t);
                    }
                }
            }
        }
        return true;
    }

    fn negate(self) -> NFA<V> {
        self.to_dfa().negate().to_nfa()
    }

    fn complete(mut self) -> NFA<V> {
        if self.is_complete() {
            return self;
        }

        let l = self.transitions.len();
        self.transitions.push(HashMap::new());
        for m in &mut self.transitions {
            for v in &self.alphabet {
                let t = m.entry(*v).or_insert(Vec::new());
                if t.is_empty() {
                    t.push(l);
                }
            }
        }

        if self.initials.is_empty() {
            self.initials.insert(l);
        }

        self
    }

    fn make_reachable(mut self) -> NFA<V> {
        let mut acc: HashSet<usize> = self.initials.clone().into_iter().collect();
        let mut stack: Vec<usize> = self.initials.iter().cloned().collect();
        while let Some(e) = stack.pop() {
            for (_, v) in &self.transitions[e] {
                for t in v {
                    if !acc.contains(t) {
                        acc.insert(*t);
                        stack.push(*t);
                    }
                }
            }
        }

        let mut map = HashMap::new();
        let mut ind = 0;
        let l = self.transitions.len();
        for i in 0..l {
            if acc.contains(&i) {
                map.insert(i, ind);
                self.transitions.swap(i, ind);
                ind += 1;
            }
        }
        self.transitions.truncate(ind);

        self.finals = self
            .finals
            .iter()
            .filter(|x| acc.contains(&x))
            .map(|x| *map.get(x).unwrap())
            .collect();
        // no need to filter the initials since they are reachable
        self.initials = self.initials.iter().map(|x| *map.get(x).unwrap()).collect();
        for m in &mut self.transitions {
            for v in m.values_mut() {
                for t in v {
                    *t = *map.get(t).unwrap();
                }
            }
        }

        self
    }

    fn make_coreachable(self) -> NFA<V> {
        self.reverse().make_reachable().reverse()
    }

    fn trim(self) -> NFA<V> {
        self.make_reachable().make_coreachable()
    }

    fn reverse(mut self) -> NFA<V> {
        let mut transitions: Vec<_> = repeat(HashMap::new())
            .take(self.transitions.len())
            .collect();

        for i in 0..self.transitions.len() {
            for (k, v) in &self.transitions[i] {
                for e in v {
                    transitions[*e].entry(*k).or_insert(Vec::new()).push(i);
                }
            }
        }

        self.transitions = transitions;
        std::mem::swap(&mut self.initials, &mut self.finals);
        return self;
    }
}

impl<V: Eq + Hash + Display + Copy + Clone + Debug> Buildable<V> for NFA<V> {
    fn unite(mut self, other: NFA<V>) -> NFA<V> {
        let NFA {
            alphabet,
            initials,
            finals,
            transitions,
        } = other;

        let l = self.transitions.len();

        append_hashset(&mut self.alphabet, alphabet);
        append_shift_hashset(&mut self.initials, initials, l);
        append_shift_hashset(&mut self.finals, finals, l);
        append_shift_transitions(&mut self.transitions, transitions);

        self
    }

    fn concatenate(mut self, mut other: NFA<V>) -> NFA<V> {
        let l = self.transitions.len();
        shift_fnda(&mut other, l);
        let NFA {
            alphabet,
            initials,
            finals,
            mut transitions,
        } = other;

        append_hashset(&mut self.alphabet, alphabet);

        for e in &initials {
            for (v, t) in &mut transitions[e - l] {
                // e - l because of the shift above
                for f in &self.finals {
                    self.transitions[*f]
                        .entry(*v)
                        .or_insert(Vec::new())
                        .append(&mut t.clone());
                }
            }
        }

        if finals.is_disjoint(&initials) {
            self.finals = finals;
        } else {
            append_hashset(&mut self.finals, finals);
        }
        self.transitions.append(&mut transitions);

        self
    }

    fn kleene(mut self) -> NFA<V> {
        let l = self.transitions.len();
        let mut map = HashMap::new();

        for i in &self.initials {
            for (k, v) in &self.transitions[*i] {
                let set = &mut map.entry(*k).or_insert(HashSet::new());
                for x in v {
                    set.insert(*x);
                }
            }
        }

        for i in &self.finals {
            for (k, v) in &map {
                let mut set: HashSet<usize> = self.transitions[*i]
                    .entry(*k)
                    .or_insert(Vec::new())
                    .drain(..)
                    .collect();
                for x in v {
                    set.insert(*x);
                }
                self.transitions[*i].insert(*k, set.into_iter().collect());
            }
        }

        self.transitions.push(
            map.into_iter()
                .map(|(k, v)| (k, v.into_iter().collect()))
                .collect(),
        );
        self.initials.clear();
        self.initials.insert(l);
        self.finals.insert(l);

        self
    }

    fn at_most(mut self, u: usize) -> NFA<V> {
        if !self.initials.iter().any(|x| self.finals.contains(x)) {
            let l = self.transitions.len();
            self.initials.insert(l);
            self.finals.insert(l);
            self.transitions.push(HashMap::new());
        }

        (0..u).fold(NFA::new_empty_word(self.alphabet.clone()), |acc, _| {
            acc.concatenate(self.clone())
        })
    }

    fn at_least(self, u: usize) -> NFA<V> {
        (0..u)
            .fold(NFA::new_empty_word(self.alphabet.clone()), |acc, _| {
                acc.concatenate(self.clone())
            })
            .concatenate(self.kleene())
    }

    fn repeat<R: RangeBounds<usize>>(self, r: R) -> NFA<V> {
        let start = match r.start_bound() {
            Included(&a) => a,
            Excluded(&a) => a + 1,
            Unbounded => 0,
        };

        let end = match r.end_bound() {
            Included(&a) => Some(a),
            Excluded(&a) => Some(a - 1),
            Unbounded => None,
        };

        if let Some(end) = end {
            if end < start {
                return NFA::new_empty(self.alphabet);
            }
        }

        if let Some(end) = end {
            (0..start)
                .fold(NFA::new_empty_word(self.alphabet.clone()), |acc, _| {
                    acc.concatenate(self.clone())
                })
                .concatenate(self.at_most(end - start))
        } else {
            self.at_least(start)
        }
    }
}

impl<V: Eq + Hash + Display + Copy + Clone + Debug> PartialEq<NFA<V>> for NFA<V> {
    fn eq(&self, other: &NFA<V>) -> bool {
        self.le(other) && self.ge(other)
    }
}

impl<V: Eq + Hash + Display + Copy + Clone + Debug> PartialEq<DFA<V>> for NFA<V> {
    fn eq(&self, other: &DFA<V>) -> bool {
        self.eq(&other.to_nfa())
    }
}

impl<V: Eq + Hash + Display + Copy + Clone + Debug> PartialEq<Regex<V>> for NFA<V> {
    fn eq(&self, other: &Regex<V>) -> bool {
        self.eq(&other.to_nfa())
    }
}

impl<V: Eq + Hash + Display + Copy + Clone + Debug> PartialEq<Automaton<V>> for NFA<V> {
    fn eq(&self, other: &Automaton<V>) -> bool {
        match other {
            Automaton::DFA(v) => self.eq(&*v),
            Automaton::NFA(v) => self.eq(&*v),
            Automaton::REG(v) => self.eq(&*v),
        }
    }
}

impl<V: Eq + Hash + Display + Copy + Clone + Debug> PartialOrd for NFA<V> {
    fn partial_cmp(&self, other: &NFA<V>) -> Option<Ordering> {
        match (self.ge(&other), self.le(&other)) {
            (true, true) => Some(Equal),
            (true, false) => Some(Greater),
            (false, true) => Some(Less),
            (false, false) => None,
        }
    }

    fn lt(&self, other: &NFA<V>) -> bool {
        other.contains(&self) && !self.contains(&other)
    }

    fn le(&self, other: &NFA<V>) -> bool {
        other.contains(&self)
    }

    fn gt(&self, other: &NFA<V>) -> bool {
        self.contains(&other) && !other.contains(&self)
    }

    fn ge(&self, other: &NFA<V>) -> bool {
        self.contains(&other)
    }
}

impl FromStr for NFA<char> {
    type Err = String;

    fn from_str(s: &str) -> Result<NFA<char>, Self::Err> {
        s.parse::<Regex<char>>().map(|x| x.to_nfa())
    }
}

/// The multiplication of A and B is A.concatenate(B)
impl<V: Eq + Hash + Display + Copy + Clone + Debug> Mul for NFA<V> {
    type Output = Self;

    fn mul(self, other: NFA<V>) -> NFA<V> {
        self.concatenate(other)
    }
}

/// The negation of A is A.negate().
impl<V: Eq + Hash + Display + Copy + Clone + Debug> Neg for NFA<V> {
    type Output = Self;

    fn neg(self) -> NFA<V> {
        self.negate()
    }
}

/// The opposite of A is A.reverse().
impl<V: Eq + Hash + Display + Copy + Clone + Debug> Not for NFA<V> {
    type Output = Self;

    fn not(self) -> NFA<V> {
        self.reverse()
    }
}

/// The substraction of A and B is an automaton that accepts a word if and only if A accepts it and B doesn't.
impl<V: Eq + Hash + Display + Copy + Clone + Debug> Sub for NFA<V> {
    type Output = Self;

    fn sub(self, other: NFA<V>) -> NFA<V> {
        self.intersect(other.negate())
    }
}

/// The addition fo A and B is an automaton that accepts a word if and only if A or B accept it.
impl<V: Eq + Hash + Display + Copy + Clone + Debug> Add for NFA<V> {
    type Output = Self;

    fn add(self, other: NFA<V>) -> NFA<V> {
        self.unite(other)
    }
}
