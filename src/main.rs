mod ngram;
mod frontend;
mod syntect_frontend;
mod backend;

use std::ffi::OsStr;
use std::fs;

use clap::{App, Arg};
use backend::{Backend, default_hash};
use frontend::Submission;
use syntect::parsing::{Scope, SyntaxSet};
use syntect_frontend::SyntectFE;

const DEF_N : &str = "16";
const DEF_THRESH_J: &str = "0.8";
const DEF_THRESH_A: &str = "0.9";

fn main() {

    let ps = SyntaxSet::load_defaults_newlines();

    let matches = App::new("Lichen")
    .version("0.1.0")
    .author("Grant Iraci <grantira@buffalo.edu>")
    .about("A FLOSS software similarity detector")
    .arg(
        Arg::with_name("threshold")
            .help("set the threshold of matches to report")
            .short("t")
            .long("thresh")
            .default_value(DEF_THRESH_J),
    )
    .arg(
        Arg::with_name("ngram")
            .help("set length of n-grams to use")
            .short("n")
            .long("ngram")
            .default_value(DEF_N),
    )
    .arg(
        Arg::with_name("alt-threshold")
            .help("set the inclusion threshold of matches to report")
            .short("a")
            .long("alt")
            .default_value(DEF_THRESH_A),
    )
    .arg(
        Arg::with_name("lang")
            .help("force a frontend language")
            .short("l")
            .long("lang")
            .long_help(
                &ps.syntaxes().iter().map(|s| &s.name).fold(
                    String::from("Available languages: "), 
                |mut a, b| { 
                    a.push_str(" \"");
                    a.push_str(b);
                    a.push_str("\"");
                    a
                })
            ).number_of_values(1),
    )
    .arg(
        Arg::with_name("input")
            .help("the input directory to use")
            .index(1)
            .required(true),
    )
    .get_matches();

    let mut fe = SyntectFE::new(ps);

    matches.value_of("lang").map(|l| fe.set_lang(l));

    // TODO: Add CLI for these
    fe.add_ignore("meta");
    fe.add_ignore("comment");

    let dir = matches.value_of("input").unwrap();
    let paths = fs::read_dir(dir).unwrap();
    
    let submissions : Vec<Submission<Scope>> = paths.filter_map(|r| {
        r.map_err(|e| e.to_string()).and_then( |e| {
            let path = e.path();
            let user = path.file_stem().and_then(OsStr::to_str).and_then(|f| f.split("@").next()).unwrap_or("unknown");
            let src = frontend::Source::student(user);
            
            Submission::single_file(&fe, src, &path)
        }).map_err(|e| println!("ERR: {}", e)).ok()
    }).collect();

    let n = matches.value_of("ngram").expect("No ngram length provided")
        .parse().expect("ngram length not an integer");

    let mut backend= Backend::new(n, |n, i| default_hash(n, i));

    // submissions.first().unwrap().units().next().unwrap().tokens().for_each(|t| println!("{}", t));

    for sub in &submissions {
        backend.populate(sub);
    }

    let thresh_j = matches.value_of("threshold").expect("No threshold provided")
        .parse::<f32>().expect("threshold invalid");
    let thresh_a = matches.value_of("alt-threshold").expect("No alt-threshold provided")
        .parse::<f32>().expect("alt-threshold invalid");
    
    for sub in &submissions {
        let matches = backend.score_cutoff(sub, thresh_j, thresh_a);
        for m in matches {
            println!("{:0.03} {:0.03} {} {} {}", 
                m.jaccard_score(), m.altmin_score(), m.match_count(), m.this(), m.that())
        }
    }
}