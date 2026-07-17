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
}

pub struct SymbolDomain {
    count: usize,
    symbols: Vec<Symbol>,
    names: Vec<String>,
    by_name: HashMap<String, u32>,

    terminals: Vec<u32>,
    nonterminals: Vec<u32>,
}

impl SymbolDomain {
    const START_ID: u32 = 2;

    pub fn new(symbols: Vec<(String, SymbolType)>) -> SymbolDomain {
        let count: usize = symbols.len() + Self::START_ID as usize;

        let capacity = count as usize;
        let mut domain = SymbolDomain {
            count,
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

    pub fn count(&self) -> usize {
        self.count
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
}

impl IntoIterator for SymbolDomain {
    type Item = Symbol;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.symbols.into_iter()
    }
}

#[derive(Clone)]
pub struct SymbolSet {
    bits: Vec<u64>,
    len: usize,
    domain_size: usize,
}

impl SymbolSet {
    pub fn new(domain_size: usize) -> SymbolSet {
        let num_buckers: usize = domain_size.div_ceil(64);
        SymbolSet {
            bits: Vec::with_capacity(num_buckers),
            len: 0,
            domain_size,
        }
    }

    fn bit_index(id: u32) -> (usize, usize) {
        let bucket = (id / 64) as usize;
        let offset = id as usize - (bucket * 64);

        (bucket, offset)
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn clear(&mut self) {
        for i in 0..self.bits.len() {
            self.bits[i] = 0;
        }
    }

    pub fn insert(&mut self, symbol: Symbol) {
        self.insert_id(symbol.id);
    }

    pub fn insert_id(&mut self, id: u32) {
        let (bucket, offset) = SymbolSet::bit_index(id);
        self.bits[bucket] |= 1 << offset;
    }

    pub fn remove(&mut self, symbol: Symbol) {
        self.remove_id(symbol.id)
    }

    fn remove_id(&mut self, id: u32) {
        let (bucket, offset) = SymbolSet::bit_index(id);
        self.bits[bucket] &= !(1 << offset);
    }

    fn contains_id(&self, id: u32) -> bool {
        let (bucket, offset) = SymbolSet::bit_index(id);
        self.bits[bucket] & (1 << offset) == 1
    }

    pub fn contains(&self, symbol: Symbol) -> bool {
        self.contains_id(symbol.id)
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn difference(&self, other: &SymbolSet) -> SymbolSet {
        // assert_eq!(self.domain, other.domain);

        let mut new_set = SymbolSet::new(self.domain_size);
        for i in 0..new_set.bits.len() {
            new_set.bits[i] = self.bits[i] & !other.bits[i];
        }

        new_set
    }

    pub fn intersection(&self, other: &SymbolSet) -> SymbolSet {
        let mut new_set = SymbolSet::new(self.domain_size);
        for i in 0..new_set.bits.len() {
            new_set.bits[i] = self.bits[i] & other.bits[i];
        }

        new_set
    }

    pub fn union(&self, other: &SymbolSet) -> SymbolSet {
        let mut new_set = SymbolSet::new(self.domain_size);
        for i in 0..new_set.bits.len() {
            new_set.bits[i] = self.bits[i] | other.bits[i];
        }

        new_set
    }

    pub fn is_subset(&self, other: &SymbolSet) -> bool {
        for i in 0..self.bits.len() {
            if self.bits[i] & !other.bits[i] > 0 {
                return false;
            }
        }
        return true;
    }
    pub fn is_superset(&self, other: &SymbolSet) -> bool {
        for i in 0..self.bits.len() {
            if other.bits[i] & !self.bits[i] > 0 {
                return false;
            }
        }
        return true;
    }

    pub fn iter(&self, domain: &SymbolDomain) -> impl Iterator<Item = Symbol> {
        let mut id = 0;

        std::iter::from_fn(move || {
            // Skip ids that are not in the set
            while id < self.len && !self.contains_id(id as u32) {
                id += 1;
            }

            let symbol = (id < self.len).then(|| domain.symbols[id]);
            id += 1;
            symbol
        })
    }
}

impl<'a> std::ops::Sub for &SymbolSet {
    type Output = SymbolSet;

    fn sub(self, rhs: Self) -> Self::Output {
        self.difference(&rhs)
    }
}

impl<'a> std::ops::BitOr for &SymbolSet {
    type Output = SymbolSet;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.union(rhs)
    }
}

impl<'a> std::ops::BitAnd for &SymbolSet {
    type Output = SymbolSet;

    fn bitand(self, rhs: Self) -> Self::Output {
        self.intersection(rhs)
    }
}
