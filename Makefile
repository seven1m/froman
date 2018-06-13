build:
	cargo build --release

package:
	$(eval VERSION := $(shell grep "version =" Cargo.toml | awk '{ print $$3 }' | sed 's/"//g'))
	cp target/release/froman .
	zip froman-$(VERSION)-macos.zip ./froman
	rm ./froman
