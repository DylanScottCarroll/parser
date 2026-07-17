use std::{arch::x86_64::_XCR_XFEATURE_ENABLED_MASK, collections::HashMap, rc::Rc};

use crate::symbol::{Symbol, SymbolDomain, SymbolSet};

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
    rules: Vec<GrammarRule>,
    action_routines: Vec<ActionRoutine>,
    rules_to_routines_map: HashMap<GrammarRule, Vec<usize>>,

    rules_by_head: HashMap<Symbol, Vec<usize>>,
    rules_by_body: HashMap<Symbol, Vec<usize>>,

    first_sets: HashMap<Symbol, SymbolSet>,
    follow_sets: HashMap<Symbol, SymbolSet>,
}

impl Grammar {
    pub fn new(
        grammar_rules: Vec<(GrammarRule, Vec<ActionRoutine>)>,
        symbol_domain_size: usize,
    ) -> Grammar {
        let capacity = grammar_rules.len();
        let mut grammar = Grammar {
            rules: Vec::with_capacity(capacity),
            action_routines: Vec::new(),
            rules_to_routines_map: HashMap::new(),
            rules_by_head: HashMap::new(),
            rules_by_body: HashMap::new(),
            first_sets: HashMap::new(),
            follow_sets: HashMap::new(),
        };

        for (rule, routines) in grammar_rules {
            grammar.rules.push(rule.clone());
            let rule_id = grammar.rules.len();

            grammar.update_rule_by_symbol_maps(rule.clone(), rule_id);

            grammar.add_routine(rule, routines);
        }

        grammar.populate_first_and_follow_sets(symbol_domain_size);

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

    fn populate_first_and_follow_sets(&mut self, symbol_domain_size: usize) {
        // TODO: Implement this logic
    }
}

impl Grammar {
    pub fn get_rule(&self, index: usize) -> Option<&GrammarRule> {
        self.rules.get(index)
    }

    pub fn get_routines(&self, grammar_rule: &GrammarRule) -> Vec<&ActionRoutine> {
        match self.rules_to_routines_map.get(grammar_rule) {
            Some(indices) => indices
                .iter()
                .map(|&i| self.action_routines.get(i))
                .flatten()
                .collect(),
            None => Vec::new(),
        }
    }

    pub fn rules_by_head(&self, head: Symbol) -> Vec<&GrammarRule> {
        match self.rules_by_head.get(&head) {
            Some(indices) => indices
                .iter()
                .map(|&i| self.rules.get(i))
                .flatten()
                .collect(),
            None => Vec::new(),
        }
    }

    pub fn rules_by_body(&self, head: Symbol) -> Vec<&GrammarRule> {
        match self.rules_by_body.get(&head) {
            Some(indices) => indices
                .iter()
                .map(|&i| self.rules.get(i))
                .flatten()
                .collect(),
            None => Vec::new(),
        }
    }

    pub fn first_set(&self, symbol: Symbol) -> &SymbolSet {
        self.first_sets.get(&symbol).unwrap()
    }
    pub fn follow_set(&self, symbol: Symbol) -> &SymbolSet {
        self.follow_sets.get(&symbol).unwrap()
    }
}
