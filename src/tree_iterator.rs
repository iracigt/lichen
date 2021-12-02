use tree_sitter::{Node, TreeCursor};

pub struct TreeIterator<'a> {
    cursor: TreeCursor<'a>,
    current: Option<Node<'a>>,
}

impl<'a> TreeIterator<'a> {
    pub fn new(cursor: TreeCursor) -> TreeIterator {
        TreeIterator { current : Some(cursor.node()), cursor : cursor }
    }
}

impl<'a> Iterator for TreeIterator<'a> {
    
    type Item = tree_sitter::Node<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let ret = self.current;

        if ret.is_some() {
            let cursor= &mut self.cursor;

            if cursor.goto_first_child() {
                self.current = Some(cursor.node());
            } else {
                while !cursor.goto_next_sibling() {
                    if !cursor.goto_parent() {
                        // We've hit the root, so this is the last node
                        self.current = None;
                        return ret; 
                    }
                }
                self.current = Some(cursor.node());
            }
        }

        return ret;   
    }
}