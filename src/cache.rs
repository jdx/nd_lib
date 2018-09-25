extern crate clonedir_lib;

use self::clonedir_lib::clonedir;
use flate2::read::GzDecoder;
use reqwest;
use serde_json;
use std::fs;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use tar::Archive;

pub fn extract<P: AsRef<Path>>(p: P, name: &str, version: &str) {
    let c = cache(name, version);
    clonedir(c, p);
}

fn cache(name: &str, version: &str) -> PathBuf {
    fn get_real_path(parent: &Path, child: &Path) -> PathBuf {
        let child = match child.starts_with("package") {
            true => child.strip_prefix("package").unwrap(),
            false => child,
        };
        let path = parent.join(child);
        if !path.starts_with(parent) {
            panic!("invalid tarball");
        }

        path
    }

    let url = format!(
        "https://registry.npmjs.org/{name}/-/{name}-{version}.tgz",
        name = name,
        version = version
    );
    let to = PathBuf::from("tmp/refresh/1").join(name).join(version);

    let response = reqwest::get(&url).unwrap();
    let ungzip = GzDecoder::new(response);
    let mut archive = Archive::new(ungzip);
    for file in archive.entries().unwrap() {
        let mut file = file.unwrap();
        let kind = file.header().entry_type();
        let path = file.path().unwrap().into_owned();
        if kind.is_pax_global_extensions() {
            break;
        }
        let path = get_real_path(&to, &path);
        debug!("{:?} {:?}", kind, path);
        if kind.is_dir() {
            fs::create_dir_all(path).unwrap();
        } else if kind.is_file() {
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            let mut output = File::create(&path).unwrap();
            io::copy(&mut file, &mut output).unwrap();
        }
    }
    create_integrity(&to);

    to
}

fn create_integrity<P: AsRef<Path>>(path: P) {
    let integrity = Integrity {
        method: String::from("sha256"),
        hash: String::from("foo"),
    };
    let f = File::create(path.as_ref().join(".nd-integrity")).unwrap();
    serde_json::to_writer(f, &integrity).unwrap();
}

#[derive(Serialize, Deserialize)]
struct Integrity {
    method: String,
    hash: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup<P: AsRef<Path>>(p: P) {
        fs::remove_dir_all(p).unwrap_or(());
    }

    fn teardown<P: AsRef<Path>>(p: P) {
        fs::remove_dir_all(p).unwrap_or(());
    }

    #[test]
    fn extracts_package() {
        let p = PathBuf::from("tmp/refresh/1");
        setup(&p);
        extract(&p, "edon-test-a", "0.0.0");
        println!("{:?}", p.join("package.json"));
        fs::read_to_string(p.join("package.json")).unwrap();
        teardown(&p);
    }
}
