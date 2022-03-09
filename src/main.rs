extern crate chrono;
extern crate clap;
extern crate cmdline_words_parser;
extern crate nix;
extern crate redis;
extern crate yaml_rust;

mod colors;
mod config;
mod errors;
mod runner;
mod workers;
use config::*;
use errors::*;
use runner::*;
use workers::*;

use clap::App;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::process::exit;
use std::thread;
use std::time;
use yaml_rust::{Yaml, YamlLoader};

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const DEFAULT_CONFIG: &'static str = "froman.yml";
const DEFAULT_REDIS_URL: &'static str = "redis://127.0.0.1/";
const DEFAULT_TIMEOUT: u16 = 30;

fn main() {
    let matches = App::new("froman")
    .version(VERSION)
    .about("process manager for your dev environment")
    .args_from_usage("-c, --config=[FILE] 'Specifies a custom config file (default: ./froman.yml)'")
    .args_from_usage("-r, --redis=[URL] 'Specifies the Redis URL (default: redis://127.0.0.1/)'")
    .args_from_usage("-d, --debug 'Enables debugging output'")
    .args_from_usage(
      "-t, --timeout=[SECONDS] 'Specifies the number of seconds to wait before stopping a worker (default: 30)'",
      )
    .get_matches();

    let config_path = matches.value_of("config").unwrap_or(DEFAULT_CONFIG);
    let redis_url = matches.value_of("redis").unwrap_or(DEFAULT_REDIS_URL);
    let debug_mode = matches.is_present("debug");
    let timeout = matches
        .value_of("timeout")
        .map(|t| {
            t.parse::<u16>()
                .expect("expected a positive integer for timeout")
        })
        .unwrap_or(DEFAULT_TIMEOUT);
    let yaml_config = read_config(&config_path);
    let command_template = yaml_config["command_template"]
        .as_str()
        .expect("config 'command_template' key not found!");
    let mut config_dir = Path::new(&config_path)
        .parent()
        .expect("could not get parent directory of config path")
        .to_str()
        .expect("could not get parent directory of config path as string");
    if config_dir.is_empty() {
        config_dir = "."
    }

    let config = Config {
        dir: config_dir.to_string(),
        command_template: command_template.to_string(),
        redis_url: redis_url.to_string(),
        timeout: timeout,
    };

    let mut workers = build_workers(&yaml_config, debug_mode);
    let mut runner = Runner::new(&config);
    loop {
        match runner.run(&mut workers) {
            Ok(()) => break,
            Err(FromanError::RedisError(e)) => {
                println!(
                    "Error connecting to Redis: {}; Will retry in 10 seconds...",
                    e
                );
            }
        }
        thread::sleep(time::Duration::from_secs(10));
    }
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

fn build_workers(config: &Yaml, debug: bool) -> Vec<Box<dyn Worker>> {
    let apps = config["apps"]
        .as_hash()
        .expect("config 'apps' key not found!");
    let mut path = "";
    apps.iter()
        .flat_map(|(app, app_config)| -> Vec<Box<dyn Worker>> {
            app_config
                .as_hash()
                .expect("config is not a hash!")
                .iter()
                .filter_map(|(worker_type, worker_config)| -> Option<Box<dyn Worker>> {
                    if debug {
                        println!("{:?}: {:?}", worker_type, worker_config);
                    }
                    let worker_type = worker_type
                        .as_str()
                        .expect("could not get worker type as string");
                    match worker_type {
                        "path" => {
                            // special key that points to the app path
                            path = worker_config
                                .as_str()
                                .expect("could not get app path as string");
                            None
                        }
                        "resque" => Some(Box::new(Resque {
                            app: app
                                .as_str()
                                .expect("could not get app name as string")
                                .to_string(),
                            path: path.to_string(),
                            namespace: worker_config["namespace"]
                                .as_str()
                                .unwrap_or("")
                                .to_string(),
                            command: worker_config["command"]
                                .as_str()
                                .expect("could not get start command as string")
                                .to_string(),
                            process: None,
                            terminate_at: None,
                        })),
                        "sidekiq" => Some(Box::new(Sidekiq {
                            app: app
                                .as_str()
                                .expect("could not get app name as string")
                                .to_string(),
                            path: path.to_string(),
                            namespace: worker_config["namespace"]
                                .as_str()
                                .unwrap_or("")
                                .to_string(),
                            command: worker_config["command"]
                                .as_str()
                                .expect("could not get start command as string")
                                .to_string(),
                            process: None,
                            terminate_at: None,
                        })),
                        _ => None,
                    }
                })
                .collect()
        })
        .collect()
}
