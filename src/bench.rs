use crate::scan_files;

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

    fn get_matcher() -> Regex {
        return Regex::new(r"(\.tsx$|\.ts$|\.jsx$|\.js$|\.mjs$|\.cjs$)").expect("Invalid extension regex provided");
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
        let runs = 50;
        let closure = || {
            scan_files(".", &get_matcher(), &get_test_deps());
        };

        let stats = run_bench(runs, closure);
        println!("{}: {:?} (n={})", "scan 1", stats, runs);
    }
}
