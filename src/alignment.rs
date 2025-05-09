#![allow(unused)]

use std::{cmp, usize};

use crate::frontend::{CodeUnit, Location, Submission, Token};

pub trait AlignmentScoring<T> {
    const GAP_PENALTY: i32;
    fn score(t1: &T, t2: &T) -> i32;
}

pub struct UnitScoring { }

impl<T: Eq> AlignmentScoring<T> for UnitScoring {
    const GAP_PENALTY: i32 = 1;

    fn score(t1: &T, t2: &T) -> i32 {
        if t1 == t2 {
            1
        } else {
            -1
        }
    }
}

pub struct HarshScoring { }

impl<T: Eq> AlignmentScoring<T> for HarshScoring {
    const GAP_PENALTY: i32 = 6;

    fn score(t1: &T, t2: &T) -> i32 {
        if t1 == t2 {
            1
        } else {
            -4
        }
    }
}


fn argmax<T: PartialOrd>(a: &Vec<Vec<T>>, max_i: usize, max_j: usize, thresh: &T) -> Option<(usize, usize)> {
    let mut max = thresh;
    let mut argmax = (usize::MAX, usize::MAX);

    for (i, row) in a.iter().enumerate().take(max_i) {
        for (j, v) in row.iter().enumerate().take(max_j) {
            if v > max {
                max = v;
                argmax = (i, j);
            }
        }
    }

    if argmax == (usize::MAX, usize::MAX) {
        None
    } else {
        Some(argmax)
    }
}

