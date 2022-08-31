use crate::scan_files;
use crate::scan_files_new;

use regex::Regex;
use std::collections::HashSet;
use std::time::Instant;

#[derive(Debug)]
struct Stats {
    mean: u128,
    max: u128,
    min: u128,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_deps() -> HashSet<String> {
        let mut deps: HashSet<String> = HashSet::new();
        for dep in vec!["react", "react-dom", "lodash.clone", "@types/react", "@testing-library/jest-dom", "@apollo/client"] {
            deps.insert(dep.parse().unwrap());
        }
        return deps;
    }

    fn run_bench(runs: u128, f: fn()) -> Stats {
        let mut min: u128 = std::u128::MAX;
        let mut max: u128 = 0;
        let mut total_time: u128 = 0;

        for _i in 0..runs {
            let now = Instant::now();
            f();


            let elapsed = now.elapsed().as_millis();
            if elapsed > max {
                max = elapsed
            }
            if elapsed < min {
                min = elapsed
            }
            total_time += elapsed;
        }

        return Stats {
            mean: total_time / runs,
            max,
            min,
        }
    }

    #[test]
    fn scan_1() {
        let runs = 10;
        let closure = || {
            let extensions = HashSet::from(["tsx", "ts", "jsx", "js", "mjs", "cjs"]);
            scan_files("/Users/alexspence/git/react/packages", &extensions, &get_test_deps());
        };

        let stats = run_bench(runs, closure);
        println!("{}: {:?} (n={})", "scan 1", stats, runs);
    }

    #[test]
    fn scan_2() {
        let runs = 10;
        let closure = || {
            let extensions = HashSet::from(["tsx", "ts", "jsx", "js", "mjs", "cjs"]);
            scan_files_new("/Users/alexspence/git/react/packages", &extensions, &get_test_deps());
        };

        let stats = run_bench(runs, closure);
        println!("{}: {:?} (n={})", "scan 2", stats, runs);
    }
}
