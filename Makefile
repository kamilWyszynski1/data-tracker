lint:
	cargo +nightly clippy

migrate:
	DATABASE_URL=db.sqlite3 diesel migration run

