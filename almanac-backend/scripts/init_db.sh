#!/usr/bin/env bash
set -x
set -eo pipefail

if ! [ -x "$(command -v psql)" ]; then
	echo >&2 "Error: psql is not installed."
	exit 1
fi

if ! [ -x "$(command -v sqlx)" ]; then
	echo >&2 "Error: sqlx is not installed"
	echo >&2 "Use:"
	echo >&2 "	cargo install sqlx-cli --no-default-features --features rustls,postgres"
	echo >&2 "to install it."
	exit 1
fi

# Check if a custom user has been set, otherwise default to 'postgres'
DB_USER="${POSTGRES_USER:=postgres}"
# Check if a custom password has been set, otherwise default to 'postgres'
DB_PASSWORD="${POSTGRES_PASSWORD:=postgres}"
# Check if a custom db name has been set, otherwise default to 'almanac'
DB_NAME="${POSTGRES_DB:=almanac}"
# Check if a custom port has been set, otherwise default to '5434'
DB_PORT="${POSTGRES_PORT:=5434}"
# Check if a custom host has been set, otherwise default to 'localhost'
DB_HOST="${POSTGRES_HOST:=localhost}"

# Launch postgres with Docker
if [[ -z "${SKIP_DOCKER}" ]]
then
	>&2 echo "$(command docker compose -f ./scripts/docker-compose.yaml -p almanac up -d )"
fi

export PGPASSWORD="${DB_PASSWORD}"
until psql -h "${DB_HOST}" -U "${DB_USER}" -p "${DB_PORT}" -d "postgres" -c '\q'; do
	>&2 echo "Postgres is still unavailable - sleeping"
	sleep 1
done

>&2 echo "Postgres is up and running on port ${DB_PORT}!"

DATABASE_URL=postgres://${DB_USER}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/${DB_NAME}
export DATABASE_URL
sqlx database create

sqlx migrate run

>&2 echo "Postgres has been migrated, ready to go!"
