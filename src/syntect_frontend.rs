use std::ffi::OsStr;
use std::path::Path;

use crate::frontend::{Tokenizer, Token, Location, FileRange, FilePos};

use syntect::parsing::{SyntaxSet, ScopeStack, ParseState, BasicScopeStackOp, Scope, ScopeStackOp};
use syntect::util::LinesWithEndings;

pub struct SyntectFE {
    ss: SyntaxSet,
    ignore: Vec<Scope>,
    lang: String,
}

impl SyntectFE {

    pub fn new(ss: SyntaxSet) -> Self {
        Self {
            ss,
            ignore: vec!(),
            lang: String::new(),
        }
    }

    pub fn add_ignore(&mut self, prefix: &str) {
        let scope = Scope::new(prefix);
        let _ = scope.map(|s| self.ignore.push(s));
    }

    pub fn set_lang(&mut self, lang: &str) {
        self.lang = lang.to_string();
    }
}

impl Token for Scope { }

impl Tokenizer<Scope> for SyntectFE {

    fn tokenize(&self, path: &Path, text: &str) -> Vec<(Scope, Location)> {
        
        let syntax = self.ss.find_syntax_by_name(&self.lang).or_else(|| {
            self.ss.find_syntax_by_extension(path.extension().and_then(OsStr::to_str).unwrap_or(""))
        }).unwrap(); // TODO: Fallback to plaintext and return words

        let fname = path.file_name().and_then(OsStr::to_str).unwrap_or("unknown");
        let mut parse_state = ParseState::new(syntax);
        let mut tokens = vec!();

        let mut last_line = 0;
        
        for (line_num, line) in LinesWithEndings::from(&text).enumerate() {
            let ops = parse_state.parse_line(line, &self.ss);
            for (char_num, op) in ops {
                match op {
                    ScopeStackOp::Push(s) => {
                        let loc = Location::File { name: fname.to_string(), range: FileRange {
                            start: FilePos { line: line_num as u32, char: char_num as u32}, 
                            end: FilePos { line: 0, char: 0},
                        }};
                        tokens.push((s, loc))
                    },
                    ScopeStackOp::Pop(_) => { },
                    ScopeStackOp::Clear(_) => { },
                    ScopeStackOp::Restore => { },
                    ScopeStackOp::Noop => { },
                }
            }
            last_line = line_num as u32;
        }

        let (first, rest) = tokens.split_at_mut(1); 
        
        let mut last = &mut first[0];
        for t in rest.iter_mut() {
            match (&mut last.1, &t.1) {
                (
                    Location::File { name: _, range }, 
                    Location::File { name: _, range: FileRange { start: end, end: _ } }
                )  => {
                    range.end = *end
                }
                _ => { last.1 = Location::Unknown },
            }

            last = t;
        }

        tokens
    }
}
