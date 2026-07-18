use std::{collections::HashMap, rc::Rc};

use crate::symbol::{Symbol, SymbolDomain, SymbolSet, SymbolType};

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct GrammarRule {
    head: Symbol,
    body: Rc<[Symbol]>,
}

#[derive(Clone, PartialEq)]
pub enum ActionExpression {
    String(String),
    Number(f64),
    Reference {
        field: String,
        index: usize,
    },
    Node {
        head: Box<ActionExpression>,
        body: Box<[(String, ActionExpression)]>,
    },
    List(Box<[ActionExpression]>),
    // In the future, action routines could be made much more script-like by
    // adding nested expressions and conditionals to this list
}

#[derive(Clone, PartialEq)]
pub struct ActionRoutine {
    dest_id: String,
    action_kind: ActionExpression,
}

pub struct Grammar {
    start_symbol: Symbol,
    rules: Vec<GrammarRule>,
    action_routines: Vec<ActionRoutine>,
    rules_to_routines_map: HashMap<GrammarRule, Vec<usize>>,

    rules_by_head: HashMap<Symbol, Vec<usize>>,
    rules_by_body: HashMap<Symbol, Vec<usize>>,

    symbol_domain_size: usize,
    first_sets: HashMap<Symbol, SymbolSet>,
    follow_sets: HashMap<Symbol, SymbolSet>,
}

impl Grammar {
    pub fn new(
        start_symbol: Symbol,
        grammar_rules: Vec<(GrammarRule, Vec<ActionRoutine>)>,
        symbol_domain: &SymbolDomain,
    ) -> Grammar {
        let capacity = grammar_rules.len();
        let mut grammar = Grammar {
            start_symbol,
            rules: Vec::with_capacity(capacity),
            action_routines: Vec::new(),
            rules_to_routines_map: HashMap::new(),
            rules_by_head: HashMap::new(),
            rules_by_body: HashMap::new(),
            symbol_domain_size: symbol_domain.len(),
            first_sets: HashMap::new(),
            follow_sets: HashMap::new(),
        };

        for (rule, routines) in grammar_rules {
            grammar.rules.push(rule.clone());
            let rule_id = grammar.rules.len();

            grammar.update_rule_by_symbol_maps(rule.clone(), rule_id);

            grammar.add_routine(rule, routines);
        }

        grammar.populate_first_and_follow_sets(symbol_domain);

        // TODO
        // - Check reachability of all symbols
        // - Check that there are no symbols in rules not in the symboldomain?
        // - Change way start symbol is specified
        // - Add a special START nonterminal the user can't reference

        grammar
    }

    fn add_routine(&mut self, rule: GrammarRule, routines: Vec<ActionRoutine>) {
        let routine_id_vec = self
            .rules_to_routines_map
            .entry(rule)
            .or_insert(Vec::with_capacity(routines.len()));

        for routine in routines {
            self.action_routines.push(routine);
            let routine_id = self.action_routines.len();

            routine_id_vec.push(routine_id);
        }
    }

    fn update_rule_by_symbol_maps(&mut self, rule: GrammarRule, rule_id: usize) {
        self.rules_by_head
            .entry(rule.head)
            .or_default()
            .push(rule_id);

        for &body_symbol in rule.body.iter() {
            self.rules_by_body
                .entry(body_symbol)
                .or_default()
                .push(rule_id);
        }
    }

    fn populate_first_and_follow_sets(&mut self, symbol_domain: &SymbolDomain) {
        self.populate_first_sets(symbol_domain);
        self.populate_follow_sets(symbol_domain);
    }

    fn populate_first_sets(&mut self, symbol_domain: &SymbolDomain) {
        for symbol in symbol_domain.iter() {
            self.first_sets.insert(
                symbol,
                match symbol.symbol_type {
                    SymbolType::Nonterminal => SymbolSet::new(),
                    _ => SymbolSet::new().plus(symbol),
                },
            );
        }

        loop {
            let mut updated = SymbolSet::new();
            let mut visited = SymbolSet::new();

            self.traverse_first_sets(self.start_symbol, &mut visited, &mut updated);

            if updated.is_empty() {
                break;
            }
        }
    }

