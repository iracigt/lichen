mod util;
mod ngram;
mod frontend;
mod syntect_frontend;
mod backend;
mod alignment;


use std::collections::hash_map::RandomState;
use std::hash::Hash;
use std::path::PathBuf;
use std::u32;
use std::{ffi::OsStr, path::Path};
use std::fs::{self, File};

use alignment::{Alignment, HarshScoring};
use clap::{Parser, Subcommand};
use backend::Backend;
use frontend::{FilePos, FileRange, Location, Origin, Submission};
use itertools::Itertools;
use onig::Regex;
use syntect::parsing::{Scope, SyntaxSet};
use syntect_frontend::SyntectFE;
use util::{StringArena, VecStringArena};
use walkdir::WalkDir;

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, base64::Base64};

const DEF_N : &str = "16";
const DEF_THRESH_J: &str = "0.8";
const DEF_THRESH_A: &str = "0.9";
const DEF_THRESH_ALIGN: &str = "25";

use git_version::git_version;
const GIT_VERSION: &str = git_version!();

#[serde_as]
#[derive(Serialize, Deserialize)]
struct TableDump<'a> {
    lichen_data_version: &'a str,
    lichen_version: &'a str,
    #[serde(default)]
    comment: &'a str,
    #[serde_as(as = "Base64")]
    data: Vec<u8>,
}

#[derive(Parser, Debug)]
#[command(name = "lichen")]
struct LichenCLI {
    #[command(subcommand)]
    command: LichenCommand,
}

#[derive(Subcommand, Debug)]
enum LichenCommand {
    #[command(name = "analyze", about = "analyze a set of student submissions")]
    Analyze(AnalyzeConfig),
    #[command(name = "tokens", about = "dump tokens from a source file")]
    Tokens(DumpTokensConfig),
    #[command(name = "compare", about = "compare two submissions")]
    Compare(CompareConfig),
    #[command(name = "find", about = "find a code snippet among the submissions")]
    Find(FindConfig),
    #[command(name = "align", about = "show matching regions between two submissions")]
    Align(AlignConfig),
}

#[derive(clap::Args, Debug)]
struct FindConfig {
    #[arg(help = "the input directory of student submissions")]
    input: String,

    #[arg(help = "the code snippet to search for")]
    clue: String,
    
    #[arg(short = 'n', long = "ngram", default_value = DEF_N, help = "set length of n-grams to use")]
    n: usize,
    #[arg(short = 't', long = "thresh", default_value = DEF_THRESH_J, help = "set the threshold of matches to report")]
    thresh_j: f32,
    #[arg(short = 'a', long = "alt", default_value = DEF_THRESH_A, help = "set the inclusion threshold of matches to report")]
    thresh_a: f32,
    #[arg(short = 'l', long = "lang", help = "force a frontend language", long_help = lang_help())]
    lang: Option<String>,
    #[arg(short = 'f', long = "filter", help = "filter source files to process by regex")]
    filter: Option<String>,
    #[arg(short = 'b', long = "bless", help = "set of allowed sources")]
    blessed: Vec<String>,
}

#[derive(clap::Args, Debug)]
struct CompareConfig {
    #[arg(help = "the input directory of student submissions")]
    input: String,

    #[arg(help = "the first user to compare")]
    left: String,
    #[arg(help = "the second user to compare")]
    right: String,
    
    #[arg(short = 'n', long = "ngram", default_value = DEF_N, help = "set length of n-grams to use")]
    n: usize,
    #[arg(short = 'l', long = "lang", help = "force a frontend language", long_help = lang_help())]
    lang: Option<String>,
    #[arg(short = 'f', long = "filter", help = "filter source files to process by regex")]
    filter: Option<String>,
    #[arg(short = 'b', long = "bless", help = "set of allowed sources")]
    blessed: Vec<String>,
}

#[derive(clap::Args, Debug)]
struct DumpTokensConfig {
    input: String,
    #[arg(short = 'l', long = "lang", help = "force a frontend language", long_help = lang_help())]
    lang: Option<String>,
}

