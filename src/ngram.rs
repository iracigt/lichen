use std::collections::VecDeque;
use std::hash::{Hash, Hasher, BuildHasher};

use crate::frontend::{Location, FileRange};

pub struct NGramHashIterator<'a, T, I, B>
where
    T : Hash + 'a,
    B: BuildHasher,
    I: Iterator<Item = &'a (T, Location)> + 'a
{
    iter: I,
    hash: &'a B,
    reg: VecDeque<&'a (T, Location)>,
    n: usize,
}

impl<'a, T : Hash, I: Iterator<Item = &'a (T, Location)>, B: BuildHasher,> NGramHashIterator<'a, T, I, B> {
    pub fn new(iter: I, n: usize, hash: &'a B) -> NGramHashIterator<'a, T, I, B> {
        NGramHashIterator { iter, reg : VecDeque::with_capacity(n), n, hash }
    }
}

impl<'a, T : Hash, I : Iterator<Item = &'a (T, Location)>, B: BuildHasher> Iterator for NGramHashIterator<'a, T, I, B> {
    type Item = (u64, Location);

    fn next(&mut self) -> Option<Self::Item> {

        // We maintain the reg at len n-1 to simplify the priming logic
        while self.reg.len() < (self.n - 1) {
            // ? means we return None if len(iter) < n
            self.reg.push_back(self.iter.next()?);
        }

        self.reg.push_back(self.iter.next()?);

        // let h = (self.hash)(self.n, self.reg.iter().map(|(t, _)| t));

        let mut loc = self.reg[0].1.clone();
        
        let mut hasher = self.hash.build_hasher();

        for (t, l) in self.reg.iter() {
            t.hash(&mut hasher);
            loc = merge_loc(&loc, l);
        }
        
        self.reg.pop_front();

        Some((hasher.finish(), loc))
    }
}


fn merge_loc(l1: &Location, l2: &Location) -> Location{
    if let Location::File { name: n1, range: r1 } = l1 {
        if let Location::File { name: n2, range: r2 } = l2 {
            if n1 == n2 {
                Location::File { 
                    name: n1.clone(),
                    range: FileRange {
                        start: r1.start.min(r2.start),
                        end: r1.end.max(r2.end)
                    }
                }
            } else {
                Location::Unknown
            }
        } else {
            Location::Unknown
        }
    } else {
        Location::Unknown
    }       
}

// pub fn default<T : Hash, I : Iterator<Item = T>>(_: usize, iter: I) -> u64 {
//     let mut hasher = DefaultHasher::new();
//     iter.for_each(|x| x.hash(&mut hasher));
//     hasher.finish()
// }