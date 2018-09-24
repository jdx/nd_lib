extern crate env_logger;
extern crate serde_json;

use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::AsRef;
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Package {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub dependencies: Option<HashMap<String, String>>,

    #[serde(skip_serializing)]
    pub root: Option<PathBuf>,
}

impl Package {
    pub fn load<P: AsRef<Path>>(path: P) -> Package {
        let path = path.as_ref();
        debug!("loading {:?}", path);
        let file = File::open(path.join("package.json")).unwrap();
        let mut package: Package = serde_json::from_reader(file).unwrap();
        package.root = Some(path.to_owned());

        package
    }

    pub fn validate(&self) -> Vec<Issue> {
        fn validate_package(package: &Package, node: &Rc<PackageTree>) -> Vec<Issue> {
            let mut issues = vec![];
            let empty = HashMap::new();
            let deps = package.dependencies.as_ref().unwrap_or(&empty);
            for name in deps.keys() {
                let mut node_issues: Vec<Issue> = match node.get(&name) {
                    Some(dep_node) => validate_package(&node.package, &dep_node.clone()),
                    None => vec![Issue::PackageNotInstalled {
                        package: name.clone(),
                    }],
                };
                issues.append(&mut node_issues);
            }

            issues
        }

        fn validate_package_lock(node: &Rc<PackageTree>, lock: &PackageLock) -> Vec<Issue> {
            let mut issues = vec![];
            // let deps = package.dependencies.unwrap_or(HashMap::new());
            let empty = HashMap::new();
            let empty2 = HashMap::new();
            let deps = node.package.dependencies.as_ref().unwrap_or(&empty);
            let lock_deps = lock.dependencies.as_ref().unwrap_or(&empty2);
            for name in deps.keys() {
                issues.append(&mut match lock_deps.get(name) {
                    Some(_) => vec![],
                    None => vec![Issue::MissingPackageFromLock {
                        package: name.clone(),
                    }],
                });
            }
            for (_name, child) in node.children.borrow().iter() {
                issues.append(&mut validate_package_lock(&child.clone(), lock));
            }

            issues
        }

        let root = package_file_tree(self.root.as_ref().unwrap());
        let lock = PackageLock::load(self.root.as_ref().unwrap());

        let mut issues = validate_package(self, &root);
        issues.append(&mut validate_package_lock(&root, &lock));

        issues
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
    fn load<P: AsRef<Path>>(path: P) -> PackageLock {
        let path = path.as_ref().join("package-lock.json");
        let file = File::open(path).unwrap();
        let package: PackageLock = serde_json::from_reader(file).unwrap();

        package
    }
}

#[derive(Debug)]
pub enum Issue {
    MissingPackageFromLock { package: String },
    PackageNotInstalled { package: String },
}

struct PackageTree {
    package: Package,
    children: RefCell<HashMap<String, Rc<PackageTree>>>,
    parent: Option<Weak<PackageTree>>,
}

impl PackageTree {
    fn get(&self, name: &str) -> Option<Rc<PackageTree>> {
        debug!("{:?}", name);
        let children = self.children.borrow();
        for (child_name, node) in children.iter() {
            if child_name == name {
                return Some(node.clone());
            }
        }

        match self.parent {
            Some(ref parent) => parent.upgrade().unwrap().get(&name),
            None => None,
        }
    }
}

fn package_file_tree<P: AsRef<Path>>(root: P) -> Rc<PackageTree> {
    fn package_file_tree<P: AsRef<Path>>(
        root: P,
        parent: Option<Weak<PackageTree>>,
    ) -> Rc<PackageTree> {
        let root = root.as_ref();
        let node = Rc::new(PackageTree {
            package: Package::load(root),
            children: RefCell::new(HashMap::new()),
            parent,
        });
        let files = match fs::read_dir(root.join("node_modules")) {
            Ok(files) => files.collect(),
            Err(_) => vec![],
        };
        let packages = files
            .into_iter()
            .map(|f| f.unwrap().path())
            .filter(|f| f.is_dir())
            .map(|d| {
                if d.file_name().unwrap().to_str().unwrap().starts_with('@') {
                    fs::read_dir(d)
                        .unwrap()
                        .into_iter()
                        .map(|f| f.unwrap().path())
                        .filter(|f| f.is_dir())
                        .collect()
                } else {
                    vec![d]
                }
            }).flatten();
        for pkg in packages {
            let child = package_file_tree(pkg, Some(Rc::downgrade(&node)));
            node.children
                .borrow_mut()
                .insert(child.package.name.clone(), child);
        }

        node
    }

    package_file_tree(root, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_package() {
        let p = Package::load("fixtures/example");
        assert_eq!(p.name, "example");
        assert_eq!(p.version, "0.0.0");
        assert_eq!(p.dependencies.unwrap().get("edon-test-a").unwrap(), "0.0.1");
    }
    #[test]
    fn loads_package_lock() {
        let p = PackageLock::load("fixtures/example");
        assert_eq!(p.name, "example");
        assert_eq!(p.version, "0.0.0");
        assert_eq!(
            p.dependencies.unwrap().get("edon-test-a").unwrap().version,
            "0.0.1"
        );
    }
    #[test]
    fn finds_missing_deps_from_lock() {
        let p = Package::load("fixtures/missing-dep-from-lock");
        let issues = p.validate();
        match &issues[0] {
            Issue::MissingPackageFromLock { ref package } => assert_eq!(package, "edon-test-c"),
            _ => panic!("invalid issue {:?}", &issues[0]),
        }
        assert_eq!(issues.len(), 1);
    }
    #[test]
    fn finds_missing_subdeps_from_lock() {
        let p = Package::load("fixtures/missing-subdep-from-lock");
        let issues = p.validate();
        match &issues[0] {
            Issue::MissingPackageFromLock { ref package } => assert_eq!(package, "edon-test-c"),
            _ => panic!("invalid issue {:?}", &issues[0]),
        }
        assert_eq!(issues.len(), 1);
    }
    #[test]
    fn does_not_error_if_no_deps() {
        let p = Package::load("fixtures/no_deps");
        let issues = p.validate();
        assert_eq!(issues.len(), 0);
    }
    #[test]
    fn test_package_file_tree() {
        let tree = package_file_tree("fixtures/example");
        assert_eq!(tree.package.name, "example");
        assert_eq!(
            tree.children
                .borrow()
                .get("edon-test-a")
                .unwrap()
                .package
                .name,
            "edon-test-a"
        );
        assert_eq!(
            tree.children
                .borrow()
                .get("edon-test-a")
                .unwrap()
                .package
                .version,
            "0.0.1"
        );
    }
    #[test]
    fn dep_not_installed() {
        let p = Package::load("fixtures/dep-not-installed");
        let issues = p.validate();
        match &issues[0] {
            Issue::PackageNotInstalled { ref package } => assert_eq!(package, "edon-test-c"),
            _ => panic!("invalid issue"),
        }
        assert_eq!(issues.len(), 1);
    }
}
