mod ngram;
mod frontend;
mod syntect_frontend;
mod backend;

use std::ffi::OsStr;
use std::fs;

use backend::{Backend, default_hash};
use frontend::Submission;
use syntect::parsing::{Scope, SyntaxSet};
use syntect_frontend::SyntectFE;

const N : usize = 16;
const THRESH_J: f32 = 0.8;
const THRESH_A: f32 = 0.9;
const DIR: &str = "/home/iracigt/Downloads/plagiarism-dataset/src/A2016/Z1/Z3/";

fn main() {
    // Load these once at the start of your program
    let ps = SyntaxSet::load_defaults_newlines();

    // ps.syntaxes().iter().for_each(|s| println!("{}", s.name));

    let mut fe = SyntectFE::new(ps);

    fe.set_lang("C");
    fe.add_ignore("meta");
    fe.add_ignore("comment");

    let paths = fs::read_dir(DIR).unwrap();
    
    let submissions : Vec<Submission<Scope>> = paths.filter_map(|r| {
        r.map_err(|e| e.to_string()).and_then( |e| {
            let path = e.path();
            let user = path.file_stem().and_then(OsStr::to_str).and_then(|f| f.split("@").next()).unwrap_or("unknown");
            let src = frontend::Source::student(user);
            
            Submission::single_file(&fe, src, &path)
        }).map_err(|e| println!("ERR: {}", e)).ok()
    }).collect();

    let mut backend= Backend::new(N, |n, i| default_hash(n, i));

    // submissions.first().unwrap().units().next().unwrap().tokens().for_each(|t| println!("{}", t));

    for sub in &submissions {
        backend.populate(sub);
    }

    for sub in &submissions {
        let matches = backend.score_cutoff(sub, THRESH_J, THRESH_A);
        for m in matches {
            println!("{}, {} ({}) matches {} ({}): J = {:0.03}, A = {:0.03}, C = {}",  m.match_count(),
                m.this(), m.count_this, m.that(), m.count_that, m.jaccard_score(), m.altmin_score(), m.match_count());
        }
    }
}