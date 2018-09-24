extern crate serde_json;

use std::collections::HashMap;
use std::convert::AsRef;
use std::fs::File;
use std::path::Path;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Package {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub dependencies: Option<HashMap<String, String>>,
}

impl Package {
    pub fn load<P: AsRef<Path>>(path: P) -> Package {
        let path = path.as_ref();
        let file = File::open(path.join("package.json")).unwrap();
        let package: Package = serde_json::from_reader(file).unwrap();

        package
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageLock {
    pub name: String,
    pub version: String,
    pub lockfile_version: u8,
    pub description: Option<String>,
    pub dependencies: Option<HashMap<String, PackageLockDependency>>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageLockDependency {
    #[serde(skip_serializing)]
    pub name: Option<String>,

    pub version: String,
    pub resolved: String,
    pub integrity: String,
    pub requires: Option<HashMap<String, String>>,
    pub dependencies: Option<HashMap<String, PackageLockDependency>>,
}

impl PackageLock {
    pub fn load<P: AsRef<Path>>(path: P) -> PackageLock {
        let path = path.as_ref().join("package-lock.json");
        let file = File::open(path).unwrap();
        let package: PackageLock = serde_json::from_reader(file).unwrap();

        package
    }
}

#[derive(Debug)]
pub enum Issue {
    MissingPackageFromLock {package: String},
}

pub fn validate_package_lock(package: Package, lock: PackageLock) -> Vec<Issue> {
    let mut issues = vec![];
    let deps = package.dependencies.unwrap_or(HashMap::new());
    let lock_deps = lock.dependencies.unwrap_or(HashMap::new());
    for (name, _version) in deps {
        if lock_deps.get(&name).is_none() {
            issues.push(Issue::MissingPackageFromLock{package: name.clone()});
        }
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_package() {
        let p = Package::load("fixtures/example");
        assert_eq!(p.name, "example");
        assert_eq!(p.version, "0.0.0");
        assert_eq!(
            p.dependencies.unwrap().get("@oclif/errors").unwrap(),
            "^1.2.1"
        );
    }
    #[test]
    fn loads_package_lock() {
        let p = PackageLock::load("fixtures/example");
        assert_eq!(p.name, "example");
        assert_eq!(p.version, "0.0.0");
        assert_eq!(
            p.dependencies.unwrap().get("ansi-regex").unwrap().version,
            "3.0.0"
        );
    }
    #[test]
    fn finds_missing_deps_from_lock() {
        let p = Package::load("fixtures/missing-dep-from-lock");
        let l = PackageLock::load("fixtures/missing-dep-from-lock");
        let issues = validate_package_lock(p, l);
        assert_matches!(issues[0], Issue::MissingPackageFromLock{..});
        assert_eq!(issues.len(), 1);
    }
    #[test]
    fn does_not_error_if_no_deps() {
        let p = Package::load("fixtures/no_deps");
        let l = PackageLock::load("fixtures/no_deps");
        let issues = validate_package_lock(p, l);
        assert_eq!(issues.len(), 0);
    }
}
