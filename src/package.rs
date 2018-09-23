extern crate serde_json;

use std::convert::AsRef;
use std::fs::File;
use std::path::Path;

#[derive(Serialize, Deserialize)]
pub struct Package {
    name: String,
    version: String,
    description: Option<String>,
}

impl Package {
    pub fn load<P: AsRef<Path>>(path: P) -> Package {
        let path = path.as_ref().join("package.json");
        let file = File::open(path).unwrap();
        let package: Package = serde_json::from_reader(file).unwrap();

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
    }
}
