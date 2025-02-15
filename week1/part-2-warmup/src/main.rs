/* The following exercises were borrowed from Will Crichton's CS 242 Rust lab. */

use std::collections::HashSet;

fn main() {
    println!("Hi! Try running \"cargo test\" to run tests.");
}

/// Takes a vector of numbers and some number n.
/// The function should return a new vector whose elements are the numbers
/// in the original vector v with n added to each number.
fn add_n(v: Vec<i32>, n: i32) -> Vec<i32> {
    let mut nv = vec![];
    for i in v.iter() {
        nv.push(i + n);
    }
    return nv;
}

/// Does the same thing as add_n, but modifies v directly (in place) and does not return anything.
fn add_n_inplace(v: &mut Vec<i32>, n: i32) {
    for mut i in v.iter_mut() {
        *i += n;
    }
}

/// removes duplicate elements from a vector in-place (i.e. modifies v directly).
/// If an element is repeated anywhere in the vector, you should keep the element that appears first.
fn dedup(v: &mut Vec<i32>) {
    let mut digits = HashSet::new();
    let mut i = 0;
    loop {
        if (i >= v.len()) {
            break;
        }
        if digits.contains(&v[i]) {
            v.remove(i);
        } else {
            digits.insert(v[i]);
            i += 1;
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_add_n() {
        assert_eq!(add_n(vec![1], 2), vec![3]);
    }

    #[test]
    fn test_add_n_inplace() {
        let mut v = vec![1];
        add_n_inplace(&mut v, 2);
        assert_eq!(v, vec![3]);
    }

    #[test]
    fn test_dedup() {
        let mut v = vec![3, 1, 0, 1, 4, 4];
        dedup(&mut v);
        assert_eq!(v, vec![3, 1, 0, 4]);
    }
}
