# Froman

**NO LONGER MAINTAINED**

Run Sidekiq/Resque workers for multiple apps only when there is work to be done. 

You can use it in place of Foreman if you need to manage several instances of Sidekiq and/or Resque and don't necessarily need them all to run at once.

## Why?

We have a dozen apps all running Sidekiq (and some running Resque). In development, I don't need/want all of them running at once, so I built this.

This might be super useful if you are using Vagrant or VM for development and want to limit its RAM usage.

**This is probably not useful in a production environment.** But, let me know if you think it is!

## Features

* Watches Redis and only starts Sidekiq/Resque processes when there is work to be done
* Spins down inactive Sidekiq/Resque processes
* Sends TERM signal to child processes so they shut down properly
* Saves memory

## Installation

You can download the binary [here](https://github.com/seven1m/froman/releases).

If your platform binary is not available there, you'll need to build with Rust:

```
curl https://sh.rustup.rs -sSf | sh
cargo build --release
```

## Configure and Run

Froman is configured with a `froman.yml` file like this:

```yaml
apps:
  check-ins:
    path: ../check-ins
    sidekiq:
      command: bundle exec sidekiq
      db: 12
  services:
    path: ../services
    resque:
      namespace: planning_center_development
      command: bundle exec resque-pool --environment development
    sidekiq:
      namespace: services-development.sidekiq
      command: bundle exec sidekiq
```

`namespace` is the Redis namespace to monitor for jobs.


Run Froman:

```
froman
```

## Command Template

If you need to execute a shell script in order to set up the environment for your command, you can
use the top-level `command_template` config item, like so:

```
command_template: "bash -c 'export FOO=bar && exec %s'"
```

Just be sure to use `exec %s` to replace bash with the actual worker process. This will ensure that
the kill signal is sent to the proper child when stopping a worker.

## Copyright

Copyright Tim Morgan, Licensed MIT.
