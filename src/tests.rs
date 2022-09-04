#[cfg(test)]
mod tests {
    use std::fs;
    use std::collections::HashSet;
    use crate::scan_files;
    use crate::Package;

    #[test]
    fn scan_1() {
        let extensions = HashSet::from(["tsx", "ts", "jsx", "js", "mjs", "cjs"]);
        let package: String =
            fs::read_to_string("./data/package.json").expect("Unable to find file");
        let package: Package =
            serde_json::from_str(&package).expect("Unable to read file. Is it json?");
        let deps: HashSet<String> = package.dependencies.keys().cloned().collect();

        let result = scan_files("./data", &extensions, &deps);
        let expected_deps = HashSet::from([
            String::from("@dep/some"),
            String::from("dependency"),
            String::from("nested"),
        ]);

        assert_eq!(
            expected_deps,
            result.iter().map(|s| s.to_string()).collect()
        );
    }
}
