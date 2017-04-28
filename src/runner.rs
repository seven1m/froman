use workers::*;
use std::thread::sleep;
use std::time::Duration;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::process::{Command, Stdio, Child, ChildStdout, ChildStderr};
use std::io::Read;
use std::path::Path;
use std::thread;
use std::process;
use redis;
use chrono;
use chrono::prelude::*;
use nix::sys::signal::{kill, Signal};

struct RunningProcess {
    process: Child,
    terminate_at: Option<DateTime<Local>>
}

pub fn run(workers: &Vec<Box<Worker>>, config_dir: &str, command_template: &str, redis_url: &str) {
    let interval = Duration::from_secs(2);
    let redis = redis::Client::open(redis_url).unwrap();
    let redis_conn = redis.get_connection().expect("Redis connection failed. Is Redis running?");
    let mut processes: HashMap<String, RunningProcess> = HashMap::new();
    let label_size = get_label_size(&workers);
    loop {
        for (worker_index, worker) in workers.iter().enumerate() {
            let key = worker.key();
            if worker.work_to_do(&redis_conn) || worker.work_being_done(&redis_conn) {
                if processes.contains_key(&key) {
                    let mut running_process = processes.get_mut(&key).unwrap();
                    running_process.terminate_at = None
                } else {
                    let mut process = spawn(worker, command_template, config_dir);
                    pipe_output(process.stdout.take().unwrap(), worker.app(), label_size);
                    pipe_output(process.stderr.take().unwrap(), worker.app(), label_size);
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

fn get_label_size(workers: &Vec<Box<Worker>>) -> usize {
    workers.iter().map(|w| w.app().len()).max().unwrap()
}

fn spawn(worker: &Box<Worker>, command_template: &str, config_dir: &str) -> Child {
    let (program, args) = worker.command_binary_and_args(command_template);
    let path = match Path::new(&worker.absolute_path(config_dir)).canonicalize() {
        Ok(p) => p,
        Err(_) => {
            println!("Path `{}` could not be found!", config_dir);
            process::exit(1);
        }
    };
    let path_str = path.to_str().unwrap();
    println!("spawn program {} with args {:?} at path {}", &program, &args, path_str);
    Command::new(&program)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .current_dir(path_str)
        .env_remove("PATH") // rvm won't update the current ruby version if a ruby version is already present in the PATH
        .env_remove("RUBY_VERSION")
        .env_remove("RBENV_VERSION")
        .env_remove("RBENV_GEMSET_ALREADY")
        .env_remove("RBENV_DIR")
        .spawn()
        .expect("failure")
}

fn pipe_output<T: 'static + Read + Send>(mut out: T, label: &str, label_size: usize) {
    let label = label.to_owned();
    thread::spawn(move || {
        loop {
            let mut buf = [0; 1000];
            match out.read(&mut buf) {
                Ok(count) => {
                    if count > 0 {
                        print!("{}: ", left_pad(&label, label_size));
                        print!("{}", String::from_utf8_lossy(&buf));
                    } else {
                        break;
                    }
                }
                Err(_) => break
            }
        }
    });
}

fn left_pad(str: &str, length: usize) -> String {
    " ".repeat(length - str.len()) + str
}
