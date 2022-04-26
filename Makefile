lint:
	cargo +nightly clippy

migrate:
	DATABASE_URL=db.sqlite3 diesel migration run

pslq-docker:
	docker run --name postgres-test -p 5432:5432 -e POSTGRES_PASSWORD=password -d postgres:14