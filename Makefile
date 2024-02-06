build:
	cargo build --release

build_all:
	which cross || cargo install cross --git https://github.com/cross-rs/cross
	cross build --target=x86_64-unknown-linux-gnu --release
	cross build --target=x86_64-apple-darwin --release
	cross build --target=aarch64-apple-darwin --release

package_all: build_all
	$(eval VERSION := $(shell grep "version =" Cargo.toml | awk '{ print $$3 }' | sed 's/"//g'))
	cd target/x86_64-unknown-linux-gnu/release && zip ../../../froman-$(VERSION)-linux-amd64.zip froman
	cd target/x86_64-apple-darwin/release && zip ../../../froman-$(VERSION)-macos-amd64.zip froman
	cd target/aarch64-apple-darwin/release && zip ../../../froman-$(VERSION)-macos-arm64.zip froman