#[derive(clap::Args, Debug)]
struct AnalyzeConfig {
    #[arg(short = 'n', long = "ngram", default_value = DEF_N, help = "set length of n-grams to use")]
    n: usize,
    #[arg(short = 't', long = "thresh", default_value = DEF_THRESH_J, help = "set the threshold of matches to report")]
    thresh_j: f32,
    #[arg(short = 'a', long = "alt", default_value = DEF_THRESH_A, help = "set the inclusion threshold of matches to report")]
    thresh_a: f32,
    #[arg(short = 'l', long = "lang", help = "force a frontend language", long_help = lang_help())]
    lang: Option<String>,
    #[arg(short = 'b', long = "bless", help = "set of allowed sources")]
    blessed: Vec<String>,
    #[arg(short = 'c', long = "curse", help = "set of disallowed sources")]
    cursed: Vec<String>,
    #[arg(short = 'f', long = "filter", help = "filter source files to process by regex")]
    filter: Option<String>,
    #[arg(help = "the input directory of student submissions")]
    input: String,
    #[arg(short = 'd', long = "dump-table", help = "dump the (anonymized) hash table to a file")]
    dump: Option<String>,
}

#[derive(clap::Args, Debug)]
struct AlignConfig {
    #[arg(help = "the input directory of student submissions")]
    input: String,

    #[arg(help = "the first user to compare")]
    left: String,
    #[arg(help = "the second user to compare")]
    right: String,

    #[arg(short = 't', long = "thresh", default_value = DEF_THRESH_ALIGN, help = "set the threshold of matches to report")]
    thresh: f32,
    #[arg(long = "num", help = "list only top <num> matches")]
    num: Option<i32>,
    
    #[arg(short = 'l', long = "lang", help = "force a frontend language", long_help = lang_help())]
    lang: Option<String>,
    #[arg(short = 'f', long = "filter", help = "filter source files to process by regex")]
    filter: Option<String>,
}


fn main() -> Result<(), String> {

    let mut str_arena = VecStringArena::new();

    let args = LichenCLI::parse();

    match args.command {
        LichenCommand::Analyze(analyze_config) => analyze(&mut str_arena, analyze_config),
        LichenCommand::Tokens(token_config) => dump_tokens(&mut str_arena, token_config),
        LichenCommand::Compare(compare_config) => compare(&mut str_arena, compare_config),
        LichenCommand::Find(find_config) => find(&mut str_arena, find_config),
        LichenCommand::Align(align_config) => align(&mut str_arena, align_config),
    }
}

