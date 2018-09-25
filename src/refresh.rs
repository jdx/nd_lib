use Package;

use flate2::read::GzDecoder;
use reqwest;
use std::fs;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use tar::Archive;

pub fn refresh(pkg: &Package) {
    println!("refreshing");
}

fn cache(name: &str, version: &str) {
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

    let url = "https://registry.npmjs.org/tslib/-/tslib-1.9.3.tgz";
    let to = PathBuf::from("tmp/refresh/1");

    let response = reqwest::get(url).unwrap();
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
    fn caches_package() {
        let p = PathBuf::from("tmp/refresh/1");
        setup(&p);
        cache("edon-test-a", "0.0.0");
        // teardown(&p);
    }
}
