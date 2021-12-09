use std::cmp::min;
use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

use itertools::Itertools;

use crate::frontend::{Source, Submission, Token};
use crate::ngram::{ElemIter, NGramHashIterator};

pub use crate::ngram::default as default_hash;

pub struct Match {
    this: Source,
    that: Source,
    count_int : usize,
    pub count_this : usize,
    pub count_that : usize,
}

impl Match {
    pub fn this(&self) -> &Source {
        &self.this
    }

    pub fn that(&self) -> &Source {
        &self.that
    }

    pub fn match_count(&self) -> usize {
        self.count_int
    }

    pub fn union_count(&self) -> usize {
        self.count_this + self.count_that - self.count_int
    }

    pub fn min_count(&self) -> usize {
        min(self.count_this, self.count_that)
    }

    pub fn jaccard_score(&self) -> f32 {
        (self.count_int as f32) / (self.union_count() as f32)
    }

    pub fn altmin_score(&self) -> f32 {
        (self.count_int as f32) / (self.min_count() as f32)
    }
}

pub struct Backend<T, K, H>
where
    T : Token,
    K : Eq + Hash + Clone,
    H : Fn(usize, ElemIter<&T>) -> K,
{
    n: usize,
    map: HashMap<K, Vec<Source>>,
    counts: HashMap<Source, usize>,
    hash: H,
    _pd: PhantomData<fn(T)->K>,
}

impl<T, H, K> Backend<T, K, H> 
where
    T : Token,
    K : Eq + Hash + Clone,
    H : for<'r> Fn(usize, ElemIter<'r, &T>) -> K,
{
    pub fn new(n: usize, hash: H) -> Self {
        Self { 
            n,
            map : HashMap::with_capacity(1<<16),
            counts : HashMap::with_capacity(512),
            hash,
            _pd: PhantomData,
        }
    }

    pub fn populate(&mut self, sub: &Submission<T>) {
        let src = sub.source().clone();

        let mut count: usize = 0;

        for u in sub.units() {
            let hashes = NGramHashIterator::new(u.tokens(), self.n, &self.hash);

            for h in hashes {
                let entry = self.map.entry(h).or_insert(vec![]);
                
                if !entry.contains(&src) {
                    entry.push(src.clone());
                    count += 1;
                }
            }
        }

        *self.counts.entry(src).or_insert(0) += count;
    }

    pub fn score(&self, sub: &Submission<T>) -> Vec<Match> {
        self.score_cutoff(sub, 0.0, 0.0) 
    }

    pub fn score_cutoff(&self, sub: &Submission<T>, kj: f32, km: f32) -> Vec<Match> {
        let this = sub.source();
        let mut count: usize = 0;

        let mut matchmap = HashMap::with_capacity(32);

        for u in sub.units() {
            let hashes = NGramHashIterator::new(u.tokens(), self.n, &self.hash);

            for h in hashes.unique() {
                count += 1;
                match self.map.get(&h) {
                    Some(hits) => {
                        // There are many more efficent ways to do this
                        // But this was easy
                        if hits.iter().all(|s| !s.is_allowed()) {
                            for hit in hits {
                                if hit != this {
                                    *matchmap.entry(hit).or_insert(0) += 1;
                                }
                            }
                        }
                    },
                    None => (),
                }
            }
        }

        matchmap.iter().filter_map(|(that, hits)| {
            let count_that = *self.counts.get(&that).unwrap();
            let count_int = *hits;
            let count_union = count + count_that - hits;
            let count_min = min(count, count_that);
            
            if (count_int as f32) / (count_union as f32) > kj || (count_int as f32) / (count_min as f32) > km {
                Some(Match { this : this.clone(), that : (*that).clone(), count_int, count_this : count, count_that })
            } else {
                None
            }
        }).collect()
    }
}