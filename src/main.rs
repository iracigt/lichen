mod ngram;
mod frontend;
mod syntect_frontend;
mod backend;

use std::collections::hash_map::RandomState;
use std::path::PathBuf;
use std::{ffi::OsStr, path::Path};
use std::fs;

use clap::{App, Arg};
use backend::Backend;
use frontend::{Submission, Origin};
use itertools::Itertools;
use onig::Regex;
use syntect::parsing::{Scope, SyntaxSet};
use syntect_frontend::SyntectFE;
use walkdir::WalkDir;

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
        Arg::with_name("blessed")
            .help("set of allowed sources")
            .short("b")
            .long("bless")
            .multiple(true)
            .number_of_values(1),
    )
    .arg(
        Arg::with_name("corpus")
            .help("set of disallowed sources")
            .short("c")
            .long("corpus")
            .multiple(true)
            .number_of_values(1),
    )
    .arg(
        Arg::with_name("filter")
            .help("filter source files to process by regex")
            .short("f")
            .long("filter")
            .multiple(false)
            .number_of_values(1),
    )
    .arg(
        Arg::with_name("input")
            .help("the input directory of student submissions")
            .index(1)
            .required(true),
    )
    .get_matches();

    let mut fe = SyntectFE::new(ps);

    matches.value_of("lang").map(|l| fe.set_lang(l));

    // TODO: Add CLI for these
    fe.add_ignore("meta");
    fe.add_ignore("comment");

    let blessed = matches.values_of("blessed").map_or(vec!(), |v| v.collect_vec());
    let allowed = blessed.iter().map(|d| fs::read_dir(d).unwrap()).flatten().filter_map(|r| {
        r.map_err(|e| e.to_string()).and_then( |e| {
            let path = e.path();
            let origin = frontend::Origin::allowed();            
            Submission::single_file(&fe, origin, &path)
        }).map_err(|e| println!("ERR: {}", e)).ok()
    }).collect_vec();

    let cursed = matches.values_of("corpus").map_or(vec!(), |v| v.collect_vec());
    let corpus = cursed.iter().map(|d| fs::read_dir(d).unwrap()).flatten().filter_map(|r| {
        r.map_err(|e| e.to_string()).and_then( |e| {
            let path = e.path();
            let group = path.parent().and_then(Path::to_str).unwrap_or("unknown");
            let desc = path.file_stem().and_then(OsStr::to_str).and_then(|f| f.split("@").next()).unwrap_or("unknown");
            let origin = frontend::Origin::corpus(group, desc);            
            Submission::single_file(&fe, origin, &path)
        }).map_err(|e| println!("ERR: {}", e)).ok()
    }).collect_vec();
    

    // let filt_regex = matches.value_of("filter").map(|r| Regex::new(r).expect("invalid regex"));
    // let dir = matches.value_of("input").unwrap();
    // let submissions : Vec<Submission<Scope>> = fs::read_dir(dir).unwrap().filter_map(|r| {
    //     let e = r.unwrap();
    //     let path = e.path();
    //     let name = path.file_name().and_then(OsStr::to_str);
    //     let user =  path.file_stem().and_then(OsStr::to_str).and_then(|f| f.split("@").next()).unwrap_or("unknown");
    //     let src = frontend::Source::student(user);
        
    //     if filt_regex.as_ref().and_then(|r| name.map(|n| r.is_match(n))).unwrap_or(true) {
    //         Submission::single_file(&fe, src, &path).map_err(|e| println!("ERR: {}", e)).ok()
    //     } else {
    //         None
    //     }
    // }).collect();


    let filt_regex = matches.value_of("filter").map(|r| Regex::new(r).expect("invalid regex"));
    let dir = matches.value_of("input").unwrap();
    let submissions : Vec<Submission<Scope>> = fs::read_dir(dir).unwrap().filter_map(|r| {
        let path = r.unwrap().path();

        if path.is_dir() {
            let user =  path.file_name().and_then(OsStr::to_str).unwrap_or("unknown");
            let origin = frontend::Origin::student(user);
            let walk = WalkDir::new(path);
            let f = walk.into_iter().map(|x| x.map_err(|e| e.to_string()))
                .filter_ok(|d| !d.path().is_dir())
                .map_ok(|r| r.into_path()).collect::<Result<Vec<PathBuf>, String>>().unwrap();
        
            let paths = f.iter().filter(|p| {
                let name = p.file_name();
                filt_regex.as_ref().and_then(|r| name.and_then(OsStr::to_str).map(|n| r.is_match(n))).unwrap_or(true)
            });

            Submission::files(&fe, origin, paths.map(PathBuf::as_path)).map_err(|e| println!("ERR: {}", e)).ok()
            
        } else {
            let name = path.file_name().and_then(OsStr::to_str);
            let user =  path.file_stem().and_then(OsStr::to_str).and_then(|f| f.split("@").next()).unwrap_or("unknown");
            let origin = frontend::Origin::student(user);

            if filt_regex.as_ref().and_then(|r| name.map(|n| r.is_match(n))).unwrap_or(true) {
                Submission::single_file(&fe, origin, &path).map_err(|e| println!("ERR: {}", e)).ok()
            } else {
                None
            }
        }
    }).collect();

    let n = matches.value_of("ngram").expect("No ngram length provided")
        .parse().expect("ngram length not an integer");

    let mut backend= Backend::new(n, RandomState::new());

    // submissions.first().unwrap().units().next().unwrap().tokens().for_each(|t| println!("{}", t));

    for sub in &allowed {
        backend.populate(sub);
    }

    for sub in &corpus {
        backend.populate(sub);
    }

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
            println!("{:0.03} {:0.03} {} {} {} {} {}", 
                m.jaccard_score(), m.altmin_score(), m.match_count(), m.this(), m.count_this, m.that(), m.count_that)
        }
    }
}