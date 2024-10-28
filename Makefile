PROFILE                 := release
LOG_LEVE                := info

check:
	@cargo check --all-targets --examples

fix:
	# @cargo machete --fix
	@cargo fix --all-targets --allow-dirty --allow-staged

build:
	@cargo build --profile ${PROFILE}

fmt:
	@cargo fmt

run:
	RUST_LOG=${LOG_LEVE} @cargo run --package qed-cli

test:
	@cargo test -- --nocapture

update-snapshots:
	@cargo insta review

.PHONE: check fix build format run test update-snapshots
