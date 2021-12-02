mod tree_iterator;
mod ngram;

use std::{collections::{HashMap, hash_map::DefaultHasher}, fs, hash::{Hash, Hasher}, path::Path};

use itertools::Itertools;

use tree_sitter::{Language, Node, Parser, Point, Tree};
use tree_iterator::TreeIterator;
use ngram::{NGramHashIterator, default};

const N : usize = 32;
const THRESH: f32 = 0.5;
const DIRS: [&str; 1] = [
    "/home/iracigt/Downloads/plagiarism-dataset/src/A2016/Z1/Z3/",
];


extern "C" { fn tree_sitter_c() -> Language; }

fn parse_file<P>(lang: Language, name: P) -> Result<Tree, String> 
where
    P : AsRef<Path>
{
    let mut parser = Parser::new();
    parser.set_language(lang).map_err(|e| e.to_string())?;
    let text = fs::read_to_string(name).map_err(|e| e.to_string())?;
    let parse = parser.parse(text, None);

    match parse {
        Some(tree) => Ok(tree),
        None => Err("parse error".to_string())
    }
}

fn populate(lang: Language, path: &Path, hashmap: &mut HashMap<u64, Vec<String>>) -> Result<(), String> {

    let file = path.file_stem().and_then(|s| s.to_str()).unwrap_or("<UNNAMED>");
    let tree = parse_file(lang, &path)?;
    let ids = TreeIterator::new(tree.walk()).map(|n| n.kind_id());
    let hashes = NGramHashIterator::new(ids, N, default);

    for h in hashes.unique() {
        match hashmap.get_mut(&h) {
            Some(v) => { v.push(file.to_string()) },
            None => { hashmap.insert(h, vec!(file.to_string())); }
        }
    }

    Ok(())
}

fn score(lang: Language, path: &Path, ngrammap: &HashMap<u64, Vec<String>>) -> Result<(String, u32, HashMap<String, Vec<(Point, Point)>>), String> {

    let mut hashmap: HashMap<String, Vec<(Point, Point)>> = HashMap::with_capacity(128);
    let mut n = 0;

    let file = path.file_stem().and_then(|s| s.to_str()).unwrap_or("<UNNAMED>");
    let tree = parse_file(lang, &path)?;

    let iter = NGramHashIterator::new(
        TreeIterator::new(tree.walk()), N,
        |_, i| {
            let v: Vec<&Node> = i.collect();
            
            let mut hasher = DefaultHasher::new();
            v.iter().for_each(|n|  n.kind_id().hash(&mut hasher));
            let hash = hasher.finish();
            
            let pos = (v.first().unwrap().start_position(), v.last().unwrap().end_position());

            (hash, pos)
    });

    for (h, p) in iter {

        n += 1;

        match ngrammap.get(&h) {
            Some(files) => for f in files {
                    if f != file || f == "<UNNAMED>" {
                    match hashmap.get_mut(f) {
                        Some(m) => m.push(p),
                        None => { hashmap.insert(f.to_string(), vec!(p)); }
                    }
                }
            },
            None => ()
        }
    }

    Ok((file.to_string(), n, hashmap))
}


fn main() {
    
    let language: Language = unsafe { tree_sitter_c() };

    let mut count = 0;
    let mut hashmap = HashMap::with_capacity(32768);

    for dir in DIRS {
        let paths = fs::read_dir(dir).unwrap();
    
        for r in paths {
            r
                .map_err(|e| e.to_string())
                .and_then(|e| populate(language, &e.path(), &mut hashmap))
                .unwrap_or_else(|e| println!("ERR: {}", e));
    
            count += 1;
        }
    }

    for dir in DIRS {
        let paths = fs::read_dir(dir).unwrap();
    
        for r in paths {
            r
                .map_err(|e| e.to_string())
                .and_then(|e| score(language, &e.path(), &hashmap))
                .map(|(f, n, m)| {
                    for (k, v) in m {
                        if (v.len() as f32) / (n as f32) > THRESH {
                            println!("{:0.03} {{{}.c,{}.c}} ({} / {})!", (v.len() as f32) / (n as f32), f, k, v.len(), n);
                        }
                    }
                }).unwrap_or_else(|e| println!("ERR: {}", e));
        }
    }

    // for (k,v) in hashmap.iter() {
    //     println!("{} {} 0x{:016x}",  v.len(), (v.len() as f64) / (count as f64), k,)
    // }
}
