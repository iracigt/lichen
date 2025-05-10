use std::ffi::OsStr;
use std::path::Path;
use std::vec;

use crate::frontend::{Tokenizer, Token, Location, FileRange, FilePos};
use crate::util::StringArena;

use syntect::parsing::{SyntaxSet, ParseState, Scope, ScopeStackOp};
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

    fn tokenize<A: StringArena>(&self, str_arena: &mut A, path: &Path, text: &str) -> Vec<(Scope, Location)> {
        
        let syntax = self.ss.find_syntax_by_name(&self.lang).or_else(|| {
            self.ss.find_syntax_by_extension(path.extension().and_then(OsStr::to_str).unwrap_or(""))
        }).expect(&format!(
            "Unable to autodetect language of {}. Try specifying language with --lang",
            path.to_str().unwrap_or("???")
        )); // TODO: Fallback to plaintext and return words

        let fname = path.to_str().unwrap_or("unknown").to_string();
        let fname_ref = str_arena.add(fname);
        let mut parse_state = ParseState::new(syntax);
        let mut tokens = vec!();
        let mut loc_stack = vec!();

        
        for (line_num, line) in LinesWithEndings::from(&text).enumerate() {
            let ops = parse_state.parse_line(line, &self.ss);
            for (char_num, op) in ops {
                match op {
                    ScopeStackOp::Push(s) => {
                        let start = FilePos { line: line_num as u32 + 1, char: char_num as u32 + 1};
                        loc_stack.push((s, start))
                    },
                    ScopeStackOp::Pop(count) => {
                        for _ in 0..count {
                            let (s1, start) = loc_stack.pop().unwrap();
                            let end = FilePos { line: line_num as u32 + 1, char: char_num as u32 + 1};
                            let loc = Location::File { 
                                name: fname_ref, 
                                range: FileRange { start, end }
                            };
                            tokens.push((s1, loc));
                        }
                    },
                    ScopeStackOp::Clear(_amt) => { 
                        #[cfg(debug_assertions)]{ todo!("handle ScopeStackOp::Clear({:?})", _amt) } 
                    },
                    ScopeStackOp::Restore => { 
                        #[cfg(debug_assertions)]{ todo!("handle ScopeStackOp::Restore") }
                    },
                    ScopeStackOp::Noop => { },
                }
            }
        }


        #[cfg(debug_assertions)]
        {
            let n = path.file_name().and_then(OsStr::to_str).unwrap_or("unknown");
            if loc_stack.len() != 1 {
                eprintln!("WARNING: non-empty token stack for {:?}: {:?}", n, loc_stack);
            } else if loc_stack.first().map(|(s,l)| (s.build_string(), l)) != Some(("source.ocaml".to_string(), &FilePos { line: 1, char: 1 })) {
                eprintln!("WARNING: invalid completed token stack for {:?}: {:?}", n, loc_stack);
            }
        }

        // debug_assert!(loc_stack.is_empty(), "non-empty token stack {:?}", loc_stack);

        tokens
    }
}
