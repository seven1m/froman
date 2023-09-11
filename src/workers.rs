use chrono::prelude::*;

use cmdline_words_parser::StrExt;
use errors::*;
use nix::sys::signal::{kill, Signal};
use nix::sys::wait::waitpid;
use nix::unistd::Pid;
use redis;
use redis::Commands;
use std::process::Child;

pub trait Worker {
    fn app(&self) -> &String;
    fn path(&self) -> &String;
    fn command(&self) -> &String;
    fn kind(&self) -> &str;
    fn db(&self) -> &String;
    fn work_to_do(&self, _: &redis::Connection) -> FromanResult<bool>;
    fn work_being_done(&self, _: &redis::Connection) -> FromanResult<bool>;
    fn process(&self) -> &Option<Child>;
    fn terminate_at(&self) -> &Option<DateTime<Local>>;
    fn set_process(&mut self, _: Option<Child>);
    fn set_terminate_at(&mut self, _: Option<DateTime<Local>>);
    fn namespace(&self) -> String;

    fn command_binary_and_args(&self, command_template: &str) -> (String, Vec<String>) {
        let mut command_to_parse = command_template.replace("%s", self.command());
        let mut args: Vec<String> = command_to_parse
            .parse_cmdline_words()
            .map(|a| a.to_string())
            .collect();
        let program = args.remove(0).to_string();
        (program, args)
    }

    fn process_id(&self) -> u32 {
        match *self.process() {
            Some(ref process) => process.id(),
            _ => 0u32,
        }
    }

    fn stop_process(&mut self) {
        let pid = Pid::from_raw(self.process_id() as i32);
        kill(pid, Signal::SIGINT).unwrap();
        waitpid(pid, None).unwrap();
        self.set_process(None);
    }

    fn namespaced(&self, key: &str) -> String {
        let namespace = self.namespace();
        if namespace.is_empty() {
            key.to_string()
        } else {
            format!("{}:{}", namespace, key)
        }
    }
}

pub struct Sidekiq {
    pub app: String,
    pub path: String,
    pub namespace: String,
    pub db: String,
    pub command: String,
    pub process: Option<Child>,
    pub terminate_at: Option<DateTime<Local>>,
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

    fn db(&self) -> &String {
        &self.db
    }

    fn work_to_do(&self, redis_conn: &redis::Connection) -> FromanResult<bool> {
        let queue_key = self.namespaced("queue:*");
        let queues: Vec<String> = redis_conn.keys(queue_key)?;
        let counts: Vec<i32> = queues
            .iter()
            .map(|q| redis_conn.llen(q).unwrap_or(0))
            .collect();
        let schedule_queue_key = self.namespaced("schedule");
        let schedule_count = redis_conn
            .zcount(schedule_queue_key, "0", "+inf")
            .unwrap_or(0);
        Ok(counts.iter().sum::<i32>() > 0 || schedule_count > 0)
    }

    fn work_being_done(&self, redis_conn: &redis::Connection) -> FromanResult<bool> {
        let processes_key = self.namespaced("processes");
        let processes: Vec<String> = redis_conn.smembers(processes_key)?;
        let counts: Vec<i32> = processes
            .iter()
            .map(|p| {
                let key = self.namespaced(p);
                redis_conn.hget(key, "busy").unwrap_or(0)
            })
            .collect();
        Ok(counts.iter().sum::<i32>() > 0)
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

    fn namespace(&self) -> String {
        self.namespace.to_owned()
    }
}

pub struct Resque {
    pub app: String,
    pub path: String,
    pub db: String,
    pub namespace: String,
    pub command: String,
    pub process: Option<Child>,
    pub terminate_at: Option<DateTime<Local>>,
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

    fn db(&self) -> &String {
        &self.db
    }

    fn kind(&self) -> &str {
        "resque"
    }

    fn work_to_do(&self, redis_conn: &redis::Connection) -> FromanResult<bool> {
        let queues_key = self.namespaced("queues");
        let queues: Vec<String> = redis_conn.smembers(queues_key)?;
        let counts: Vec<i32> = queues
            .iter()
            .map(|q| {
                let queue_key = self.namespaced(&format!("queue:{}", q));
                redis_conn.llen(queue_key).unwrap_or(0)
            })
            .collect();
        Ok(counts.iter().sum::<i32>() > 0)
    }

    fn work_being_done(&self, _redis_conn: &redis::Connection) -> FromanResult<bool> {
        Ok(false) // no way to know if work is being done in Resque
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

    fn namespace(&self) -> String {
        self.namespace.to_owned()
    }
}
