extern crate redis;
extern crate chrono;
extern crate nix;

use workers::*;

use std::thread::sleep;
use std::time::Duration;
use std::collections::HashMap;
use std::process::{Command, Stdio, Child};
use std::io::Write;
use std::path::Path;
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
                    running_process.terminate_at = None
                } else {
                    let process = spawn(worker, command_template, config_dir);
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
                                kill(running_process.process.id() as i32, Signal::SIGTERM).unwrap();
                                remove = true;
                            }
                        }
                        None => {
                            let terminate_at = now + chrono::Duration::seconds(30);
                            running_process.terminate_at = Some(terminate_at);
                        }
                    }
                }
                if remove {
                    println!("stop process for {}", worker.app());
                    processes.remove(&key);
                }
            }
        }
        sleep(interval);
    }
}

fn spawn(worker: &Box<Worker>, command_template: &str, config_dir: &str) -> Child {
    let (program, args) = worker.command_binary_and_args(command_template);
    let path = Path::new(&worker.absolute_path(config_dir)).canonicalize().unwrap();
    let path_str = path.to_str().unwrap();
    println!("spawn program {} with args {:?} at path {}", &program, &args, path_str);
    Command::new(&program)
        .args(&args)
        .stdin(Stdio::piped())
        //.stdout(Stdio::piped())
        .current_dir(path_str)
        .env_remove("PATH") // rvm won't update the current ruby version if a ruby version is already present in the PATH
        .env_remove("RUBY_VERSION")
        .env_remove("RBENV_VERSION")
        .env_remove("RBENV_GEMSET_ALREADY")
        .env_remove("RBENV_DIR")
        .spawn()
        .expect("failure")
}
