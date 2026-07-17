pub mod ll;
pub mod lr;
pub mod node;

use crate::language::{Language, TokenRule};
use std::rc::Rc;

pub trait Parser {
    fn new(language: Rc<Language>) -> Self;
}

pub struct Tokenizer {
    language: Rc<Language>,
}
