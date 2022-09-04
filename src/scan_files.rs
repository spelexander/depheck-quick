use daachorse::{DoubleArrayAhoCorasick, DoubleArrayAhoCorasickBuilder, MatchKind};
use jwalk::{Parallelism, WalkDir};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::str;

/**
* Scans files in directory recursively looking for multiple search terms
*/
pub(crate) fn scan_files<'a>(
    path: &str,
    extensions: &HashSet<&str>,
    dep_list: &'a HashSet<String>,
) -> HashSet<&'a String> {
    let dep_by_id: HashMap<usize, &String> = dep_list.iter().enumerate().collect();

    let searcher: DoubleArrayAhoCorasick<u16> = DoubleArrayAhoCorasickBuilder::new()
        .match_kind(MatchKind::LeftmostLongest)
        .build(dep_list)
        .unwrap();

    let found_deps: HashSet<&String> = WalkDir::new(path)
        .parallelism(Parallelism::RayonNewPool(0))
        .into_iter()
        .par_bridge()
        .fold(HashSet::new, |mut elements, dir_entry_result| {
            let dir_entry = dir_entry_result.expect("Dir entry unable to be parsed");

            if !dir_entry.file_type().is_file() {
                return elements;
            }

            if !extensions.contains(dir_entry.path().extension().unwrap().to_str().unwrap()) {
                return elements;
            }

            let path = dir_entry.path();
            let file_content = fs::read(path).expect("Unable to read file content");

            for matcher in searcher.leftmost_find_iter(file_content) {
                let val = matcher.value();
                elements.insert(val);
            }

            elements
        })
        .reduce(HashSet::new, |mut a, b| {
            a.extend(b);
            a
        })
        .iter()
        .map(|v| {
            *dep_by_id
                .get(&(*v as usize))
                .expect("Invalid pattern index returned")
        })
        .collect();

    found_deps
}