fn compare(str_arena: &mut VecStringArena, cfg: CompareConfig) -> Result<(), String> {
    
    let ps = SyntaxSet::load_defaults_newlines();
    let mut fe = SyntectFE::new(ps);

    if let Some(l) = cfg.lang {
        fe.set_lang(&l);
    }

    // TODO: Add CLI for these
    fe.add_ignore("meta");
    fe.add_ignore("comment");

    let allowed = cfg.blessed.iter().map(|d| fs::read_dir(d).unwrap()).flatten().filter_map(|r| {
        r.map_err(|e| e.to_string()).and_then( |e| {
            let path = e.path();
            let origin = frontend::Origin::allowed();            
            Submission::single_file(&fe, str_arena, origin, &path)
        }).map_err(|e| eprintln!("ERR: {}", e)).ok()
    }).collect_vec();

    let filter = cfg.filter.map(|r| Regex::new(&r).expect("invalid regex"));
    let pat = filter.as_ref();
    
    let mut get_subs = |username: &str| {
        fs::read_dir(&cfg.input).unwrap().filter_map(|r| {
            let path = r.unwrap().path();
    
            if path.is_dir() {
                let user = path.file_name().and_then(OsStr::to_str).unwrap_or("unknown").to_string();
    
                if user == username {
                    let origin = frontend::Origin::student(str_arena.add(user));
                    let walk = WalkDir::new(path);
                    let f = walk.into_iter().map(|x| x.map_err(|e| e.to_string()))
                        .filter_ok(|d| !d.path().is_dir())
                        .map_ok(|r| r.into_path()).collect::<Result<Vec<PathBuf>, String>>().unwrap();
                
                    let paths = f.iter().filter(|p| {
                        let name = p.file_name();
                        pat.and_then(|r| name.and_then(OsStr::to_str).map(|n| r.is_match(n))).unwrap_or(true)
                    });
    
                    Submission::files(&fe, str_arena, origin, paths.map(PathBuf::as_path)).map_err(|e| println!("ERR: {}", e)).ok()
                } else {
                    None
                }
            } else {
                let name = path.file_name().and_then(OsStr::to_str);
                let user = path.file_stem().and_then(OsStr::to_str)
                    .and_then(|f| f.split("@").next()).unwrap_or("unknown").to_string();
    
                if user == username
                    && pat.and_then(|r| name.map(|n| r.is_match(n))).unwrap_or(true) {
                    let origin = frontend::Origin::student(str_arena.add(user));
                    Submission::single_file(&fe, str_arena, origin, &path).map_err(|e| println!("ERR: {}", e)).ok()
                } else {
                    None
                }
            }
        }).collect()
    };

    let lefts : Vec<Submission<Scope>> = get_subs(&cfg.left);
    let rights : Vec<Submission<Scope>> = get_subs(&cfg.right);

    let mut backend = Backend::new(cfg.n, RandomState::new());

    for sub in &allowed {
        backend.populate(sub);
    }

    for sub in &rights {
        backend.populate(sub);
    }

    for (o1, loc1, o2, loc2) in lefts.iter().flat_map(|s| backend.list_matches(s).into_iter().map(
        |(l1, o, l2)| (s.origin(), l1, o, l2)
    )) {
        let s1 = o1.to_str(str_arena);
        let s2 = o2.to_str(str_arena);
        let FileRange { start: FilePos { line: l1start, char: _ }, end: FilePos { line: l1end, char: _ } } = loc1.range().unwrap();
        let FileRange { start: FilePos { line: l2start, char: _ }, end: FilePos { line: l2end, char: _ } } = loc2.range().unwrap();
        println!("{s1},{l1start},{l1end},{s2},{l2start},{l2end}");
    }

    Ok(())
}

fn dump_tokens(str_arena: &mut VecStringArena, cfg: DumpTokensConfig) -> Result<(), String> {

    let ps = SyntaxSet::load_defaults_newlines();
    let mut fe = SyntectFE::new(ps);

    if let Some(l) = cfg.lang {
        fe.set_lang(&l);
    }
    
    let file = Path::new(&cfg.input);
    let filename = file.file_name().and_then(OsStr::to_str).unwrap_or("unknown");
    let sub = Submission::single_file(&fe, str_arena, Origin::Allowed, file)?;

    for u in sub.units() {
        for (t, l) in u.tokens() {
            let range = l.range().unwrap();
            println!("{}:{:16}\t{}", filename, format!("{}", range), t)
        }
    }

    Ok(())
}

