
use std::fmt::Display;
use std::hash::Hash;
use std::ffi::OsStr;
use std::path::Path;
use std::fs;

#[derive(PartialEq, Eq, Hash, Clone)]
pub enum Source {
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

pub struct CodeUnit<T> {
    filename: Option<String>,
    contents: String,
    tokens: Vec<T>,
}

pub struct Submission<T> {
    src: Source,
    units: Vec<CodeUnit<T>>,
}

pub trait Token : Hash { }

pub trait Tokenizer<T> 
where
    T : Token {
    fn tokenize(&self, path: &Path, text: &str) -> Vec<T>;
}


impl Source {
    
    pub fn student(username: &str) -> Source {
        Self::Student {
            username: username.to_string(),
        }
    }

    pub fn corpus(group: &str, desc: &str) -> Source {
        Self::Corpus {
            group: group.to_string(),
            desc: desc.to_string(),
        }
    }

    pub fn allowed() -> Source {
        Self::Allowed
    }

    pub fn is_allowed(&self) -> bool {
        match &self {
            Self::Allowed => true,
            _ => false
        }
    }
}

impl Display for Source {
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

    pub fn tokens(&self) -> impl Iterator<Item = &T> {
        self.tokens.iter()
    }
}

impl<T> Submission<T> 
where
    T : Token
{
    pub fn single_file<F: Tokenizer<T>>(t: &F, src: Source, path: &Path) -> Result<Self, String> {
        Ok(Self { src,  units : vec![ CodeUnit::from_path(t, path)? ] } )
    }

    pub fn files<'a, F: Tokenizer<T>, I: Iterator<Item = &'a Path>>(t: &F, src: Source, paths: I) -> Result<Self, String> {
        Ok(Self { src,  units : paths.map(|p| CodeUnit::from_path(t, p)).collect::<Result<Vec<CodeUnit<T>>, String>>()? } )
    }

    pub fn units(&self) -> impl Iterator<Item = &CodeUnit<T>> {
        self.units.iter()
    }

    pub fn source(&self) -> &Source {
        &self.src
    }
}
