#[allow(dead_code)]

use std::fmt::{Debug, Display};
use std::{cmp, hash::Hash};
use std::path::Path;
use std::fs;

use serde::{Deserialize, Serialize};

use crate::util::{StringArena, StringRef};

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Origin {
    // A student currently in the course
    Student {
        username: StringRef,
    },
    // Part of the known existing code
    Corpus {
        group: StringRef,
        desc: StringRef,
    },
    // Part of the provided (allowable) code
    Allowed,
}

impl Origin {    
    pub fn student(username: StringRef) -> Self {
        Self::Student { username }
    }

    pub fn corpus(group: StringRef, desc: StringRef) -> Self {
        Self::Corpus { group, desc }
    }

    pub fn allowed() -> Self {
        Self::Allowed
    }

    pub fn to_str<A: StringArena>(&self, arena: &A) -> String {
        match self {
            Self::Student { username } => arena.get(*username).unwrap_or("unknown").to_string(),
            Self::Corpus { group, desc } => format!("{}::{}", arena.get(*group).unwrap_or("unknown"), arena.get(*desc).unwrap_or("unknown")),
            Self::Allowed => "allowed".to_string(),
        }
    }
}

impl Debug for Origin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Student { username } => write!(f, "Student({:?})", username.0),
            Self::Corpus { group, desc } => write!(f, "Corpus({:?}, {:?})", group.0, desc.0),
            Self::Allowed => write!(f, "Allowed"),
        }
    }
}

// Deriving PartialOrd is lexicographic ordering
#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord)]
pub struct FilePos {
    pub line: u32, // Must come first
    pub char: u32  // Must come second
}

impl From<(u32, u32)> for FilePos {
    fn from((line, char): (u32, u32)) -> Self {
        Self { line, char }
    }
}

impl Into<(u32, u32)> for FilePos {
    fn into(self) -> (u32, u32) {
        (self.line, self.char)
    }
}

impl Display for FilePos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.char)
    }
}

impl Debug for FilePos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Copy)]
pub struct FileRange {
    pub start: FilePos,
    pub end: FilePos
}

impl From<((u32,u32), (u32,u32))> for FileRange {
    fn from((start, end): ((u32,u32), (u32,u32))) -> Self {
        Self { start: start.into(), end: end.into() }
    }
}

impl Into<((u32,u32), (u32,u32))> for FileRange {
    fn into(self) -> ((u32,u32), (u32,u32)) {
        (self.start.into(), self.end.into())
    }
}

impl Display for FileRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.start, self.end)
    }
}

impl Debug for FileRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Location {
    File {
        name: StringRef,
        range: FileRange
    },
    Unknown
}

impl Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Location::File { name, range } => write!(f, "{:?}:{}", name, range),
            Location::Unknown => write!(f, "unknown"),
        }
    }
}

impl Debug for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl Location {
    pub fn range(&self) -> Option<&FileRange> {
        match self {
            Location::File { range, .. } => Some(range),
            Location::Unknown => None,
        }
    }

    pub fn extend(&self, other: &Self) -> Self {
        match (self, other) {
            (Location::File { name: n1, range: r1 }, Location::File { name: n2, range: r2 }) if n1 == n2 => {
                Location::File { name: *n1, range: FileRange { start: cmp::min(r1.start, r2.start), end: cmp::max(r1.end, r2.end) } }
            }
            _ => Location::Unknown,
        }
    }
}


#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct Source {
    orig: Origin,
    loc: Location,
}



#[allow(dead_code)]
pub struct CodeUnit<T> {
    filename: Option<String>,
    contents: String,
    tokens: Vec<(T, Location)>,
}

pub struct Submission<T> {
    origin: Origin,
    units: Vec<CodeUnit<T>>,
}

pub trait Token : Hash { }

impl Token for String { }
impl Token for char { }

pub trait Tokenizer<T> 
where
    T : Token {
    fn tokenize<A: StringArena>(&self, str_arena: &mut A, path: &Path, text: &str) -> Vec<(T, Location)>;
}


impl Source {
    
    // pub fn student(username: &str) -> Source {
    //     Self::Student {
    //         username: username.to_string(),
    //     }
    // }

    // pub fn corpus(group: &str, desc: &str) -> Source {
    //     Self::Corpus {
    //         group: group.to_string(),
    //         desc: desc.to_string(),
    //     }
    // }

    // pub fn allowed() -> Source {
    //     Self::Allowed
    // }

    pub fn new(origin: Origin, loc: Location) -> Self {
        Self { orig: origin, loc }
    }

    pub fn origin(&self) -> &Origin {
        &self.orig
    }

    #[allow(dead_code)]
    pub fn location(&self) -> &Location {
        &self.loc
    }

    pub fn is_allowed(&self) -> bool {
        match &self.orig {
            Origin::Allowed => true,
            _ => false
        }
    }
}

impl Display for Origin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::Student { username } => write!(f, "{:?}", username),
            Self::Corpus { group, desc } => write!(f, "{:?}::{:?}", group, desc),
            Self::Allowed => write!(f, "allowed"),
        }
    }
}

impl<T> CodeUnit<T> 
where
    T : Token
{
    pub fn from_path<F : Tokenizer<T>, A: StringArena>(t: &F, str_arena: &mut A, path: &Path) -> Result<Self, String> {
        
        let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
        
        let tokens = t.tokenize(str_arena, path, &text);
        
        Ok(Self {
            filename: path.to_str().map(&str::to_string),
            contents: text,
            tokens,
        })
    }

    pub fn tokens(&self) -> impl Iterator<Item = &(T, Location)> {
        self.tokens.iter()
    }
}

impl<T> Submission<T> 
where
    T : Token
{
    #[allow(dead_code)]
    pub fn from_tokens<I: Iterator<Item = T>>(iter: I) -> Self {
        Self { origin: Origin::Allowed, units: vec![ CodeUnit {
            filename: None, 
            contents: String::new(), 
            tokens: iter.enumerate().map(|(i, t)| (t, Location::File { 
                name: StringRef(0), 
                range: FileRange { 
                    start: FilePos { line: 0, char: i as u32 },
                    end: FilePos { line: 0, char: i as u32 + 1 }
                }})).collect()
        } ] }
    }

    pub fn single_file<F: Tokenizer<T>, A: StringArena>(t: &F, str_arena: &mut A, origin: Origin, path: &Path) -> Result<Self, String> {
        Ok(Self { origin,  units : vec![ CodeUnit::from_path(t, str_arena, path)? ] } )
    }

    pub fn files<'a, F: Tokenizer<T>, I: Iterator<Item = &'a Path>, A: StringArena>(t: &F, str_arena: &mut A, origin: Origin, paths: I) -> Result<Self, String> {
            Ok(Self { origin,  units : paths.map(|p| CodeUnit::from_path(t, str_arena, p)).collect::<Result<Vec<CodeUnit<T>>, String>>()? } )
        }

    pub fn units(&self) -> impl Iterator<Item = &CodeUnit<T>> {
        self.units.iter()
    }

    pub fn origin(&self) -> &Origin {
        &self.origin
    }
}
