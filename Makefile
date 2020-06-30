# Run test one by one because there are some tests
# that are stateful e.g. creating filesystem
test:
	cargo test -- --test-threads 1


build:
	cargo build

install:
	cargo install --force --path .

.PHONY: build install test
