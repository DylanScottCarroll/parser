pub mod grammar;

use crate::symbol::{Symbol, SymbolDomain, SymbolType};

use self::grammar::{Grammar, GrammarRule};

pub struct TokenRule {
    symbol: Symbol,
    pattern: String,
}

pub struct Language {
    symbol_domain: SymbolDomain,
    token_rules: Vec<TokenRule>,
    grammar: Grammar,
}

impl Language {
    fn parser_language() {
        let sd = SymbolDomain::new(vec![
            (String::from("Symbol1"), SymbolType::Terminal),
            (String::from("Symbol2"), SymbolType::Nonterminal),
        ]);

        let tr = vec![
            TokenRule {
                symbol: sd.by_name("Symbol1").unwrap(),
                pattern: String::from("pattern1"),
            },
            TokenRule {
                symbol: sd.by_name("Symnol2").unwrap(),
                pattern: String::from("pattern2"),
            },
        ];

        let gr = Grammar::new(vec![], sd.count())
    }
}
