#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StringRef(usize);

pub struct StringArena {
    strings: Vec<String>,
}

impl StringArena {
    pub fn new() -> Self {
        StringArena {
            strings: Vec::new(),
        }
    }

    pub fn add(&mut self, value: String) -> StringRef {
        self.strings.push(value);
        StringRef(self.strings.len() - 1)
    }

    pub fn get(&self, index: StringRef) -> Option<&str> {
        self.strings.get(index.0).map(|s| s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_arena() {
        let mut arena = StringArena::new();
        let hello = arena.add("Hello".to_string());
        let world = arena.add("World".to_string());

        assert_eq!(arena.get(hello), Some("Hello"));
        assert_eq!(arena.get(world), Some("World"));
        assert_eq!(arena.get(StringRef(42)), None);
    }
}