use std::{path::{Path, PathBuf}, fs, ffi::OsStr};

use itertools::Itertools;
use onig::{Regex, Error};
use walkdir::WalkDir;

use crate::frontend::{Submission, Tokenizer, Token, Source};

pub struct Ingestor {
    filt_regex : Option<Regex>,
}

impl Ingestor {
    pub fn new() -> Self {
        Self { filt_regex : None }
    }

    pub fn set_filter(&mut self, pattern: &str) -> Result<(), Error> {
        Regex::new(pattern).map(|e| self.filt_regex = Some(e))
    }

    pub fn ingest_dir<T, F, P, S>(&self, fe: &F, path: P, source_fn: S) -> Vec<Submission<T>>
    where
    P: AsRef<Path>,
    T: Token,
    F: Tokenizer<T>,
    S: Fn(&Path) -> Source,
    {
        fs::read_dir(path).unwrap().filter_map(|r| {
            let path = r.unwrap().path();
    
            if path.is_dir() {
                let src = source_fn(&path);
                let walk = WalkDir::new(path);
                let f = walk.into_iter().map(|x| x.map_err(|e| e.to_string()))
                    .filter_ok(|d| !d.path().is_dir())
                    .map_ok(|r| r.into_path()).collect::<Result<Vec<PathBuf>, String>>().unwrap();
            
                let paths = f.iter().filter(|p| {
                    let name = p.file_name();
                    self.filt_regex.as_ref().and_then(|r| name.and_then(OsStr::to_str).map(|n| r.is_match(n))).unwrap_or(true)
                });
    
                Submission::files(fe, src, paths.map(PathBuf::as_path)).map_err(|e| println!("ERR: {}", e)).ok()
            } else {
                let name = path.file_name().and_then(OsStr::to_str);
                let src = source_fn(&path);
    
                if self.filt_regex.as_ref().and_then(|r| name.map(|n| r.is_match(n))).unwrap_or(true) {
                    Submission::single_file(fe, src, &path).map_err(|e| println!("ERR: {}", e)).ok()
                } else {
                    None
                }
            }
        }).collect()
    }
}