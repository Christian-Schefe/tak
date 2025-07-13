$ErrorActionPreference = "Stop"
echo "Transferring files to server..."
ssh tak_server "mkdir -p /root/app/certs"
sleep 1
scp certs/certificate.pem certs/key.pem tak_server:/root/app/certs/
sleep 1
scp docker-compose.prod.yml Caddyfile.prod tak_server:/root/app/