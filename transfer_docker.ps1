$ErrorActionPreference = "Stop"
echo "Building Docker image..."
docker build --build-arg SERVER_HOSTNAME=tak.meeshroom.xyz -t tak-app .
echo "Saving Docker image to tar file..."
docker save -o target/tak-app.tar tak-app:latest
sleep 1
echo "Transferring files to server..."
scp target/tak-app.tar tak_server:/root/app/
sleep 1
ssh tak_server "mkdir -p /root/app/data && chmod 777 /root/app/data && docker load -i /root/app/tak-app.tar && docker image prune"