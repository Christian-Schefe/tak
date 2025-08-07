export SURREALDB_USER=root
export SURREALDB_PASS=secret
export JWT_SECRET=secret
rm -rf ./target/dx/
dx serve --platform web
