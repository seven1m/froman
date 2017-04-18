extern crate redis;
use self::redis::Commands;

pub trait Worker {
    fn app(&self) -> &String;
    fn path(&self) -> &String;
    fn namespace(&self) -> &String;
    fn command(&self) -> &String;
    fn kind(&self) -> String;
    fn work_to_do(&self, &redis::Connection) -> bool;
    fn work_being_done(&self, &redis::Connection) -> bool;
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

    fn namespace(&self) -> &String {
        &self.namespace
    }

    fn command(&self) -> &String {
        &self.command
    }

    fn kind(&self) -> String {
        "sidekiq".to_string()
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

    fn namespace(&self) -> &String {
        &self.namespace
    }

    fn command(&self) -> &String {
        &self.command
    }

    fn kind(&self) -> String {
        "resque".to_string()
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
