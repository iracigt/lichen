
use std::fmt::Display;
use std::hash::Hash;
use std::ffi::OsStr;
use std::path::Path;
use std::fs;

#[derive(PartialEq, Eq, Hash, Clone)]
pub enum Origin {
    // A student currently in the course
    Student {
        username: String,
    },
    // Part of the known existing code
    Corpus {
        group: String, 
        desc: String,
    },
    // Part of the provided (allowable) code
    Allowed,
}

impl Origin {    
    pub fn student(username: &str) -> Self {
        Self::Student {
            username: username.to_string(),
        }
    }

    pub fn corpus(group: &str, desc: &str) -> Self {
        Self::Corpus {
            group: group.to_string(),
            desc: desc.to_string(),
        }
    }

    pub fn allowed() -> Self {
        Self::Allowed
    }
}

// Deriving PartialOrd is lexographic ordering
#[derive(PartialEq, Eq, Hash, Clone, Copy, PartialOrd, Ord)]
pub struct FilePos {
    pub line: u32, // Must come first
    pub char: u32  // Must come second
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct FileRange {
    pub start: FilePos,
    pub end: FilePos
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub enum Location {
    File {
        name: String,
        range: FileRange
    },
    Unknown
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct Source {
    orig: Origin,
    loc: Location,
}

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

pub trait Tokenizer<T> 
where
    T : Token {
    fn tokenize(&self, path: &Path, text: &str) -> Vec<(T, Location)>;
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
            Self::Student { username } => write!(f, "{}", username),
            Self::Corpus { group, desc } => write!(f, "{}:{}", group, desc),
            Self::Allowed => write!(f, "allowed"),
        }
    }
}

impl<T> CodeUnit<T> 
where
    T : Token
{
    pub fn from_path<F : Tokenizer<T>>(t: &F, path: &Path) -> Result<Self, String> {
        
        let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
        
        let tokens = t.tokenize(path, &text);
        
        Ok(Self {
            filename: path.file_name().and_then(OsStr::to_str).map(&str::to_string),
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
    pub fn single_file<F: Tokenizer<T>>(t: &F, origin: Origin, path: &Path) -> Result<Self, String> {
        Ok(Self { origin,  units : vec![ CodeUnit::from_path(t, path)? ] } )
    }

    pub fn files<'a, F: Tokenizer<T>, I: Iterator<Item = &'a Path>>(t: &F, origin: Origin, paths: I) -> Result<Self, String> {
        Ok(Self { origin,  units : paths.map(|p| CodeUnit::from_path(t, p)).collect::<Result<Vec<CodeUnit<T>>, String>>()? } )
    }

    pub fn units(&self) -> impl Iterator<Item = &CodeUnit<T>> {
        self.units.iter()
    }

    pub fn origin(&self) -> &Origin {
        &self.origin
    }
}