fn align(str_arena: &mut VecStringArena, cfg: AlignConfig) -> Result<(), String> {

    let ps = SyntaxSet::load_defaults_newlines();
    let mut fe = SyntectFE::new(ps);

    if let Some(l) = cfg.lang {
        fe.set_lang(&l);
    }
    
    // TODO: Add CLI for these
    fe.add_ignore("meta");
    fe.add_ignore("comment");

    let filter = cfg.filter.map(|r| Regex::new(&r).expect("invalid regex"));
    let pat = filter.as_ref();
    
    let mut get_subs = |username: &str| {
        fs::read_dir(&cfg.input).unwrap().filter_map(|r| {
            let path = r.unwrap().path();
    
            if path.is_dir() {
                let user = path.file_name().and_then(OsStr::to_str).unwrap_or("unknown").to_string();
    
                if user == username {
                    let origin = frontend::Origin::student(str_arena.add(user));
                    let walk = WalkDir::new(path);
                    let f = walk.into_iter().map(|x| x.map_err(|e| e.to_string()))
                        .filter_ok(|d| !d.path().is_dir())
                        .map_ok(|r| r.into_path()).collect::<Result<Vec<PathBuf>, String>>().unwrap();
                
                    let paths = f.iter().filter(|p| {
                        let name = p.file_name();
                        pat.and_then(|r| name.and_then(OsStr::to_str).map(|n| r.is_match(n))).unwrap_or(true)
                    });
    
                    Submission::files(&fe, str_arena, origin, paths.map(PathBuf::as_path)).map_err(|e| println!("ERR: {}", e)).ok()
                } else {
                    None
                }
            } else {
                let name = path.file_name().and_then(OsStr::to_str);
                let user = path.file_stem().and_then(OsStr::to_str)
                    .and_then(|f| f.split("@").next()).unwrap_or("unknown").to_string();
    
                if user == username
                    && pat.and_then(|r| name.map(|n| r.is_match(n))).unwrap_or(true) {
                    let origin = frontend::Origin::student(str_arena.add(user));
                    Submission::single_file(&fe, str_arena, origin, &path).map_err(|e| println!("ERR: {}", e)).ok()
                } else {
                    None
                }
            }
        }).collect()
    };

    let lefts : Vec<Submission<Scope>> = get_subs(&cfg.left);
    let rights : Vec<Submission<Scope>> = get_subs(&cfg.right);

    if lefts.is_empty() {
        return Err(format!("No submissions found for {}", cfg.left));
    }

    if rights.is_empty() {
        return Err(format!("No submissions found for {}", cfg.right));
    }

    // println!("{} {}", 
    //     lefts.first().unwrap().units().map(|u| u.tokens().count()).sum::<usize>(), 
    //     rights.first().unwrap().units().map(|u| u.tokens().count()).sum::<usize>()
    // );

    let alignment = Alignment::align::<HarshScoring, _>(
        lefts.first().expect("left submission not found"),
        rights.first().expect("right submission not found"),
        cfg.thresh as i32, cfg.num.unwrap_or(i32::MAX),
    );

    for m in alignment.matches() {

        let (left_name, left_range) = match m.left() {
            Location::File { name, range } => (str_arena.get(*name).unwrap_or("unknown"), Some(range)),
            _ => ("unknown", None),
        };
        let (right_name, right_range) = match m.right() {
            Location::File { name, range } => (str_arena.get(*name).unwrap_or("unknown"), Some(range)),
            _ => ("unknown", None),
        };
        
        let left = if let Some(range) = left_range {
            format!("{} {:16}", left_name, format!("{}", range))
        } else {
            format!("{} unknown", left_name)
        };

        let right = if let Some(range) = right_range {
            format!("{} {:16}", right_name, format!("{}", range))
        } else {
            format!("{} unknown", right_name)
        };
        
        println!("{} {} {}", m.score(), left, right);
    }

    Ok(())   
}


