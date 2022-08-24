extern crate clap;

use std::collections::{BTreeMap, HashSet};
use std::time::Instant;
use std::fs;
use std::format;
use serde::{Deserialize, Serialize};
use clap::{Arg, App};
use regex::Regex;
use prettydiff::diff_lines;
use jwalk::{WalkDir};

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

        if dep.starts_with("@types") || package.devDependencies.contains_key(&*dep) {
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
    let mut found_deps: HashSet<String> = HashSet::new();
    let mut check_list: HashSet<String> = dep_list.clone();

    let parallel_reads = WalkDir::new(path);

    // loop over every file/dir and dive deeper or scan for deps
    for source_path in parallel_reads {
        if check_list.len() == 0 {
            break;
        }

        let entry = source_path.unwrap();
        let file_name = entry.path().display().to_string();

        // if not a valid file extension
        if !matcher.is_match(&file_name) {
            continue;
        }

        let file_content: String = fs::read_to_string(file_name).expect("Unable to find file");

        // if it's a match find specific dep and remove it from the master list
        for dep in check_list.clone() {
            if file_content.contains(&*dep) {
                found_deps.insert(dep.to_owned());
                check_list.remove(&*dep);

                // if type exists add also
                let type_variant = format!("@types/{}", dep);
                if check_list.contains(&*type_variant) {
                    found_deps.insert(type_variant.to_owned());
                    check_list.remove(&*type_variant);
                }
            }
        }
    }

    return found_deps;
}
