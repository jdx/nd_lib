extern crate rayon;
extern crate semver;

use self::rayon::prelude::*;
use self::semver::{Version, VersionReq};
use cache::extract_tarball;
use reqwest;
use serde_json;
use std::collections::HashMap;
use Package;
use std::path::PathBuf;

#[derive(Deserialize)]
struct RegistryPackage {
    // name: String,
    versions: HashMap<String, RegistryPackageVersion>,
}

#[derive(Deserialize)]
struct RegistryPackageVersion {
    // name: String,
    // version: String,
    dist: RegistryPackageVersionDist,
}

#[derive(Deserialize)]
struct RegistryPackageVersionDist {
    // integrity: String,
    tarball: String,
}

pub fn refresh(pkg: &Package) {
    let root = pkg.root.as_ref().unwrap().clone();
    let deps = HashMap::new();
    let deps = pkg.dependencies.as_ref().unwrap_or(&deps);
    deps.into_par_iter().for_each(|(name, semver_range)| {
        let root = root.join("node_modules").join(name);
        let metadata = get_metadata(name);
        let version = get_actual_version(&metadata, semver_range);
        let m_version = &metadata.versions[&version];
        extract_tarball(&m_version.dist.tarball, root, PathBuf::from("tmp").join(name).join(version))
    });
}

fn get_metadata(name: &str) -> RegistryPackage {
    let url = format!("https://registry.npmjs.org/{}", name);
    println!("{:?}", url);
    let response = reqwest::get(&url).unwrap();

    serde_json::from_reader(response).unwrap()
}

fn get_actual_version(metadata: &RegistryPackage, semver_range: &str) -> String {
    let semver_range = VersionReq::parse(semver_range).unwrap();
    let mut versions: Vec<Version> = metadata
        .versions
        .keys()
        .map(|v| Version::parse(v).unwrap())
        .filter(|v| semver_range.matches(&v))
        .collect();
    versions.sort_unstable();
    println!("done");

    versions.last().unwrap().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_get_actual_version() {
        let metadata = get_metadata("edon-test-c");
        let version = get_actual_version(&metadata, "^1.0.0");
        assert_eq!(version, "1.0.4");
    }
    #[test]
    fn test_installs_subdeps_4() {
        let t = PathBuf::from("fixtures/4-installs-subdeps");
        fs::remove_dir_all(t.join("node_modules")).unwrap_or(());
        let p = Package::load(&t);
        p.refresh();
        assert_eq!(Package::load(&t.join("node_modules/edon-test-c")).version, "1.0.4");
        assert_eq!(Package::load(&t.join("node_modules/edon-test-a")).version, "0.0.1");
        // assert_eq!(Package::load(&t.join("node_modules/edon-test-a/node_modules/edon-test-c")).version, "0.0.0");
    }
}
