pub enum Worker {
    Resque {
        app: String,
        path: String,
        namespace: String,
        command: String,
    },
    Sidekiq {
        app: String,
        path: String,
        namespace: String,
        command: String,
    }
}

impl Worker {
    pub fn app(&self) -> String {
        match *self {
            Worker::Resque { ref app, .. } => app,
            Worker::Sidekiq { ref app, .. } => app
        }.to_string()
    }

    pub fn command(&self) -> String {
        match *self {
            Worker::Resque { ref command, .. } => command,
            Worker::Sidekiq { ref command, .. } => command
        }.to_string()
    }

    pub fn kind(&self) -> String {
        match *self {
            Worker::Resque { .. } => "resque",
            Worker::Sidekiq { .. } => "sidekiq"
        }.to_string()
    }
}
