#[derive(Debug)]
pub enum TreeError {
    InvalidReferenceError(String),
}

#[derive(Hash)]
pub struct NodeRef<T> {
    index: usize,
    id: usize,

    marker: std::marker::PhantomData<T>,
}

impl<T> Clone for NodeRef<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for NodeRef<T> {}

impl<T> NodeRef<T> {
    pub fn new(index: usize, id: usize) -> Self {
        NodeRef {
            index,
            id,
            marker: std::marker::PhantomData,
        }
    }
}

impl<T> PartialEq for NodeRef<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.index == other.id
    }
}

impl<T> Eq for NodeRef<T> {}

pub struct TreeNode<T> {
    node: T,
    id: usize,
    free: bool,
    parent: Option<NodeRef<T>>,
    children: Vec<NodeRef<T>>,
}

pub struct Tree<T> {
    nodes: Vec<TreeNode<T>>,
    free_indices: Vec<usize>,
}

impl<T> Tree<T> {
    pub fn new() -> Tree<T> {
        Tree {
            nodes: Vec::new(),
            free_indices: Vec::new(),
        }
    }

    /// Add the given node to the tree, returning a reference. If there are free
    /// slots in the backing vec, those are filled in before the vec is expanded.
    fn alloc_tree_node(&mut self, node: T) -> NodeRef<T> {
        let index;
        let id;
        if !self.free_indices.is_empty() {
            index = self.free_indices.pop().unwrap();
            id = self.nodes[index].id + 1;

            self.nodes[index] = TreeNode {
                node,
                id,
                free: false,
                parent: None,
                children: Vec::new(),
            };
        } else {
            index = self.nodes.len();
            id = 0;

            self.nodes.push(TreeNode {
                node,
                id,
                free: false,
                parent: None,
                children: Vec::new(),
            });
        }

        NodeRef::new(index, id)
    }

    // If the given node is orphaned, mark its slot as free
    fn check_dealloc(&mut self, node_ref: NodeRef<T>) {
        if let Some(tree_node) = self.nodes.get_mut(node_ref.index)
            && (tree_node.parent.is_none() && tree_node.children.is_empty())
            && !tree_node.free
        {
            tree_node.free = true;
            self.free_indices.push(node_ref.index);
        }
    }

    fn get_tree_node(&self, node_ref: NodeRef<T>) -> Option<&TreeNode<T>> {
        if let Some(node) = self.nodes.get(node_ref.index)
            && node.id == node_ref.id
            && !node.free
        {
            Some(node)
        } else {
            None
        }
    }

    fn get_tree_node_mut(&mut self, node_ref: NodeRef<T>) -> Option<&mut TreeNode<T>> {
        if let Some(node) = self.nodes.get_mut(node_ref.index)
            && node.id == node_ref.id
            && !node.free
        {
            Some(node)
        } else {
            None
        }
    }

    pub fn add_node_as_parent(
        &mut self,
        node: T,
        children_ref: &[NodeRef<T>],
    ) -> Result<NodeRef<T>, TreeError> {
        let node_ref = self.alloc_tree_node(node);

        for (i, child_ref) in children_ref.iter().enumerate() {
            if self.get_tree_node(*child_ref).is_some() {
                self.add_child(node_ref, *child_ref).unwrap();
            } else {
                let msg = format!("children_ref[{}]", i);
                return Err(TreeError::InvalidReferenceError(msg));
            }
        }

        Ok(node_ref)
    }

    pub fn add_node_as_child(
        &mut self,
        node: T,
        parent_ref: NodeRef<T>,
    ) -> Result<NodeRef<T>, TreeError> {
        let node_ref = self.alloc_tree_node(node);

        if self.get_tree_node(parent_ref).is_some() {
            self.add_child(parent_ref, node_ref).unwrap();
        } else {
            return Err(TreeError::InvalidReferenceError(String::from("parent_ref")));
        }

        Ok(node_ref)
    }

    pub fn add_child(
        &mut self,
        parent_ref: NodeRef<T>,
        child_ref: NodeRef<T>,
    ) -> Result<(), TreeError> {
        if self.get_tree_node(parent_ref).is_none() {
            Err(TreeError::InvalidReferenceError(String::from("parent_ref")))
        } else if self.get_tree_node(child_ref).is_none() {
            Err(TreeError::InvalidReferenceError(String::from("child_ref")))
        } else {
            // Remove child from its prev parent if it exitsts
            if let Some(prev_parent_ref) = self.nodes[child_ref.index].parent
                && let Some(prev_parent) = self.get_tree_node_mut(prev_parent_ref)
            {
                prev_parent.children.retain(|&c| c != child_ref);
                self.check_dealloc(prev_parent_ref);
            }

            // Set the child's new parent
            self.nodes[child_ref.index].parent = Some(child_ref);

            // Add the child to ote new parent's children
            self.nodes[parent_ref.index].children.push(child_ref);

            Ok(())
        }
    }

    pub fn remove_child(&mut self, child_ref: NodeRef<T>) -> Result<(), TreeError> {
        if self.get_tree_node(child_ref).is_none() {
            Err(TreeError::InvalidReferenceError(String::from("child_ref")))
        } else {
            // Remove child from its prev parent if it exitsts
            if let Some(prev_parent_ref) = self.nodes[child_ref.index].parent
                && let Some(prev_parent) = self.get_tree_node_mut(prev_parent_ref)
            {
                prev_parent.children.retain(|&c| c != child_ref);
                self.check_dealloc(prev_parent_ref);
            }

            // Set the child's new parent
            self.nodes[child_ref.index].parent = None;
            self.check_dealloc(child_ref);

            Ok(())
        }
    }

    pub fn get(&self, node_ref: NodeRef<T>) -> Option<&T> {
        self.get_tree_node(node_ref).map(|n| &n.node)
    }

    pub fn get_mut(&mut self, node_ref: NodeRef<T>) -> Option<&mut T> {
        self.get_tree_node_mut(node_ref).map(|n| &mut n.node)
    }

    pub fn parent(&self, node_ref: NodeRef<T>) -> Result<Option<NodeRef<T>>, TreeError> {
        match self.get_tree_node(node_ref) {
            Some(node) => Ok(node.parent),
            None => Err(TreeError::InvalidReferenceError(String::from("node_ref"))),
        }
    }

    pub fn children(&self, node_ref: NodeRef<T>) -> Result<&Vec<NodeRef<T>>, TreeError> {
        match self.get_tree_node(node_ref) {
            Some(node) => Ok(&node.children),
            None => Err(TreeError::InvalidReferenceError(String::from("node_ref"))),
        }
    }
}
