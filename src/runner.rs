extern crate redis;
extern crate cmdline_words_parser;

use workers::*;

use std::thread::sleep;
use std::time::Duration;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::process::{Command, Stdio, Child};
use runner::redis::Commands;
use self::cmdline_words_parser::StrExt;

pub fn run(workers: &Vec<Worker>, command_template: &str, redis_url: &str) {
    let interval = Duration::from_secs(2);
    println!("here");
    let redis = redis::Client::open(redis_url).unwrap();
    let redis_conn = redis.get_connection().unwrap();
    let mut processes: HashMap<String, Child> = HashMap::new();
    loop {
        for worker in workers {
            if work_to_do(worker, &redis_conn) || work_being_done(worker, &redis_conn) {
                println!("doing work for {}", worker.app());
                let key = format!("{}: {}", worker.app(), worker.kind());
                match processes.entry(key.to_string()) {
                    Entry::Occupied(mut entry) => {
                        println!("process already spawned {}", worker.command());
                    },
                    Entry::Vacant(entry) => {
                        let mut command = command_template.replace("%s", &worker.command());
                        let mut command_to_parse = command.clone();
                        let mut args: Vec<&str> = command_to_parse.parse_cmdline_words().collect();
                        let program = args.remove(0);
                        println!("spawn program {} with args {:?}", &program, &args);
                        let child = Command::new(&program)
                            .args(args)
                            //.stdin(Stdio::piped())
                            //.stdout(Stdio::piped())
                            .spawn()
                            .expect(&format!("Failed to execute command {}", &command));
                        entry.insert(child);
                    }
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
