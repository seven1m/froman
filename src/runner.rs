use workers::*;
use colors::*;
use config::*;
use std::thread::sleep;
use std::time::Duration;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::process::{Command, Stdio, Child, ChildStdout, ChildStderr};
use std::io::Read;
use std::path::Path;
use std::thread;
use std::process;
use std::io::prelude::*;
use std::io;
use redis;
use chrono;
use chrono::prelude::*;
use nix::sys::signal::{kill, Signal};

pub fn run(mut workers: &mut Vec<Box<Worker>>, config: &Config) {
    let interval = Duration::from_secs(2);
    let redis = redis::Client::open(config.redis_url.as_str()).unwrap();
    let redis_conn = redis.get_connection().expect("Redis connection failed. Is Redis running?");
    let label_size = get_label_size(&workers);
    loop {
        for (worker_index, worker) in workers.iter_mut().enumerate() {
            let key = worker.key();
            let color = COLORS[worker_index % COLORS.len()];
            if worker.work_to_do(&redis_conn) || worker.work_being_done(&redis_conn) {
                if worker.process().is_some() {
                    worker.set_terminate_at(None);
                } else {
                    log(worker.app(), label_size, color, "STARTING\n");
                    let mut process = spawn(worker, &config.command_template, &config.dir);
                    pipe_output(process.stdout.take().unwrap(), worker.app(), label_size, color);
                    pipe_output(process.stderr.take().unwrap(), worker.app(), label_size, color);
                    worker.set_process(Some(process));
                }
            } else {
                if worker.process().is_some() {
                    let now = Local::now();
                    if worker.terminate_at().is_some() {
                        if worker.terminate_at().unwrap() <= now {
                            kill(worker.process_id() as i32, Signal::SIGTERM).unwrap();
                            worker.set_process(None)
                        }
                    } else {
                        let terminate_at = now + chrono::Duration::seconds(30);
                        worker.set_terminate_at(Some(terminate_at));
                    }
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

fn pipe_output<T: 'static + Read + Send>(mut out: T, label: &str, label_size: usize, color: &str) {
    let label = label.to_owned();
    let color = color.to_owned();
    thread::spawn(move || {
        loop {
            let mut buf = [0; 10000];
            match out.read(&mut buf) {
                Ok(count) => {
                    if count > 0 {
                        log(&label, label_size, &color, &String::from_utf8_lossy(&buf));
                    } else {
                        break;
                    }
                }
                Err(_) => break
            }
        }
    });
}

fn log(label: &str, label_size: usize, color: &str, message: &str) {
    print!("{}: ", colorize(&left_pad(&label, label_size), color));
    print!("{}", message);
    io::stdout().flush().ok().expect("Could not flush stdout");
}

fn left_pad(str: &str, length: usize) -> String {
    " ".repeat(length - str.len()) + str
}
