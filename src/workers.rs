use chrono;
use chrono::prelude::*;

use redis;
use redis::Commands;
use cmdline_words_parser::StrExt;
use std::process::{Command, Stdio, Child};

pub trait Worker {
    fn app(&self) -> &String;
    fn path(&self) -> &String;
    fn command(&self) -> &String;
    fn kind(&self) -> &str;
    fn work_to_do(&self, &redis::Connection) -> bool;
    fn work_being_done(&self, &redis::Connection) -> bool;
    fn process(&self) -> &Option<Child>;
    fn terminate_at(&self) -> &Option<DateTime<Local>>;
    fn set_process(&mut self, Option<Child>);
    fn set_terminate_at(&mut self, Option<DateTime<Local>>);

    fn command_binary_and_args(&self, command_template: &str) -> (String, Vec<String>) {
        let mut command_to_parse = command_template.replace("%s", self.command());
        let mut args: Vec<String> = command_to_parse.parse_cmdline_words().map(|a| a.to_string()).collect();
        let program = args.remove(0).to_string();
        (program, args)
    }

    fn key(&self) -> String {
        format!("{}: {}", self.app(), self.kind())
    }

    fn absolute_path(&self, config_dir: &str) -> String {
        if self.path().starts_with("/") {
            self.path().to_string()
        } else {
            format!("{}/{}", config_dir, self.path())
        }
    }

    fn process_id(&self) -> u32 {
        match *self.process() {
            Some(ref process) => process.id(),
            _ => 0u32
        }
    }
}

pub struct Sidekiq {
    pub app: String,
    pub path: String,
    pub namespace: String,
    pub command: String,
    pub process: Option<Child>,
    pub terminate_at: Option<DateTime<Local>>
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
            redis_conn.hget(format!("{}:{}", self.namespace, p), "busy").unwrap_or(0)
        }).collect();
        counts.iter().fold(0i32, |a, &b| a + b) > 0
    }

    fn process(&self) -> &Option<Child> {
        &self.process
    }

    fn terminate_at(&self) -> &Option<DateTime<Local>> {
        &self.terminate_at
    }

    fn set_process(&mut self, process: Option<Child>) {
        self.process = process;
    }

    fn set_terminate_at(&mut self, terminate_at: Option<DateTime<Local>>) {
        self.terminate_at = terminate_at;
    }
}

pub struct Resque {
    pub app: String,
    pub path: String,
    pub namespace: String,
    pub command: String,
    pub process: Option<Child>,
    pub terminate_at: Option<DateTime<Local>>
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

    fn process(&self) -> &Option<Child> {
        &self.process
    }

    fn terminate_at(&self) -> &Option<DateTime<Local>> {
        &self.terminate_at
    }

    fn set_process(&mut self, process: Option<Child>) {
        self.process = process;
    }

    fn set_terminate_at(&mut self, terminate_at: Option<DateTime<Local>>) {
        self.terminate_at = terminate_at;
    }
}
