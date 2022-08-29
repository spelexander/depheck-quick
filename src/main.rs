extern crate clap;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::Instant;
use std::fs;
use std::format;
use serde::{Deserialize, Serialize};
use clap::{Arg, App};
use regex::Regex;
use prettydiff::diff_lines;
use jwalk::{Parallelism, WalkDir};
use rayon::prelude::*;
use daachorse::{DoubleArrayAhoCorasickBuilder, MatchKind};
use std::str;

const FILE_NAME: &str = "package.json";

#[derive(Serialize, Deserialize)]
#[allow(non_snake_case)]
struct Package {
    dependencies: BTreeMap<String, String>,
    devDependencies: BTreeMap<String, String>,
}

pub fn path_exists(path: &str) -> bool {
    fs::metadata(path).is_ok()
}

fn main() {
    let now = Instant::now();

    let matches = App::new("depster")
        .version("0.0.2")
        .author("spelexander")
        .about("Organise and optimise your package.json packages with depster")
        .arg(Arg::with_name("root")
            .short("r")
            .long("root")
            .help("Root directory path containing package.json file, defaults to ./")
            .takes_value(true))
        .arg(Arg::with_name("src")
            .short("s")
            .long("src")
            .help("Directory path containing the input source files, defaults to ./src")
            .takes_value(true))
        .arg(Arg::with_name("ext")
            .short("e")
            .long("ext")
            .help("Regex for file extensions to scan, defaults to (.tsx|.ts|.jsx|.js|.mjs|.cjs)")
            .takes_value(true))
        .get_matches();

    let root = matches.value_of("root").unwrap_or(".");

    let src = matches.value_of("src").unwrap_or("src");
    let src = format!("{}/{}", &root, src);

    let extension_matcher = matches.value_of("ext").unwrap_or(r"(\.tsx$|\.ts$|\.jsx$|\.js$|\.mjs$|\.cjs$)");
    let extension_matcher = Regex::new(&extension_matcher).expect("Invalid extension regex provided");

    let package_json_path = root.ends_with(FILE_NAME).then(|| root.to_owned()).unwrap_or(format!("{}/{}", &root, FILE_NAME));

    if !path_exists(&package_json_path) {
        panic!("{}/{} not found, doing nothing.", &root, FILE_NAME);
    } else if !path_exists(&src) {
        panic!("{} not found, doing nothing.", &src);
    }

    println!("🔬  scanning: {}", &src);

    let package: String = fs::read_to_string(package_json_path).expect("Unable to find file");
    let package: Package = serde_json::from_str(&package).expect("Unable to read file. Is it json?");

    println!("📦  dependencies: {} / {} dev", package.dependencies.keys().len(), package.devDependencies.keys().len());

    let deps: HashSet<String> = package.dependencies
        .keys()
        .cloned()
        .collect();

    // Scan dist files for dev dependencies
    let result = scan_files(&src, &extension_matcher, &deps);
    let mut new_deps: BTreeMap<String, String> = BTreeMap::new();
    let mut new_dev_deps: BTreeMap<String, String> = BTreeMap::new();

    for dep in result {
        let version = package.devDependencies.get(&*dep).unwrap_or(package.dependencies.get(&*dep).unwrap()).to_owned();

        // if it contains a type dependency in the wrong place
        let type_variant = format!("@types/{}", dep);
        if package.dependencies.contains_key(&*type_variant) {
            new_dev_deps.insert(type_variant.to_owned(), package.dependencies.get(&type_variant).unwrap().to_owned());
        }

        if package.devDependencies.contains_key(&*dep) {
            new_dev_deps.insert(dep.to_owned(), version);
        } else {
            new_deps.insert(dep.to_owned(), version);
        }
    }

    for (dep, version) in &package.devDependencies {
        new_dev_deps.insert(dep.to_owned(), version.to_owned());
    }

    println!("📦  used dependencies: {}", &new_deps.len());

    let new_package = Package {
        dependencies: new_deps,
        devDependencies: new_dev_deps,
    };

    let input = serde_json::to_string_pretty(&package).expect("Could not serialise input json");
    let output = serde_json::to_string_pretty(&new_package).expect("Could not serialise output json");

    println!("📦  proposed changes:");
    println!("{}", diff_lines(&input, &output));

    let elapsed = now.elapsed();
    println!("⌛  done in {:.2?}", elapsed);
}

fn scan_files(path: &str, matcher: &Regex, dep_list: &HashSet<String>) -> HashSet<String> {
    let mut dep_by_id: HashMap<u16, String> = HashMap::new();
    let mut patterns: Vec<String> = Vec::new();

    dep_list
        .iter()
        .enumerate()
        .for_each(|(id, s)| {
            patterns.push(s.to_owned());
            dep_by_id.insert(id as u16, s.to_owned());
        });

    let searcher = DoubleArrayAhoCorasickBuilder::new()
        .match_kind(MatchKind::LeftmostLongest)
        .build(&patterns)
        .unwrap();

    let found_deps: HashSet<u16> = WalkDir::new(path)
        .parallelism(Parallelism::RayonNewPool(0))
        .into_iter()
        .par_bridge()
        .filter_map(|dir_entry_result| {
            let mut found_deps: HashSet<u16> = HashSet::new();

            let dir_entry = dir_entry_result.ok()?;

            if !dir_entry.file_type().is_file() {
                return None;
            }

            if !matcher.is_match(&dir_entry.path().display().to_string()) {
                return None;
            }

            let path = dir_entry.path();
            let file_content = std::fs::read(path).ok()?;

            for matcher in searcher.leftmost_find_iter(file_content) {
                let val = matcher.value();
                found_deps.insert(val);
            }

            return Some(found_deps);
        })
        .flatten()
        .collect();

    let mut result: HashSet<String> = HashSet::new();
    for id in found_deps {
        let dep = dep_by_id.get(&id).expect("Invalid pattern index returned");
        result.insert(dep.to_owned());
    }

    return result;
}

