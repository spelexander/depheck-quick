#![deny(clippy::all)]
extern crate clap;

use clap::{App, Arg};
use daachorse::{DoubleArrayAhoCorasick, DoubleArrayAhoCorasickBuilder, MatchKind};
use jwalk::{Parallelism, WalkDir};
use prettydiff::diff_lines;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::format;
use std::fs;
use std::io::{BufRead, BufReader};
use std::fs::File;
use std::process::exit;
use std::str;
use std::time::Instant;

const FILE_NAME: &str = "package.json";
const NOT_AN_IMPORT: &str = "import";

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

    let matches = App::new("depcheck-quick")
        .version("0.0.2")
        .author("spelexander")
        .about("Organise and optimise your package.json packages with depcheck-quick")
        .arg(
            Arg::with_name("root")
                .short("r")
                .long("root")
                .help("Root directory path containing package.json file, defaults to ./")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("src")
                .short("s")
                .long("src")
                .help("Directory path containing the input source files, defaults to ./src")
                .takes_value(true),
        )
        .get_matches();

    let root = matches.value_of("root").unwrap_or(".");

    let src = matches.value_of("src").unwrap_or("src");
    let src = format!("{}/{}", &root, src);

    let extensions = HashSet::from(["tsx", "ts", "jsx", "js", "mjs", "cjs"]);

    let package_json_path = root
        .ends_with(FILE_NAME)
        .then(|| root.to_owned())
        .unwrap_or(format!("{}/{}", &root, FILE_NAME));

    if !path_exists(&package_json_path) {
        println!("📦  {}/{} Not found, exiting", &root, FILE_NAME);
        exit(0);
    } else if !path_exists(&src) {
        println!("📦  {} Not found, exiting", &src);
    }

    let package: String = fs::read_to_string(package_json_path).expect("Unable to find file");
    let package: Package = serde_json::from_str(&package).expect("Unable to read file. Is it json?");

    println!("🔬  Scanning: {}", &src);
    println!(
        "📦  Dependencies: {} / {} dev",
        package.dependencies.keys().len(),
        package.devDependencies.keys().len()
    );

    let deps: HashSet<String> = package.dependencies.keys().cloned().collect();

    // Scan dist files for dev dependencies
    let result = scan_files(&src, &extensions, &deps);
    let mut new_deps: BTreeMap<String, String> = BTreeMap::new();
    let mut new_dev_deps: BTreeMap<String, String> = BTreeMap::new();

    for dep in result {
        let version = package
            .devDependencies
            .get(dep)
            .unwrap_or_else(|| package.dependencies.get(dep).unwrap())
            .to_owned();

        // if it contains a type dependency in the wrong place
        let type_variant = format!("@types/{}", dep);
        if package.dependencies.contains_key(&*type_variant) {
            new_dev_deps.insert(
                type_variant.to_owned(),
                package.dependencies.get(&type_variant).unwrap().to_owned(),
            );
        }

        if package.devDependencies.contains_key(dep) {
            new_dev_deps.insert(dep.to_owned(), version);
        } else {
            new_deps.insert(dep.to_owned(), version);
        }
    }

    println!("📦  Used dependencies: {}", &new_deps.len());

    for (dep, version) in &package.devDependencies {
        new_dev_deps.insert(dep.to_owned(), version.to_owned());
    }

    let new_package = Package {
        dependencies: new_deps,
        devDependencies: new_dev_deps,
    };

    let input = serde_json::to_string_pretty(&package).expect("Could not serialise input json");
    let output =
        serde_json::to_string_pretty(&new_package).expect("Could not serialise output json");

    println!("📦  Proposed changes:");
    println!("{}", diff_lines(&input, &output));
    let elapsed = now.elapsed();
    println!("⌛  Done in {:.2?}", elapsed);
}

fn scan_files<'a>(
    path: &str,
    matcher: &HashSet<&str>,
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
            let dir_entry = dir_entry_result.expect(":(");

            if !dir_entry.file_type().is_file() {
                return elements;
            }

            if !matcher.contains(dir_entry.path().extension().unwrap().to_str().unwrap()) {
                return elements;
            }

            let path = dir_entry.path();

            // Only read the file up to the first constant (assume imports are at the top and are es6 only)
            let f = BufReader::new(File::open(path).expect(":("));
            let mut file_content = Vec::<u8>::new();
            for line in f.lines() {
                let value = line.unwrap();
                if value.trim().len() > 0 && !value.contains(NOT_AN_IMPORT) {
                    break;
                }
                file_content.extend_from_slice(&value.as_bytes());
            }

            for matcher in searcher.leftmost_find_iter(file_content) {
                let val = matcher.value();
                elements.insert(val);
            }

            elements
        })
        .reduce(HashSet::new, |mut a, b| {
            a.extend(b);
            return a;
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
