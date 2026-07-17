use std::rc::Rc;

#[derive(Clone, PartialEq, Eq)]
pub struct FileSlice {
    file: Rc<String>,
    start_line: usize,
    end_line: usize,
    start_col: usize,
    end_col: usize,
}
