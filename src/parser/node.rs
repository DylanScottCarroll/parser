use std::collections::HashMap;

use crate::symbol::Symbol;
use crate::text::FileSlice;
use crate::tree::{NodeRef, Tree};

pub type ParseTree = Tree<ParseNode>;
pub type ParseNodeRef = NodeRef<SyntaxNode>;

#[derive(Clone)]
pub struct ParseNode {
    symbol: Symbol,
    pos: FileSlice,

    syntax_attributes: HashMap<String, NodeRef<SyntaxNode>>,
}

pub type SyntaxTree = Tree<SyntaxNode>;
pub type SyntaxNodeRef = NodeRef<ParseNode>;

#[derive(Clone)]
pub enum SyntaxNode {
    String(String),
    Number(f64),
    Node {
        head: NodeRef<SyntaxNode>,
        body: HashMap<String, NodeRef<SyntaxNode>>,
    },
    List(Vec<NodeRef<SyntaxNode>>),
}
