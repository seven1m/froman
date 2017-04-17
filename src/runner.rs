extern crate redis;
extern crate cmdline_words_parser;
extern crate nix;

use workers::*;

use std::thread::sleep;
use std::time::Duration;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::process::{Command, Stdio, Child};
use runner::redis::Commands;
use self::cmdline_words_parser::StrExt;
use self::nix::sys::signal::kill;
use self::nix::sys::signal::Signal;

pub fn run(workers: &Vec<Worker>, command_template: &str, redis_url: &str) {
    let interval = Duration::from_secs(2);
    println!("here");
    let redis = redis::Client::open(redis_url).unwrap();
    let redis_conn = redis.get_connection().unwrap();
    let mut processes: HashMap<String, Child> = HashMap::new();
    loop {
        for worker in workers {
            let key = format!("{}: {}", worker.app(), worker.kind()).to_string();
            if work_to_do(worker, &redis_conn) || work_being_done(worker, &redis_conn) {
                if !processes.contains_key(&key) {
                    let mut command = command_template.replace("%s", &worker.command());
                    let mut command_to_parse = command.clone();
                    let mut args: Vec<&str> = command_to_parse.parse_cmdline_words().collect();
                    let program = args.remove(0);
                    println!("spawn program {} with args {:?} at path {}", &program, &args, worker.path());
                    let child = Command::new(&program)
                        .args(args)
                        //.stdin(Stdio::piped())
                        //.stdout(Stdio::piped())
                        .current_dir(worker.path())
                        .spawn()
                        .expect(&format!("Failed to execute command {}", &command));
                    processes.insert(key, child);
                }
            } else {
                if processes.contains_key(&key) {
                    println!("removing process {}", worker.command());
                    let process = processes.remove(&key).unwrap();
                    kill(process.id() as i32, Signal::SIGTERM);
                }
            }
        }
        sleep(interval);
    }
}

fn work_to_do(worker: &Worker, redis_conn: &redis::Connection) -> bool {
    match *worker {
        Worker::Resque { ref namespace, .. } => {
            let queues: Vec<String> = redis_conn.smembers(format!("{}:queues", namespace)).unwrap();
            let counts: Vec<i32> = queues.iter().map(|q| {
                redis_conn.llen(format!("{}:queue:{}", namespace, q)).unwrap()
            }).collect();
            counts.iter().fold(0i32, |a, &b| a + b) > 0
        },
        Worker::Sidekiq { ref namespace, .. } => {
            let queues: Vec<String> = redis_conn.keys(format!("{}:queue:*", namespace)).unwrap();
            let counts: Vec<i32> = queues.iter().map(|q| {
                redis_conn.llen(q).unwrap()
            }).collect();
            counts.iter().fold(0i32, |a, &b| a + b) > 0
        }
    }
}

fn work_being_done(worker: &Worker, redis_conn: &redis::Connection) -> bool {
    match *worker {
        Worker::Resque { .. } => {
            false // no way to know if work is being done in Resque
        },
        Worker::Sidekiq { ref namespace, .. } => {
            let processes: Vec<String> = redis_conn.smembers(format!("{}:processes", namespace)).unwrap();
            let counts: Vec<i32> = processes.iter().map(|p| {
                redis_conn.hget(format!("{}:{}", namespace, p), "busy").unwrap()
            }).collect();
            counts.iter().fold(0i32, |a, &b| a + b) > 0
        }
    }
}
