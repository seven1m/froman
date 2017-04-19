extern crate redis;
extern crate cmdline_words_parser;

use self::redis::Commands;
use self::cmdline_words_parser::StrExt;
use std::process::{Command, Stdio, Child};

pub trait Worker {
    fn app(&self) -> &String;
    fn path(&self) -> &String;
    fn command(&self) -> &String;
    fn kind(&self) -> &str;
    fn work_to_do(&self, &redis::Connection) -> bool;
    fn work_being_done(&self, &redis::Connection) -> bool;

    fn command_binary_and_args(&self, command_template: &str) -> (String, Vec<String>) {
        let mut command_to_parse = command_template.replace("%s", &format!("cd {} && {}", self.path(), self.command()));
        let mut args: Vec<String> = command_to_parse.parse_cmdline_words().map(|a| a.to_string()).collect();
        let program = args.remove(0).to_string();
        (program, args)
    }

    fn key(&self) -> String {
        format!("{}: {}", self.app(), self.kind())
    }

    fn spawn(&self, command_template: &str, config_dir: &str) -> Child {
        let (program, args) = self.command_binary_and_args(command_template);
        println!("spawn process {} with args {:?} at path {}", &program, &args, &config_dir);
        Command::new(&program)
            .args(args)
            .stdin(Stdio::piped())
            //.stdout(Stdio::piped())
            .current_dir(config_dir)
            .spawn()
            .expect(&format!("Failed to execute command {}", self.command()))
    }
}

pub struct Sidekiq {
    pub app: String,
    pub path: String,
    pub namespace: String,
    pub command: String
}

impl Worker for Sidekiq {
    fn app(&self) -> &String {
        &self.app
    }

    fn path(&self) -> &String {
        &self.path
    }

    fn command(&self) -> &String {
        &self.command
    }

    fn kind(&self) -> &str {
        "sidekiq"
    }

    fn work_to_do(&self, redis_conn: &redis::Connection) -> bool {
        let queues: Vec<String> = redis_conn.keys(format!("{}:queue:*", self.namespace)).unwrap();
        let counts: Vec<i32> = queues.iter().map(|q| {
            redis_conn.llen(q).unwrap()
        }).collect();
        counts.iter().fold(0i32, |a, &b| a + b) > 0
    }

    fn work_being_done(&self, redis_conn: &redis::Connection) -> bool {
        let processes: Vec<String> = redis_conn.smembers(format!("{}:processes", self.namespace)).unwrap();
        let counts: Vec<i32> = processes.iter().map(|p| {
            redis_conn.hget(format!("{}:{}", self.namespace, p), "busy").unwrap()
        }).collect();
        counts.iter().fold(0i32, |a, &b| a + b) > 0
    }
}

pub struct Resque {
    pub app: String,
    pub path: String,
    pub namespace: String,
    pub command: String
}

impl Worker for Resque {
    fn app(&self) -> &String {
        &self.app
    }

    fn path(&self) -> &String {
        &self.path
    }

    fn command(&self) -> &String {
        &self.command
    }

    fn kind(&self) -> &str {
        "resque"
    }

    fn work_to_do(&self, redis_conn: &redis::Connection) -> bool {
        let queues: Vec<String> = redis_conn.smembers(format!("{}:queues", self.namespace)).unwrap();
        let counts: Vec<i32> = queues.iter().map(|q| {
            redis_conn.llen(format!("{}:queue:{}", self.namespace, q)).unwrap()
        }).collect();
        counts.iter().fold(0i32, |a, &b| a + b) > 0
    }

    fn work_being_done(&self, _redis_conn: &redis::Connection) -> bool {
        false // no way to know if work is being done in Resque
    }
}
