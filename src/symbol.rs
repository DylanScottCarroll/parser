use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolType {
    Nonterminal,
    Terminal,
    Epsilon,
    Eof,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Symbol {
    pub symbol_type: SymbolType,
    pub id: u32,
}

impl Symbol {
    pub const EPSILON: Symbol = Symbol {
        symbol_type: SymbolType::Epsilon,
        id: 0,
    };
    pub const EOF: Symbol = Symbol {
        symbol_type: SymbolType::Epsilon,
        id: 1,
    };

    pub fn is_terrminal(&self) -> bool {
        self.symbol_type == SymbolType::Terminal
    }

    pub fn is_nonterrminal(&self) -> bool {
        self.symbol_type == SymbolType::Nonterminal
    }
}

pub struct SymbolDomain {
    len: usize,
    symbols: Vec<Symbol>,
    names: Vec<String>,
    by_name: HashMap<String, u32>,

    terminals: Vec<u32>,
    nonterminals: Vec<u32>,
}

impl SymbolDomain {
    const START_ID: u32 = 2;

    pub fn new(symbols: Vec<(String, SymbolType)>) -> SymbolDomain {
        let len: usize = symbols.len() + Self::START_ID as usize;

        let capacity = len as usize;
        let mut domain = SymbolDomain {
            len,
            symbols: Vec::with_capacity(capacity),
            names: Vec::with_capacity(capacity),
            by_name: HashMap::with_capacity(capacity),

            terminals: Vec::new(),
            nonterminals: Vec::new(),
        };

        domain.push_reserved_tokens();

        let mut id = Self::START_ID;
        for (name, symbol_type) in symbols {
            domain.push_new_symbol(id, name, symbol_type);
            id += 1;
        }

        domain
    }

    fn push_reserved_tokens(&mut self) {
        self.symbols.push(Symbol::EPSILON);
        self.symbols.push(Symbol::EOF);

        self.by_name.insert(String::from("epsilon"), 0);
        self.by_name.insert(String::from("eof"), 1);

        self.names.push(String::from("epsilon"));
        self.names.push(String::from("eof"));
    }

    fn push_new_symbol(&mut self, id: u32, name: String, symbol_type: SymbolType) {
        match symbol_type {
            SymbolType::Epsilon | SymbolType::Eof => {
                panic!("Cannot create new symbols with type Epsillon or Eof")
            }
            SymbolType::Terminal => self.terminals.push(id),
            SymbolType::Nonterminal => self.nonterminals.push(id),
        }

        self.symbols.push(Symbol { symbol_type, id });
        self.names.push(name.clone());
        self.by_name.insert(name, id);
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn symbol_name(&self, symbol: Symbol) -> String {
        self.names[symbol.id as usize].clone()
    }

    pub fn by_name(&self, name: &str) -> Option<Symbol> {
        self.by_name
            .get(name)
            .and_then(|&i| self.symbols.get(i as usize))
            .map(|s| *s)
    }

    pub fn terminals(&self) -> Vec<Symbol> {
        self.iter().filter(|s| s.is_terrminal()).collect()
    }

    pub fn nonterminals(&self) -> Vec<Symbol> {
        self.iter().filter(|s| s.is_nonterrminal()).collect()
    }

    pub fn iter(&self) -> impl Iterator<Item = Symbol> {
        (0..self.len)
            .map(|i| self.symbols.get(i))
            .flatten()
            .cloned()
    }
}

#[derive(Clone)]
enum SymbolSetStore {
    Inline([u64; 4]),
    Heap(Vec<u64>),
}

use SymbolSetStore::{Heap, Inline};
impl SymbolSetStore {
    const WORD_SIZE: usize = 64;
    const INLINE_WORDS: usize = 4;
    const INLINE_BITS: usize = SymbolSetStore::WORD_SIZE * SymbolSetStore::INLINE_WORDS;

    fn inline() -> SymbolSetStore {
        Inline([0; SymbolSetStore::INLINE_WORDS])
    }

    fn heap(words: usize) -> SymbolSetStore {
        Heap(vec![0; words])
    }

    fn word(&self, word: usize) -> u64 {
        if word >= self.words() {
            0
        } else {
            match self {
                Inline(arr) => arr[word],
                Heap(vec) => vec[word],
            }
        }
    }

    fn word_mut(&mut self, word: usize) -> &mut u64 {
        match self {
            Inline(arr) => &mut arr[word],
            Heap(vec) => &mut vec[word],
        }
    }

    fn words(&self) -> usize {
        match self {
            Inline(_) => SymbolSetStore::INLINE_WORDS,
            Heap(vec) => vec.len(),
        }
    }
}

#[derive(Clone)]
pub struct SymbolSet(SymbolSetStore);

impl SymbolSet {
    pub fn new() -> SymbolSet {
        SymbolSet(SymbolSetStore::inline())
    }

    pub fn with_capacity(capacity: usize) -> SymbolSet {
        if capacity <= SymbolSetStore::INLINE_BITS {
            Self::new()
        } else {
            SymbolSet(SymbolSetStore::heap(capacity.div_ceil(64)))
        }
    }

    fn matching_capacity(&self) -> SymbolSet {
        match &self.0 {
            Inline(_) => Self::new(),
            Heap(vec) => SymbolSet(SymbolSetStore::heap(vec.len())),
        }
    }

    fn bit_index(id: u32) -> (usize, usize) {
        let i = id as usize;
        let word = i / SymbolSetStore::WORD_SIZE;
        let offset = i - (word * SymbolSetStore::WORD_SIZE);

        (word, offset)
    }

    fn grow(&mut self, words: usize) {
        if words > self.0.words() {
            match &mut self.0 {
                Inline(arr) => {
                    let mut vec = Vec::from(arr.as_slice());
                    let extra = words - vec.len();
                    vec.extend(std::iter::repeat(0).take(extra));
                    self.0 = SymbolSetStore::Heap(vec);
                }
                Heap(vec) => {
                    let extra = words - vec.len();
                    vec.extend(std::iter::repeat(0).take(extra));
                }
            }
        }
    }

    pub fn clear(&mut self) {
        match &mut self.0 {
            Inline(arr) => arr.fill(0),
            Heap(vec) => vec.fill(0),
        }
    }

    pub fn insert(&mut self, symbol: Symbol) {
        let (word, offset) = SymbolSet::bit_index(symbol.id);
        *self.0.word_mut(word) |= 1 << offset;
    }

    pub fn remove(&mut self, symbol: Symbol) {
        let (word, offset) = SymbolSet::bit_index(symbol.id);
        *self.0.word_mut(word) &= !(1 << offset);
    }

    pub fn extend(&mut self, other: &SymbolSet) {
        self.grow(other.0.words());
        let words = usize::min(self.0.words(), other.0.words());
        for i in 0..words {
            *self.0.word_mut(i) |= other.0.word(i);
        }
    }

    pub fn contains(&self, symbol: Symbol) -> bool {
        self.contains_id(symbol.id)
    }

    fn contains_id(&self, id: u32) -> bool {
        let (bucket, offset) = SymbolSet::bit_index(id);
        self.0.word(bucket) & (1 << offset) != 0
    }

    pub fn is_empty(&self) -> bool {
        for i in 0..self.0.words() {
            if self.0.word(i) != 0 {
                return false;
            }
        }

        true
    }

    pub fn difference(&self, other: &SymbolSet) -> SymbolSet {
        let mut new_set = self.matching_capacity();
        new_set.grow(other.0.words());
        for i in 0..new_set.0.words() {
            *new_set.0.word_mut(i) = self.0.word(i) & !other.0.word(i);
        }

        new_set
    }

    /// Create a new set without the given symbol
    pub fn minus(&self, symbol: Symbol) -> SymbolSet {
        let mut new_set = self.clone();
        new_set.remove(symbol);

        new_set
    }

    pub fn intersection(&self, other: &SymbolSet) -> SymbolSet {
        let mut new_set = self.matching_capacity();
        for i in 0..new_set.0.words() {
            *new_set.0.word_mut(i) = self.0.word(i) & other.0.word(i);
        }

        new_set
    }

    pub fn union(&self, other: &SymbolSet) -> SymbolSet {
        let mut new_set = self.matching_capacity();
        new_set.grow(other.0.words());
        for i in 0..new_set.0.words() {
            *new_set.0.word_mut(i) = self.0.word(i) | other.0.word(i);
        }

        new_set
    }

    /// Create a new set with the given symbol added
    pub fn plus(&self, symbol: Symbol) -> SymbolSet {
        let mut new_set = self.clone();
        new_set.insert(symbol);

        new_set
    }

    /// non-strict subset
    /// no bits are set in self that aren't set in other
    pub fn is_subset(&self, other: &SymbolSet) -> bool {
        for i in 0..self.0.words() {
            if self.0.word(i) & !other.0.word(i) != 0 {
                return false;
            }
        }

        true
    }

    /// non-strict superset
    /// no bits are set in other that aren't set in self
    pub fn is_superset(&self, other: &SymbolSet) -> bool {
        other.is_subset(self)
    }

    pub fn iter(&self, domain: &SymbolDomain) -> impl Iterator<Item = Symbol> {
        (0..domain.len)
            .filter(move |&id| self.contains_id(id as u32))
            .map(move |id| domain.symbols[id])
    }
}
