# Changelog

## 1.5.1 - Sep 20, 2022

- Chore: Upgrade dependencies with Dependabot security alerts

## 1.5.0 - Mar 10, 2022

- Fix for child process to exit so we don't have zombie processes
- Fix: Refactor working directory resolution and how we stop processes
- Chore: Update dependencies and fix Rust deprecation warnings
- Docs: Add section to readme about command template shell `exec`

## 1.4.0 - May 31, 2019

- Feature: Check for scheduled jobs in sidekiq too

## 1.3.0 - Aug 6, 2018

- Feature: Add timeout config option for specifying how long to wait before stopping worker

## 1.2.1 - Jun 13, 2018

- Feature: Add debug mode switch
- Fix: Show better error message if panicking 

## 1.2.0 - May 24, 2018

- Fix: Monitor queues without a namespace too

## 1.1.0 - Feb 23, 2018

- Fix: Better handle errors connecting to Redis

## 1.0.0 - May 18, 2017

First release!
