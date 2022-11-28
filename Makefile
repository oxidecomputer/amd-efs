.PHONY: all tests

CARGO = cargo

all:
	$(CARGO) build

tests:
	$(CARGO) test
	$(CARGO) test --features serde
	$(CARGO) test --features serde,schemars
