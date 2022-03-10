use std::path::{Path, PathBuf};

pub struct Config {
    pub dir: String,
    pub command_template: String,
    pub redis_url: String,
    pub timeout: u16,
}

impl Config {
    pub fn path_relative_to_config_dir(&self, path: &str) -> PathBuf {
        let path_obj = Path::new(path);
        if path_obj.is_absolute() {
            return path_obj.to_owned();
        }
        let dir = Path::new(&self.dir);
        let full_path = dir.join(path_obj);
        match full_path.canonicalize() {
            Ok(p) => p.to_owned(),
            Err(_) => {
                println!("Path `{:?}` could not be found!", full_path);
                panic!();
            }
        }
    }
}
