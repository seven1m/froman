extern crate redis;
extern crate cmdline_words_parser;
extern crate nix;

use workers::*;

use std::thread::sleep;
use std::time::Duration;
use std::collections::HashMap;
use std::process::{Command, Stdio, Child};
use self::cmdline_words_parser::StrExt;
use self::nix::sys::signal::kill;
use self::nix::sys::signal::Signal;

pub fn run(workers: &Vec<Box<Worker>>, config_dir: &str, command_template: &str, redis_url: &str) {
    let interval = Duration::from_secs(2);
    let redis = redis::Client::open(redis_url).unwrap();
    let redis_conn = redis.get_connection().unwrap();
    let mut processes: HashMap<String, Child> = HashMap::new();
    loop {
        for worker in workers {
            let key = format!("{}: {}", worker.app(), worker.kind()).to_string();
            if worker.work_to_do(&redis_conn) || worker.work_being_done(&redis_conn) {
                if !processes.contains_key(&key) {
                    let command = command_template.replace("%s", &format!("cd {} && {}", worker.path(), &worker.command()));
                    let mut command_to_parse = command.clone();
                    let mut args: Vec<&str> = command_to_parse.parse_cmdline_words().collect();
                    let program = args.remove(0);
                    println!("spawn program {} with args {:?} at path {}", &program, &args, &config_dir);
                    let child = Command::new(&program)
                        .args(args)
                        //.stdin(Stdio::piped())
                        //.stdout(Stdio::piped())
                        .current_dir(config_dir)
                        .spawn()
                        .expect(&format!("Failed to execute command {}", &command));
                    processes.insert(key, child);
                }
            } else {
                if processes.contains_key(&key) {
                    println!("removing process {}", &key);
                    let process = processes.remove(&key).unwrap();
                    kill(process.id() as i32, Signal::SIGTERM).unwrap();
                }
            }
        }
        sleep(interval);
    }
}
