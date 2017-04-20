extern crate redis;
extern crate chrono;
extern crate nix;

use workers::*;

use std::thread::sleep;
use std::time::Duration;
use std::collections::HashMap;
use std::process::{Command, Stdio, Child};
use std::io::Write;
use self::chrono::prelude::*;
use self::nix::sys::signal::{kill, Signal};

struct RunningProcess {
    process: Child,
    terminate_at: Option<DateTime<Local>>
}

pub fn run(workers: &Vec<Box<Worker>>, config_dir: &str, command_template: &str, redis_url: &str) {
    let interval = Duration::from_secs(2);
    let redis = redis::Client::open(redis_url).unwrap();
    let redis_conn = redis.get_connection().unwrap();
    let mut processes: HashMap<String, RunningProcess> = HashMap::new();
    loop {
        for worker in workers {
            let key = worker.key();
            if worker.work_to_do(&redis_conn) || worker.work_being_done(&redis_conn) {
                if processes.contains_key(&key) {
                    let mut running_process = processes.get_mut(&key).unwrap();
                    println!("more work found; setting terminate_at to None");
                    running_process.terminate_at = None
                } else {
                    let process = worker.spawn(command_template, config_dir);
                    let running_process = RunningProcess {
                        process: process,
                        terminate_at: None
                    };
                    processes.insert(key, running_process);
                }
            } else if processes.contains_key(&key) {
                let now = Local::now();
                let mut remove = false;
                {
                    let mut running_process = processes.get_mut(&key).unwrap();
                    match running_process.terminate_at {
                        Some(terminate_at) => {
                            if terminate_at <= now {
                                println!("terminate_at is before {:?}", now);
                                kill(running_process.process.id() as i32, Signal::SIGTERM).unwrap();
                                remove = true;
                            } else {
                                println!("terminate_at is after {:?}", now);
                            }
                        }
                        None => {
                            let terminate_at = now + chrono::Duration::seconds(30);
                            println!("setting terminate_at to {:?}", terminate_at);
                            running_process.terminate_at = Some(terminate_at);
                        }
                    }
                }
                if remove {
                    processes.remove(&key);
                }
            }
        }
        sleep(interval);
    }
}
