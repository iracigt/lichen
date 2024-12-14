use std::cmp::min;
use std::collections::HashMap;
use rustc_hash::{FxBuildHasher, FxHashMap};
use std::hash::BuildHasher;
use std::marker::PhantomData;

use itertools::Itertools;

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

pub struct Backend<T, B>
where
    T : Token,
    B: BuildHasher
{
    n: usize,
    map: FxHashMap<u64, Vec<Source>>,
    counts: FxHashMap<Origin, usize>,
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
            map : HashMap::with_capacity_and_hasher(1<<16, FxBuildHasher::default()),
            counts: HashMap::with_capacity_and_hasher(512, FxBuildHasher::default()),
            hash,
            _pd : PhantomData
        }
    }

    pub fn populate(&mut self, sub: &Submission<T>) {
        let origin = *sub.origin();

        let mut count: usize = 0;

        for u in sub.units() {
            let hashes = NGramHashIterator::new(u.tokens(), self.n, &self.hash);

            for (h, l) in hashes {
                let src = Source::new(origin, l);
                self.map.entry(h).or_insert_with(|| Vec::with_capacity(4)).push(src);
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

        // Here we maintain two different maps to track matches
        // Both use the origin of the matching hash as they key
        // `matchmap` tracks all pairs of locations of matching ngrams
        // `hitmap` simply counts how many matches there are with a given origin
        // The difference is `hitmap` will only count one match *per ngram*
        // ngrams that occur in multiple locations within the same files
        // must appear in `matchmap` once for each pair of locations


        let mut matchmap: HashMap<&Origin, Vec<(Location, Location)>, _> = HashMap::with_capacity_and_hasher(32, FxBuildHasher::default());
        let mut hitmap: HashMap<&Origin, usize, _> = HashMap::with_capacity_and_hasher(32, FxBuildHasher::default());

        // To avoid using two `NGramHashIterator`s, 
        // we're going to maintain a seen ngram hashset manually
        let mut tok_seen = HashMap::with_capacity_and_hasher(256, FxBuildHasher::default());

        for u in sub.units() {
            let hashes = NGramHashIterator::new(u.tokens(), self.n, &self.hash);
            for (h, l) in hashes {
                count += 1;
                match self.map.get(&h) {
                    Some(hits) => {
                        // There are many more efficient ways to do this
                        // But this was easy
                        if hits.iter().all(|s| !s.is_allowed()) {
                            for hit in hits {
                                if hit.origin() != this {
                                    // Profiling was showing this `Vec::push` to be a hotspot
                                    // We'll preallocate space to avoid the first few reallocs
                                    // Note: `Vec::with_capacity` needs to be in a thunk to avoid
                                    // spuriously allocating a vec on every access
                                    matchmap.entry(hit.origin()).or_insert_with(|| Vec::with_capacity(128)).push((l, *hit.location()));
                                }
                            }

                            // Only count hits the first time we see ngram `h`
                            if tok_seen.contains_key(&h) {
                                tok_seen.insert(h, ());
                                for hit in hits.iter().map(|s| s.origin()).unique() {
                                    *hitmap.entry(hit).or_insert(0) += 1;
                                }
                            }
                            
                        }
                    },
                    None => (),
                }
            }
        }

        hitmap.into_iter().filter_map(|(that, hits)| {
            let count_that = *self.counts.get(that).unwrap();
            let count_int = hits;
            let count_union = count + count_that;
            let count_min = min(count, count_that);
            
            if (count_int as f32) / (count_union as f32) > kj || (count_int as f32) / (count_min as f32) > km {
                Some(Match { 
                    this : this.clone(), 
                    that : that.clone(),
                    count_int,
                    count_this : count,
                    count_that })
            } else {
                None
            }
        }).collect()
    }
}