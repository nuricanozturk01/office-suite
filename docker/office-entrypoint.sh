#!/usr/bin/env bash
# Entrypoint of the single all-in-one image (docker/office.Dockerfile): brings up postgres,
# redis, the api binary and nginx in one container, in that order, and forwards shutdown.
set -euo pipefail

PGBIN=/usr/lib/postgresql/18/bin
DATA_DIR=/data
PG_DATA="$DATA_DIR/postgres"
REDIS_DIR="$DATA_DIR/redis"
SECRET_FILE="$DATA_DIR/jwt_secret"

mkdir -p "$PG_DATA" "$REDIS_DIR"
chown -R postgres:postgres "$PG_DATA"
chown -R redis:redis "$REDIS_DIR"

# Generated once, then persisted on the /data volume so restarts don't invalidate every
# refresh token.
if [ ! -s "$SECRET_FILE" ]; then
    head -c 32 /dev/urandom | od -An -tx1 | tr -d ' \n' >"$SECRET_FILE"
fi
chmod 600 "$SECRET_FILE"

if [ ! -s "$PG_DATA/PG_VERSION" ]; then
    su postgres -c "'$PGBIN/initdb' -D '$PG_DATA' --auth=trust --no-sync"
fi

su postgres -c "'$PGBIN/pg_ctl' -D '$PG_DATA' -l '$PG_DATA/startup.log' -o '-c listen_addresses=127.0.0.1' start"

until su postgres -c "'$PGBIN/pg_isready' -h 127.0.0.1" >/dev/null 2>&1; do
    sleep 1
done

su postgres -c "'$PGBIN/psql' -h 127.0.0.1 -tAc \"select 1 from pg_roles where rolname='office'\"" | grep -q 1 \
    || su postgres -c "'$PGBIN/createuser' -h 127.0.0.1 office"
su postgres -c "'$PGBIN/psql' -h 127.0.0.1 -tAc \"select 1 from pg_database where datname='office'\"" | grep -q 1 \
    || su postgres -c "'$PGBIN/createdb' -h 127.0.0.1 -O office office"

redis-server --dir "$REDIS_DIR" --bind 127.0.0.1 --port 6379 --daemonize no &
REDIS_PID=$!

export DATABASE_URL="postgres://office@127.0.0.1:5432/office"
export REDIS_URL="redis://127.0.0.1:6379"
export JWT_SECRET_FILE="$SECRET_FILE"
export API_ADDR="127.0.0.1:8081"
export COOKIE_SECURE="${COOKIE_SECURE:-false}"

/usr/local/bin/api &
API_PID=$!

nginx -g 'daemon off;' &
NGINX_PID=$!

shutdown() {
    kill "$NGINX_PID" "$API_PID" "$REDIS_PID" 2>/dev/null || true
    su postgres -c "'$PGBIN/pg_ctl' -D '$PG_DATA' stop -m fast" 2>/dev/null || true
    exit 0
}
trap shutdown TERM INT

wait -n "$NGINX_PID" "$API_PID" "$REDIS_PID"
shutdown
