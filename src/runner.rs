use chrono;
use chrono::prelude::*;
use colors::*;
use config::*;
use errors::*;
use redis;
use std::io;
use std::io::prelude::*;
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::thread::sleep;
use std::time::Duration;
use workers::*;

pub struct Runner<'a> {
    config: &'a Config,
}

impl<'a> Runner<'a> {
    pub fn new(config: &'a Config) -> Runner<'a> {
        Runner { config }
    }

    pub fn run(&mut self, workers: &mut Vec<Box<dyn Worker>>) -> FromanResult<()> {
        let interval = Duration::from_secs(2);
        let redis = redis::Client::open(self.config.redis_url.as_str()).unwrap();
        let redis_conn = redis.get_connection()?;
        let label_size = self.get_label_size(&workers);
        println!("Froman monitoring queues...");
        loop {
            for (worker_index, mut worker) in workers.iter_mut().enumerate() {
                let color = COLORS[worker_index % COLORS.len()];
                self.work(&mut worker, &redis_conn, color, label_size)?;
            }
            sleep(interval);
        }
    }

    fn work(
        &self,
        worker: &mut Box<dyn Worker>,
        redis_conn: &redis::Connection,
        color: &str,
        label_size: usize,
    ) -> FromanResult<()> {
        if worker.work_to_do(&redis_conn)? || worker.work_being_done(&redis_conn)? {
            if worker.process().is_some() {
                worker.set_terminate_at(None);
            } else {
                log(worker.app(), label_size, color, "STARTING\n");
                let mut process = self.spawn(worker);
                self.pipe_output(
                    process.stdout.take().unwrap(),
                    worker.app(),
                    label_size,
                    color,
                );
                self.pipe_output(
                    process.stderr.take().unwrap(),
                    worker.app(),
                    label_size,
                    color,
                );
                worker.set_process(Some(process));
            }
        } else {
            if worker.process().is_some() {
                let now = Local::now();
                if worker.terminate_at().is_some() {
                    if worker.terminate_at().unwrap() <= now {
                        log(worker.app(), label_size, color, "STOPPING\n");
                        worker.stop_process();
                    }
                } else {
                    let terminate_at = now + chrono::Duration::seconds(self.config.timeout as i64);
                    worker.set_terminate_at(Some(terminate_at));
                }
            }
        }
        Ok(())
    }

    fn get_label_size(&self, workers: &Vec<Box<dyn Worker>>) -> usize {
        workers.iter().map(|w| w.app().len()).max().unwrap()
    }

    fn spawn(&self, worker: &Box<dyn Worker>) -> Child {
        let (program, args) = worker.command_binary_and_args(&self.config.command_template);
        let working_directory = self.config.path_relative_to_config_dir(worker.path());
        Command::new(&program)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(working_directory)
            .env_remove("PATH") // rvm won't update the current ruby version if a ruby version is already present in the PATH
            .env_remove("RUBY_VERSION")
            .env_remove("RBENV_VERSION")
            .env_remove("RBENV_GEMSET_ALREADY")
            .env_remove("RBENV_DIR")
            .spawn()
            .expect("failed to launch worker")
    }

    fn pipe_output<T: 'static + Read + Send>(
        &self,
        mut out: T,
        label: &str,
        label_size: usize,
        color: &str,
    ) {
        let label = label.to_owned();
        let color = color.to_owned();
        thread::spawn(move || loop {
            let mut buf = [0; 10000];
            match out.read(&mut buf) {
                Ok(count) => {
                    if count > 0 {
                        log(
                            &label,
                            label_size,
                            &color,
                            &String::from_utf8_lossy(&buf).replace("\u{0}", ""),
                        );
                    } else {
                        break;
                    }
                }
                Err(_) => break,
            }
        });
    }
}

fn left_pad(str: &str, length: usize) -> String {
    " ".repeat(length - str.len()) + str
}

fn log(label: &str, label_size: usize, color: &str, message: &str) {
    if message.trim().is_empty() {
        return;
    }
    for line in message.trim().split("\n") {
        println!(
            "{}: {}",
            colorize(&left_pad(&label, label_size), color),
            line
        );
        io::stdout().flush().ok().expect("Could not flush stdout");
    }
}
