.PHONY: diesel-gen
diesel-gen:
	(cd db ; diesel database reset ; diesel print-schema > src/schema.rs ; diesel_ext -d "Queryable, Debug, Identifiable, AsChangeset, Model, Clone" -t -I "crate::schema::*" > src/models.rs)

.PHONY: migration-redo
migration-redo:
	(cd db ; diesel migration redo)

.PHONY: database-reset
database-reset:
	(cd db ; diesel database reset)

.PHONY: lint
lint:
	cargo +nightly clippy --fix -Z unstable-options --all

.PHONY: fmt
fmt:
	find -type f -name "*.rs" -not -path "*target*" -exec rustfmt --edition 2021 {} \;

.PHONY: test-cargo
test-cargo:
	cargo test --lib --no-fail-fast


.PHONY: build-docker
build-docker:
	DOCKER_BUILDKIT=1 docker \
		build \
		--memory 8g \
		--cpu-shares 4096 \
		--shm-size 8g \
		-t tulip-cli:latest \
		--squash .
	docker image save liquidator-cli:latest -o liquidator_cli.tar
	pigz -f -9 liquidator_cli.tar

.PHONY: build-cli
build-cli:
	(cargo build --release; cp target/release/cli liquidator-cli)

.PHONY: build-cli-debug
build-cli-debug:
	(cargo build ; cp target/debug/cli liquidator-cli)
