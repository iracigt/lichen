use std::ffi::OsStr;
use std::path::Path;

use crate::frontend::{Tokenizer, Token};

use syntect::parsing::{SyntaxSet, ScopeStack, ParseState, BasicScopeStackOp, Scope};
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

    fn tokenize(&self, path: &Path, text: &str) -> Vec<Scope> {
        
        let syntax = self.ss.find_syntax_by_name(&self.lang).or_else(|| {
            self.ss.find_syntax_by_extension(path.extension().and_then(OsStr::to_str).unwrap_or(""))
        }).unwrap(); // TODO: Fallback to plaintext and return words


        let mut parse_state = ParseState::new(syntax);
        let mut stack = ScopeStack::new();
        let mut tokens = vec!();
        
        for (_line_num, line) in LinesWithEndings::from(&text).enumerate() {
            let ops = parse_state.parse_line(line, &self.ss);
            for (_char_num, op) in ops {
                stack.apply_with_hook(&op, |bop, _| { 
                    match bop {
                        BasicScopeStackOp::Push(s) => {
                            if !self.ignore.iter().any(|i| i.is_prefix_of(s)) {
                                tokens.push(s);
                            }
                        },
                        _ => { },
                    }
                 });
            }
        }

        tokens
    }
}