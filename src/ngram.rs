use std::collections::{VecDeque, hash_map::DefaultHasher, vec_deque};
use std::hash::{Hash, Hasher};

pub type ElemIter<'a, T> = vec_deque::Iter<'a, T>;
pub struct NGramHashIterator<T, I, F, H>
where
    F: Fn(usize, ElemIter<T>) -> H,
    I: Iterator<Item = T>
{
    iter: I,
    hash: F,
    reg: VecDeque<T>,
    n: usize,
}

impl<T, I: Iterator<Item = T>, F: Fn(usize, ElemIter<T>) -> H, H> NGramHashIterator<T, I, F, H> {
    pub fn new(iter: I, n: usize, hash: F) -> NGramHashIterator<T, I, F, H> {
        NGramHashIterator { iter: iter, reg : VecDeque::with_capacity(n), n: n, hash: hash }
    }
}

impl<T, I : Iterator<Item = T>, F : Fn(usize, ElemIter<T>) -> H, H> Iterator for NGramHashIterator<T, I, F, H> {
    type Item = H;

    fn next(&mut self) -> Option<Self::Item> {

        // We maintain the reg at len n-1 to simplify the priming logic
        while self.reg.len() < (self.n - 1) {
            // ? means we return None if len(iter) < n
            self.reg.push_back(self.iter.next()?);
        }

        self.reg.push_back(self.iter.next()?);
        let h = (self.hash)(self.n, self.reg.iter());
        self.reg.pop_front();

        Some(h)
    }
}   

pub fn ngramify<T : Clone>(_: usize, iter: ElemIter<T>) -> Vec<T> {
    iter.cloned().collect()
}

pub fn default<T : Hash>(_: usize, iter: ElemIter<T>) -> u64 {
    let mut hasher = DefaultHasher::new();
    iter.for_each(|x| x.hash(&mut hasher));
    hasher.finish()
}