#[allow(dead_code)]

use std::cmp::min;
use std::collections::HashMap;
use rustc_hash::{FxBuildHasher, FxHashMap, FxHashSet};
use std::hash::BuildHasher;
use std::marker::PhantomData;

use itertools::Itertools;
use serde::{Deserialize, Serialize};

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

    /// Standard Jaccard "intersection over union" set similarity.
    pub fn jaccard_score(&self) -> f32 {
        (self.count_int as f32) / (self.union_count() as f32)
    }


    /// An alternative scoring function for when one set is much larger than the other.
    /// In that case, the Jaccard similarity is bounded by the ratio of sizes.
    /// This score is more appropriate for checking inclusion.
    pub fn altmin_score(&self) -> f32 {
        (self.count_int as f32) / (self.min_count() as f32)
    }
}


/// Factor the inner data out of the backend so that we can serialize it
/// without needing to serialize the hash function
#[derive(Deserialize, Serialize)]
struct BackendInner {
    n: usize,
    map: FxHashMap<u64, Vec<Source>>,
    counts: FxHashMap<Origin, usize>,
}

pub struct Backend<T, B>
where
    T : Token,
    B: BuildHasher
{
    inner: BackendInner,
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
            inner : BackendInner { 
                n,
                map : HashMap::with_capacity_and_hasher(1<<16, FxBuildHasher::default()),
                counts: HashMap::with_capacity_and_hasher(512, FxBuildHasher::default()),
            },
            hash,
            _pd : PhantomData
        }
    }

    pub fn populate(&mut self, sub: &Submission<T>) {
        let origin = *sub.origin();

        let mut seen = FxHashSet::default();

        for u in sub.units() {
            let hashes = NGramHashIterator::new(u.tokens(), self.inner.n, &self.hash);

            for (h, l) in hashes {
                let src = Source::new(origin, l);
                self.inner.map.entry(h).or_insert_with(|| Vec::with_capacity(4)).push(src);
                seen.insert(h);
            }
        }

        *self.inner.counts.entry(origin).or_insert(0) += seen.len();
    }

    pub fn score_cutoff(&self, sub: &Submission<T>, kj: f32, km: f32) -> Vec<Match> {
        let this = sub.origin();

        let mut hitmap: HashMap<&Origin, usize, _> = HashMap::with_capacity_and_hasher(32, FxBuildHasher::default());

        for u in sub.units() {
            let hashes = NGramHashIterator::new(u.tokens(), self.inner.n, &self.hash);
            for h in hashes.map(|(h, _l)| h).unique() {
                match self.inner.map.get(&h) {
                    Some(hits) => {
                        // There are many more efficient ways to do this
                        // But this was easy
                        if hits.iter().all(|s| !s.is_allowed()) {
                            for origin in hits.into_iter().map(Source::origin).unique() {
                                if origin != this {
                                    // Profiling was showing this `Vec::push` to be a hotspot
                                    // We'll preallocate space to avoid the first few reallocs
                                    // Note: `Vec::with_capacity` needs to be in a thunk to avoid
                                    // spuriously allocating a vec on every access
                                    *hitmap.entry(origin).or_insert(0) += 1;
                                }
                            }
                        }
                    },
                    None => (),
                }
            }
        }

        let count_this = *self.inner.counts.get(this).unwrap();

        hitmap.into_iter().filter_map(|(that, hits)| {
            let count_that = *self.inner.counts.get(that).unwrap();
            let count_int = hits;
            let count_union = count_this + count_that - hits;
            let count_min = min(count_this, count_that);
            
            if (count_int as f32) / (count_union as f32) > kj || (count_int as f32) / (count_min as f32) > km {
                Some(Match { 
                    this : this.clone(), 
                    that : that.clone(),
                    count_int,
                    count_this,
                    count_that })
            } else {
                None
            }
        }).collect()
    }

    #[allow(dead_code)]
    fn list_matches(&self, sub: &Submission<T>) -> Vec<(Location, Origin, Location)> {

        let mut matchmap: HashMap<&Origin, Vec<(Location, Location)>, _> = HashMap::with_capacity_and_hasher(32, FxBuildHasher::default());

        for u in sub.units() {
            let hashes = NGramHashIterator::new(u.tokens(), self.inner.n, &self.hash);
            for (h, l) in hashes {
                match self.inner.map.get(&h) {
                    Some(hits) => {
                        if hits.iter().all(|s| !s.is_allowed()) {
                            for hit in hits {
                                if hit.origin() != sub.origin() {
                                    // Profiling was showing this `Vec::push` to be a hotspot
                                    // We'll preallocate space to avoid the first few reallocs
                                    // Note: `Vec::with_capacity` needs to be in a thunk to avoid
                                    // spuriously allocating a vec on every access
                                    matchmap.entry(hit.origin()).or_insert_with(|| Vec::with_capacity(128)).push((l, *hit.location()));
                                }
                            }
                            
                        }
                    },
                    None => (),
                }
            }
        }

        matchmap.into_iter().flat_map(|(that, hits)| {
            hits.into_iter().map(|(src, dst)| (src, *that, dst))
        }).collect()
    }

    pub fn dump_table(&self) -> Vec<u8> {
        postcard::to_stdvec(&self.inner).expect("Could not serialize backend")
    }
}