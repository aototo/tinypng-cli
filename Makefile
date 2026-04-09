# tinypng-cli local development tasks.
#
# Releases are fully automated by .github/workflows/release.yml:
# push a tag `vX.Y.Z` and GitHub Actions will build, upload, and publish.

.PHONY: build test clippy fmt fmt-check clean run

build:
	cargo build --release

test:
	cargo test -- --test-threads=1

clippy:
	cargo clippy --all-targets -- -D warnings

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

clean:
	cargo clean

run:
	cargo run -- $(ARGS)
