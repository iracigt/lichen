use std::cmp::min;
use std::collections::HashMap;
use std::hash::BuildHasher;
use std::marker::PhantomData;


use crate::frontend::{Source, Submission, Token, Origin, Location};
use crate::ngram::NGramHashIterator;

pub struct Match {
    this: Origin,
    that: Origin,
    count_int : usize,
    pub count_this : usize,
    pub count_that : usize,
}

impl Match {
    pub fn this(&self) -> &Origin {
        &self.this
    }

    pub fn that(&self) -> &Origin {
        &self.that
    }

    pub fn match_count(&self) -> usize {
        self.count_int
    }

    pub fn union_count(&self) -> usize {
        self.count_this + self.count_that
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

pub struct Backend<T, B>
where
    T : Token,
    B: BuildHasher
{
    n: usize,
    map: HashMap<u64, Vec<Source>>,
    counts: HashMap<Origin, usize>,
    hash: B,
    _pd: PhantomData<T>
}

impl<T, B> Backend<T, B> 
where
    T : Token,
    B: BuildHasher
{
    pub fn new(n: usize, hash: B) -> Self {
        Self { 
            n,
            map : HashMap::with_capacity(1<<16),
            counts : HashMap::with_capacity(512),
            hash,
            _pd : PhantomData
        }
    }

    pub fn populate(&mut self, sub: &Submission<T>) {
        let origin = sub.origin().clone();

        let mut count: usize = 0;

        for u in sub.units() {
            let hashes = NGramHashIterator::new(u.tokens(), self.n, &self.hash);

            for (h, l) in hashes {
                let src = Source::new(origin.clone(), l);
                let entry = self.map.entry(h).or_insert(vec![]);
                entry.push(src.clone());
                count += 1;
            }
        }

        *self.counts.entry(origin).or_insert(0) += count;
    }

    pub fn score(&self, sub: &Submission<T>) -> Vec<Match> {
        self.score_cutoff(sub, 0.0, 0.0) 
    }

    pub fn score_cutoff(&self, sub: &Submission<T>, kj: f32, km: f32) -> Vec<Match> {
        let this = sub.origin();
        let mut count: usize = 0;

        let mut matchmap = HashMap::with_capacity(32);

        for u in sub.units() {
            let hashes = NGramHashIterator::new(u.tokens(), self.n, &self.hash);

            for (h, _l) in hashes {
                count += 1;
                match self.map.get(&h) {
                    Some(hits) => {
                        // There are many more efficient ways to do this
                        // But this was easy
                        if hits.iter().all(|s| !s.is_allowed()) {
                            for hit in hits {
                                if hit.origin() != this {
                                    *matchmap.entry(hit.origin()).or_insert(0) += 1;
                                }
                            }
                        }
                    },
                    None => (),
                }
            }
        }

        matchmap.iter().filter_map(|(that, hits)| {
            let count_that = *self.counts.get(*that).unwrap();
            let count_int = *hits;
            let count_union = count + count_that;
            let count_min = min(count, count_that);
            
            if (count_int as f32) / (count_union as f32) > kj || (count_int as f32) / (count_min as f32) > km {
                Some(Match { this : this.clone(), that : (*that).clone(), count_int, count_this : count, count_that })
            } else {
                None
            }
        }).collect()
    }
}