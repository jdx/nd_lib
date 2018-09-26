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
use std::iter::Iterator;
use std::option::Option;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Deserialize)]
#[derive(Debug, Clone)]
struct RegistryPackage {
    name: String,
    versions: HashMap<String, RegistryPackageVersion>,
}

#[derive(Deserialize)]
#[derive(Debug, Clone)]
struct RegistryPackageVersion {
    name: String,
    version: String,
    dist: RegistryPackageVersionDist,
    dependencies: Option<HashMap<String, String>>,
}

#[derive(Deserialize)]
#[derive(Debug, Clone)]
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
        let range = VersionReq::parse(semver_range).unwrap();
        let versions: Vec<&String> = metadata.versions.keys().collect();
        let version = get_max_compatible_version(&range, &versions);
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

fn get_max_compatible_version(range: &VersionReq, versions: &Vec<&String>) -> String {
    let mut versions: Vec<Version> = versions.iter()
        .map(|v| Version::parse(v).unwrap())
        .filter(|v| range.matches(&v))
        .collect();
    versions.sort_unstable();

    versions.last().unwrap().to_string()
}

fn fetch_all_metadata(name: &str, r: &VersionReq) -> Rc<RefCell<HashMap<String, RegistryPackage>>> {
    fn fetch(name: &str, r: &VersionReq, map: Rc<RefCell<HashMap<String, RegistryPackage>>>) {
        let m = get_metadata(name);
        map.borrow_mut().insert(name.to_owned(), m.clone());
        // let m = map.get(name).unwrap().clone();
        let versions: Vec<&String> = m.versions.keys().collect();
        let latest_version = get_max_compatible_version(&r, &versions);
        let versions = m.versions.get(&latest_version).unwrap();
        let empty = HashMap::new();
        for (n, v) in versions.clone().dependencies.unwrap_or(empty) {
            fetch(&n, &VersionReq::parse(&v).unwrap(), map.clone());
        }
    }
    let map = Rc::new(RefCell::new(HashMap::new()));
    fetch(name, r, map.clone());

    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
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
    #[test]
    fn test_fetch_all_metadata() {
        let m = fetch_all_metadata("edon-test-a", &VersionReq::parse("^0.0.1").unwrap());
        for (_, i) in m.borrow().iter() {
            println!("{:?}", i.name);
        }
        assert_eq!(m.borrow().get("edon-test-a").unwrap().versions.get("0.0.0").unwrap().version, "0.0.0");
        assert_eq!(m.borrow().get("edon-test-b").unwrap().versions.get("0.0.0").unwrap().version, "0.0.0");
    }
}
