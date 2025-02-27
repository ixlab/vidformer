set -e

wait-for-it postgres:5432
sleep 10 # Wait for init scripts to finish

CURRENT_USERS=$(/usr/local/bin/vidformer-igni user ls)
if [[ $CURRENT_USERS == *"Local Admin"* ]]; then
    echo "Local Admin user already exists"
else
    /usr/local/bin/vidformer-igni user add --name "Local Admin" --permissions test --api-key local
    echo "Created local admin user"
fi

echo "Starting local igni server"
echo "    VF_IGNI_ENDPOINT=http://localhost:8080"
echo "    VF_IGNI_API_KEY=local"

/usr/local/bin/vidformer-igni server --config /etc/igni-local.toml