fn analyze(str_arena: &mut VecStringArena, cfg: AnalyzeConfig) -> Result<(), String> { 

    let ps = SyntaxSet::load_defaults_newlines();
    let mut fe = SyntectFE::new(ps);

    if let Some(l) = cfg.lang {
        fe.set_lang(&l);
    }

    // TODO: Add CLI for these
    fe.add_ignore("meta");
    fe.add_ignore("comment");

    let allowed = cfg.blessed.iter().map(|d| fs::read_dir(d).unwrap()).flatten().filter_map(|r| {
        r.map_err(|e| e.to_string()).and_then( |e| {
            let path = e.path();
            let origin = frontend::Origin::allowed();            
            Submission::single_file(&fe, str_arena, origin, &path)
        }).map_err(|e| eprintln!("ERR: {}", e)).ok()
    }).collect_vec();

    let corpus = cfg.cursed.iter().map(|d| fs::read_dir(d).unwrap()).flatten().filter_map(|r| {
        r.map_err(|e| e.to_string()).and_then( |e| {
            let path = e.path();
            let group = path.parent().and_then(Path::to_str).unwrap_or("unknown").to_string();
            let desc = path.file_stem().and_then(OsStr::to_str).and_then(|f| f.split("@").next()).unwrap_or("unknown").to_string();
            let origin = frontend::Origin::corpus(str_arena.add(group), str_arena.add(desc));            
            Submission::single_file(&fe, str_arena, origin, &path)
        }).map_err(|e| eprintln!("ERR: {}", e)).ok()
    }).collect_vec();


    let filter = cfg.filter.map(|r| Regex::new(&r).expect("invalid regex"));
    let pat = filter.as_ref();
    let submissions : Vec<Submission<Scope>> = fs::read_dir(cfg.input).unwrap().filter_map(|r| {
        let path = r.unwrap().path();

        if path.is_dir() {
            let user =  path.file_name().and_then(OsStr::to_str).unwrap_or("unknown").to_string();
            let origin = frontend::Origin::student(str_arena.add(user));
            let walk = WalkDir::new(path);
            let f = walk.into_iter().map(|x| x.map_err(|e| e.to_string()))
                .filter_ok(|d| !d.path().is_dir())
                .map_ok(|r| r.into_path()).collect::<Result<Vec<PathBuf>, String>>().unwrap();
        
            let paths = f.iter().filter(|p| {
                let name = p.file_name();
                pat.and_then(|r| name.and_then(OsStr::to_str).map(|n| r.is_match(n))).unwrap_or(true)
            });

            Submission::files(&fe, str_arena, origin, paths.map(PathBuf::as_path)).map_err(|e| println!("ERR: {}", e)).ok()
            
        } else {
            let name = path.file_name().and_then(OsStr::to_str);
            let user =  path.file_stem().and_then(OsStr::to_str).and_then(|f| f.split("@").next()).unwrap_or("unknown").to_string();

            if pat.and_then(|r| name.map(|n| r.is_match(n))).unwrap_or(true) {
                let origin = frontend::Origin::student(str_arena.add(user));
                Submission::single_file(&fe, str_arena, origin, &path).map_err(|e| println!("ERR: {}", e)).ok()
            } else {
                None
            }
        }
    }).collect();

    let mut backend = Backend::new(cfg.n, RandomState::new());

    for sub in &submissions {
        println!("{} {}", sub.origin().to_str(str_arena), sub.units().next().unwrap().tokens().count());
    }

    // submissions.first().unwrap().units().next().unwrap().tokens().for_each(|(t, l)| println!("{:?} {}", l.range().unwrap(), t));

    for sub in &allowed {
        backend.populate(sub);
    }

    for sub in &corpus {
        backend.populate(sub);
    }

    for sub in &submissions {
        backend.populate(sub);
    }

    if let Some(dump) = cfg.dump {
        let file = File::create(dump).expect("Could not create dump file");
        let tbl = backend.dump_table();
        serde_json::to_writer(file, &TableDump { data: tbl, 
            lichen_data_version: "0.0.0", 
            lichen_version: GIT_VERSION, 
            comment: "" 
        }).expect("Could not write dump data");
    }
    
    for sub in &submissions {
        let matches = backend.score_cutoff(sub, cfg.thresh_j, cfg.thresh_a);

        for m in matches {
            println!("{:0.03} {:0.03} {} {} {} {} {}", 
                m.jaccard_score(), m.altmin_score(), m.match_count(),
                m.this().to_str(str_arena), m.count_this,
                m.that().to_str(str_arena), m.count_that,
            )
        }
    }

    Ok(())
}


