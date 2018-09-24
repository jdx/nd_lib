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
        let path = path.as_ref().join("package.json");
        let file = File::open(path).unwrap();
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
}
