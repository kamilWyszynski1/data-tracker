lint:
	cargo +nightly clippy

migrate:
	DATABASE_URL=db.sqlite3 diesel migration run

test:
	cargo t

pslq-docker:
	docker start postgres-test || docker run --name postgres-test -p 5432:5432 -e POSTGRES_PASSWORD=password -d postgres:14

test-integ:
	RUST_LOG=DEBUG INTEGRATION=1 cargo test --package datatracker_rust --test psql -- --test-threads 1