/// Find the location of the max value in the matrix,
/// using a cache `c` of the argmax of each row.
/// Invalidate cache entry with `usize::MAX` when any element of that row is modified.
fn rowcache_argmax<T: PartialOrd>(a: &Vec<Vec<T>>, c: &mut Vec<usize>, max_i: usize, max_j: usize, thresh: &T) -> Option<(usize, usize)> {

    for (i, line) in c.iter_mut().enumerate().take(max_i) {
        if *line == usize::MAX {
            let mut max = thresh;
            let mut argmax = 0;
            let row = &a[i];

            for (j, v) in row.iter().enumerate().take(max_j) {
                if v > max {
                    max = v;
                    argmax = j;
                }
            }

            *line = argmax;
        }
    }

    let mut max = thresh;
    let mut argmax = (usize::MAX, usize::MAX);
    for (i, line) in c.iter().enumerate().take(max_i) {
        let j = *line;
        let v = &a[i][j];
        if v > max {
            max = v;
            argmax = (i, j);
        }
    }

    if argmax == (usize::MAX, usize::MAX) {
        None
    } else {
        Some(argmax)
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
enum Score {
    Match(i32),
    Delete(i32),
    Insert(i32),
    Empty,
    Dead
}

impl Score {
    fn score(&self) -> i32 {
        match self {
            Score::Match(a) => *a,
            Score::Delete(a) => *a,
            Score::Insert(a) => *a,
            Score::Empty => 0,
            Score::Dead => 0,
        }
    }

    fn extend(&self, x: i32) -> Self {
        Score::Match(self.score() + x)
    }

    fn delete(&self, gap_penalty: i32) -> Self {
        Score::Delete(self.score() - gap_penalty)
    }

    fn insert(&self, gap_penalty: i32) -> Self {
        Score::Insert(self.score() - gap_penalty)
    }
}

impl Ord for Score {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.score().cmp(&other.score())
    }
}

impl PartialOrd for Score {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    left: Location,
    right: Location,
    score: i32,
}

impl Match {
    pub fn left(&self) -> &Location {
        &self.left
    }

    pub fn right(&self) -> &Location {
        &self.right
    }

    pub fn score(&self) -> i32 {
        self.score
    }
}

#[derive(Debug)]
pub struct Alignment {
    matches: Vec<Match>,
}

impl Alignment {
    pub fn align<S: AlignmentScoring<T>, T: Token + Eq>(left: &Submission<T>, right: &Submission<T>, thresh: i32, max_matches: i32) -> Self {

        let left_tok: Vec<_> = left.units().flat_map(CodeUnit::tokens).collect();
        let right_tok: Vec<_> = right.units().flat_map(CodeUnit::tokens).collect();

        let n_left = left_tok.len();
        let n_right = right_tok.len();

        let mut matrix = vec![vec![Score::Empty; n_right + 1]; n_left + 1];

        for (i, (t1, _)) in left_tok.iter().enumerate() {
            for (j, (t2, _)) in right_tok.iter().enumerate() {
                let score_match = matrix[i][j].extend(S::score(t1, t2));
                let score_delete = matrix[i + 1][j].delete(S::GAP_PENALTY);
                let score_insert = matrix[i][j + 1].insert(S::GAP_PENALTY);
                matrix[i + 1][j + 1] = cmp::max(score_delete, score_insert).max(score_match).max(Score::Empty);
            }
        }

        let mut matches = vec!();
        let mut row_maxes = vec![usize::MAX; n_left + 1];

        while let Some((max_i, max_j)) = rowcache_argmax(&matrix, &mut row_maxes, n_left + 1, n_right + 1, &Score::Match(thresh)) {

            if matches.len() as i32 >= max_matches {
                break;
            }

            let mut i = max_i-1;
            let mut j = max_j-1;

            let mut left_range = left_tok[i].1.clone();
            let mut right_range = right_tok[j].1.clone();

            let mut score = matrix[max_i][max_j].score();

            while i > 0 && j > 0 {
                match matrix[i][j] {
                    Score::Match(_) => {
                        i -= 1;
                        j -= 1;
                        left_range = left_range.extend(&left_tok[i].1);
                        right_range = right_range.extend(&right_tok[j].1);
                    }
                    Score::Delete(_) => {
                        j -= 1;
                        right_range = right_range.extend(&right_tok[j].1);
                    }
                    Score::Insert(_) => {
                        i -= 1;
                        left_range = left_range.extend(&left_tok[i].1);
                    }
                    Score::Empty => break,
                    Score::Dead => {
                        score = 0;
                        break;
                    },
                }
            }

            // Box off matched region to prevent overlaps
            for row in &mut matrix[i..=max_i] {
                for cell in &mut row[j..=max_j] {
                    *cell = Score::Dead;
                }
            }

            for e in &mut row_maxes[i..=max_i] {
                *e = usize::MAX;
            }

            matrix[max_i][max_j] = Score::Dead;

            if score > 0 {
                matches.push(Match {
                    left: left_range,
                    right: right_range,
                    score,
                });
            }
        };

        Alignment { matches }
    }

    pub fn matches(&self) -> &[Match] {
        &self.matches
    }
}

#[cfg(test)]
mod tests {
    use crate::alignment::{Alignment, UnitScoring};
    use crate::frontend::{FileRange, Location, Submission};

    #[test]
    fn test_complex() {
        let seq1 = "ABCABCA";
        let seq2 = "ABC";

        let left = Submission::from_tokens(seq1.chars());
        let right = Submission::from_tokens(seq2.chars());

        let alignment = Alignment::align::<UnitScoring, _>(&left, &right, 0, i32::MAX);
        println!("{:?}", alignment.matches);
        for m in &alignment.matches {
            print!("S {}", m.score);
            if let Location::File { name: _, range: FileRange { start, end } } = m.left {
                let s = start.char as usize;
                let e = end.char as usize;
                print!("\tL {}-{}: {}", s, e, &seq1[s..e]);
            }
            if let Location::File { name: _, range: FileRange { start, end } } = m.right {
                let s = start.char as usize;
                let e = end.char as usize;
                print!("\tR {}-{}: {}", s, e, &seq2[s..e]);
            }
            println!();
        }
    }
}