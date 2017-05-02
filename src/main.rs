extern crate clap;
extern crate yaml_rust;
extern crate redis;
extern crate chrono;
extern crate nix;

mod runner;
mod workers;
mod colors;
mod config;
use workers::*;
use config::*;

use std::io::prelude::*;
use std::fs::File;
use std::path::Path;
use std::process::exit;
use clap::App;
use yaml_rust::{YamlLoader, Yaml};

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const DEFAULT_CONFIG: &'static str = "honcho.yml";
const DEFAULT_REDIS_URL: &'static str = "redis://127.0.0.1/";

fn main() {
    let matches = App::new("froman")
        .version(VERSION)
        .about("process manager for your dev environment")
        .args_from_usage("-c, --config=[FILE] 'Use a custom config file (default: ./honcho.yml)'")
        .args_from_usage("-r, --redis=[URL] 'Specify Redis URL (default: redis://127.0.0.1/)'")
        .get_matches();

    let dir = matches.value_of("config").unwrap_or(DEFAULT_CONFIG);
    let redis_url = matches.value_of("redis").unwrap_or(DEFAULT_REDIS_URL);
    let yaml_config = read_config(&dir);
    let command_template = yaml_config["command_template"].as_str().expect("config 'command_template' key not found!");

    let config = Config {
        dir: dir.to_string(),
        command_template: command_template.to_string(),
        redis_url: redis_url.to_string()
    };

    let workers = build_workers(&yaml_config);
    let mut config_dir = Path::new(&dir).parent().unwrap().to_str().unwrap();
    if config_dir.is_empty() { config_dir = "." }
    runner::run(&workers, &config);
}

fn read_config(path: &str) -> Yaml {
    let mut f = match File::open(&path) {
        Ok(f) => f,
        Err(e) => {
            println!("ERROR: could not open file {}: {}", path, e);
            exit(1);
        }
    };
    let mut s = String::new();
    f.read_to_string(&mut s).expect("error reading config file");
    match YamlLoader::load_from_str(&s) {
        Ok(docs) => docs[0].to_owned(),
        Err(e) => {
            println!("ERROR: could not parse config file {}: {}", path, e);
            exit(2);
        }
    }
}

fn build_workers(config: &Yaml) -> Vec<Box<Worker>> {
    let apps = config["apps"].as_hash().expect("config 'apps' key not found!");
    let mut path = "";
    apps.iter().flat_map(|(app, app_config)| -> Vec<Box<Worker>> {
        app_config.as_hash().unwrap().iter().filter_map(|(worker_type, worker_config)| -> Option<Box<Worker>> {
            let worker_type = worker_type.as_str().unwrap();
            match worker_type {
                "path" => {
                    // special key that points to the app path
                    path = worker_config.as_str().unwrap();
                    None
                },
                "resque" => {
                    Some(Box::new(Resque {
                        app: app.as_str().unwrap().to_string(),
                        path: path.to_string(),
                        namespace: worker_config["namespace"].as_str().unwrap().to_string(),
                        command: worker_config["command"].as_str().unwrap().to_string()
                    }))
                },
                "sidekiq" => {
                    Some(Box::new(Sidekiq {
                        app: app.as_str().unwrap().to_string(),
                        path: path.to_string(),
                        namespace: worker_config["namespace"].as_str().unwrap().to_string(),
                        command: worker_config["command"].as_str().unwrap().to_string()
                    }))
                },
                _ => None
            }
        }).collect()
    }).collect()
}
