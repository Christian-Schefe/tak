$Env:SURREALDB_USER="root"
$Env:SURREALDB_PASS="secret"
$Env:JWT_SECRET="secret"
$Env:RUST_BACKTRACE="1"
if (Test-Path ./target/dx/) {
    Remove-Item -Recurse -Force ./target/dx/
}
dx serve --platform web
