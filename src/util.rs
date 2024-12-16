use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StringRef(pub usize);

pub trait StringArena {
    fn add(&mut self, value: String) -> StringRef;
    fn get(&self, index: StringRef) -> Option<&str>;
}


pub struct VecStringArena {
    strings: Vec<String>,
}

impl VecStringArena {
    pub fn new() -> Self {
        Self {
            strings: Vec::new(),
        }
    }
}

impl StringArena for VecStringArena {
    fn add(&mut self, value: String) -> StringRef {
        self.strings.push(value);
        StringRef(self.strings.len() - 1)
    }

    fn get(&self, index: StringRef) -> Option<&str> {
        self.strings.get(index.0).map(|s| s.as_str())
    }
}




pub struct RandStringArena {
    strings: Vec<String>,
}


#[allow(dead_code)]
impl RandStringArena {
    pub fn new() -> Self {
        Self {
            strings: Default::default(),
        }
    }
}

impl StringArena for RandStringArena {
    fn add(&mut self, _value: String) -> StringRef {
        self.strings.push(format!("{:08x}", rand::random::<u32>()));
        StringRef(self.strings.len() - 1)
    }

    fn get(&self, index: StringRef) -> Option<&str> {
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