    /// Walks the production tree, updating each symbol with the first sets of all
    /// of that symbol's production rules. Appliede iteratively, this will converge
    /// to a correct solution once updated.is_empty() after returning.
    /// Will only populate symbols reachable from the provided start symbol.
    /// Assumes that the first sets of terminals are pre-populated
    fn traverse_first_sets(
        &mut self,
        symbol: Symbol,
        visited: &mut SymbolSet,
        updated: &mut SymbolSet,
    ) -> SymbolSet {
        if symbol.symbol_type != SymbolType::Nonterminal || visited.contains(symbol) {
            return self.first_set(symbol);
        }
        visited.insert(symbol);

        let mut updated_first_set = self.first_set(symbol);
        for rule in self.rule_by_head(symbol) {
            let mut body_is_nulllable = true;

            for body_symbol in rule.body.iter().cloned() {
                let body_symbol_set = self.traverse_first_sets(body_symbol, visited, updated);

                updated_first_set.extend(&body_symbol_set.minus(Symbol::EPSILON));

                if !body_symbol_set.contains(Symbol::EPSILON) {
                    body_is_nulllable = false;
                    break;
                }
            }

            if body_is_nulllable {
                updated_first_set.insert(Symbol::EPSILON);
            }
        }

        if !updated_first_set.is_subset(&self.first_set(symbol)) {
            updated.insert(symbol);
            self.first_sets.insert(symbol, updated_first_set.clone());
        }

        updated_first_set
    }

    fn populate_follow_sets(&mut self, symbol_domain: &SymbolDomain) {
        for symbol in symbol_domain.iter() {
            self.follow_sets.insert(
                symbol,
                if symbol == self.start_symbol {
                    SymbolSet::new().plus(Symbol::EOF)
                } else {
                    SymbolSet::new()
                },
            );
        }

        loop {
            let mut updated = SymbolSet::new();
            let mut visited = SymbolSet::new();

            self.traverse_follow_sets(self.start_symbol, &mut visited, &mut updated);

            if updated.is_empty() {
                break;
            }
        }
    }

    /// Walk the production tree, applying `apply_rule_to_follow_sets` to each rule
    /// encountered. Starting at the start symbol will evaluate every rule where the
    /// head is reachable by the grammar
    fn traverse_follow_sets(
        &mut self,
        symbol: Symbol,
        visited: &mut SymbolSet,
        updated: &mut SymbolSet,
    ) {
        if symbol.symbol_type != SymbolType::Nonterminal || visited.contains(symbol) {
            return;
        }
        visited.insert(symbol);

        for rule in self.rule_by_head(symbol) {
            self.apply_rule_to_follow_sets(&rule, updated);

            for body_symbol in rule.body.iter().cloned() {
                self.traverse_follow_sets(body_symbol, visited, updated);
            }
        }
    }

    /// Updates the follow set for every symbol in the rule body, relying on the follow
    /// set of the head. Marks in updaed each time the set for a symbol is updated.
    /// If the follow sets aren't completely populated, iterative applications will
    /// converge on a solution, indicated when updated.is_empty().
    fn apply_rule_to_follow_sets(&mut self, rule: &GrammarRule, updated: &mut SymbolSet) {
        let mut suffix_follow_set = self.follow_set(rule.head);

        for body_symbol in rule.body.iter().rev().cloned() {
            let symbol_follow_set = self.follow_sets.get_mut(&body_symbol).unwrap();
            if !suffix_follow_set.is_subset(symbol_follow_set) {
                symbol_follow_set.extend(&suffix_follow_set);
                updated.insert(body_symbol);
            }

            let symbol_first_set = self.first_set(body_symbol);
            if symbol_first_set.contains(Symbol::EPSILON) {
                suffix_follow_set.extend(&symbol_first_set.minus(Symbol::EPSILON));
            } else {
                suffix_follow_set = symbol_first_set;
            }
        }
    }
}

impl Grammar {
    pub fn get_rule(&self, index: usize) -> Option<GrammarRule> {
        self.rules.get(index).cloned()
    }

    pub fn get_routines(&self, grammar_rule: GrammarRule) -> Vec<ActionRoutine> {
        match self.rules_to_routines_map.get(&grammar_rule) {
            Some(indices) => indices
                .iter()
                .map(|&i| self.action_routines.get(i).cloned())
                .flatten()
                .collect(),
            None => Vec::new(),
        }
    }

    pub fn rule_by_head(&self, head: Symbol) -> Vec<GrammarRule> {
        match self.rules_by_head.get(&head) {
            Some(indices) => indices
                .iter()
                .map(|&i| self.rules.get(i).cloned())
                .flatten()
                .collect(),
            None => Vec::new(),
        }
    }

    pub fn rule_by_body(&self, head: Symbol) -> Vec<GrammarRule> {
        match self.rules_by_body.get(&head) {
            Some(indices) => indices
                .iter()
                .map(|&i| self.rules.get(i).cloned())
                .flatten()
                .collect(),
            None => Vec::new(),
        }
    }

    pub fn first_set(&self, symbol: Symbol) -> SymbolSet {
        self.first_sets.get(&symbol).unwrap().clone()
    }
    pub fn follow_set(&self, symbol: Symbol) -> SymbolSet {
        self.follow_sets.get(&symbol).unwrap().clone()
    }
}