fn find(str_arena: &mut VecStringArena, cfg: FindConfig) -> Result<(), String> { 

    let ps = SyntaxSet::load_defaults_newlines();
    let mut fe = SyntectFE::new(ps);

    if let Some(l) = cfg.lang {
        fe.set_lang(&l);
    }

    // TODO: Add CLI for these
    fe.add_ignore("meta");
    fe.add_ignore("comment");

    let allowed = cfg.blessed.iter().map(|d| fs::read_dir(d).unwrap()).flatten().filter_map(|r| {
        r.map_err(|e| e.to_string()).and_then( |e| {
            let path = e.path();
            let origin = frontend::Origin::allowed();            
            Submission::single_file(&fe, str_arena, origin, &path)
        }).map_err(|e| eprintln!("ERR: {}", e)).ok()
    }).collect_vec();

    let filter = cfg.filter.map(|r| Regex::new(&r).expect("invalid regex"));
    let pat = filter.as_ref();
    let submissions : Vec<Submission<Scope>> = fs::read_dir(cfg.input).unwrap().filter_map(|r| {
        let path = r.unwrap().path();

        if path.is_dir() {
            let user =  path.file_name().and_then(OsStr::to_str).unwrap_or("unknown").to_string();
            let origin = frontend::Origin::student(str_arena.add(user));
            let walk = WalkDir::new(path);
            let f = walk.into_iter().map(|x| x.map_err(|e| e.to_string()))
                .filter_ok(|d| !d.path().is_dir())
                .map_ok(|r| r.into_path()).collect::<Result<Vec<PathBuf>, String>>().unwrap();
        
            let paths = f.iter().filter(|p| {
                let name = p.file_name();
                pat.and_then(|r| name.and_then(OsStr::to_str).map(|n| r.is_match(n))).unwrap_or(true)
            });

            Submission::files(&fe, str_arena, origin, paths.map(PathBuf::as_path)).map_err(|e| println!("ERR: {}", e)).ok()
            
        } else {
            let name = path.file_name().and_then(OsStr::to_str);
            let user =  path.file_stem().and_then(OsStr::to_str).and_then(|f| f.split("@").next()).unwrap_or("unknown").to_string();

            if pat.and_then(|r| name.map(|n| r.is_match(n))).unwrap_or(true) {
                let origin = frontend::Origin::student(str_arena.add(user));
                Submission::single_file(&fe, str_arena, origin, &path).map_err(|e| println!("ERR: {}", e)).ok()
            } else {
                None
            }
        }
    }).collect();

    let path = Path::new(&cfg.clue);

    let search : Submission<Scope> = if path.is_dir() {
            let user =  path.file_name().and_then(OsStr::to_str).unwrap_or("unknown").to_string();
            let origin = frontend::Origin::student(str_arena.add(user));
            let walk = WalkDir::new(path);
            let f = walk.into_iter().map(|x| x.map_err(|e| e.to_string()))
                .filter_ok(|d| !d.path().is_dir())
                .map_ok(|r| r.into_path()).collect::<Result<Vec<PathBuf>, String>>().unwrap();
        
            let paths = f.iter().filter(|p| {
                let name = p.file_name();
                pat.and_then(|r| name.and_then(OsStr::to_str).map(|n| r.is_match(n))).unwrap_or(true)
            });

            Submission::files(&fe, str_arena, origin, paths.map(PathBuf::as_path))?
        } else {
            let name = path.file_name().and_then(OsStr::to_str);
            let user =  path.file_stem().and_then(OsStr::to_str).and_then(|f| f.split("@").next()).unwrap_or("unknown").to_string();

            if pat.and_then(|r| name.map(|n| r.is_match(n))).unwrap_or(true) {
                let origin = frontend::Origin::student(str_arena.add(user));
                Submission::single_file(&fe, str_arena, origin, &path)?
            } else {
                return Err(format!("Could not find {}", cfg.clue));
            }
        };

    let mut backend = Backend::new(cfg.n, RandomState::new());

    for sub in &allowed {
        backend.populate(sub);
    }

    for sub in &submissions {
        backend.populate(sub);
    }
    
    let matches = backend.score_cutoff(&search, cfg.thresh_j, cfg.thresh_a);

    for m in matches {
        println!("{:0.03} {:0.03} {} {} {} {}", 
            m.jaccard_score(), m.altmin_score(), m.match_count(), 
            m.count_this, m.that().to_str(str_arena), m.count_that,
        )
    }

    Ok(())
}

fn lang_help() -> String { 
    let langs = SyntaxSet::load_defaults_newlines().syntaxes().iter().map(|s| format!("\"{}\"", s.name)).join(", ");   
    String::from("force a frontend language (default is based on file extension)\nAvailable languages:\n") + &langs
}