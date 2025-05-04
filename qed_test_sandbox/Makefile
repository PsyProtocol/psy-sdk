shutdown:
	@redis-cli 'FLUSHALL'
	@rm -fr db

compile:
	@cd basic_ups && dargo compile --contract-name=Contract --method-names simple_mint_debug simple_transfer simple_claim

launch: shutdown compile
	@cargo run --release --example full_group_1




