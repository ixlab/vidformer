#!/usr/bin/env python3

import subprocess as sp
import os
import requests
import time

current_dir = os.path.dirname(os.path.realpath(__file__))
project_dir = os.path.join(current_dir, "..")
igni_dir = os.path.join(project_dir, "vidformer-igni")

try:
    requests.get("http://localhost:8080/")
    raise Exception("Port 8080 is already in use")
except requests.exceptions.ConnectionError:
    pass

# Postgres
print("Starting Postgres")
igni_docker_compose = os.path.join(igni_dir, "docker-compose-db.yaml")
sp.run(["docker-compose", "-f", igni_docker_compose, "down"], check=True)
sp.run(["docker-compose", "-f", igni_docker_compose, "up", "-d"], check=True)

time.sleep(10)  # Give the database time to apply the init scripts

# Igni admin cli
# These just make sure the binary runs without crashin and can talk to the server
# Mostly a canary to make sure a schema change didn't break the admin cli
print("Running Igni admin cli checks")
vidformer_igni_bin = os.path.join(project_dir, "target", "debug", "vidformer-igni")

sp.run([vidformer_igni_bin, "ping"], check=True)

# Add a user for the tests
test_user = sp.run(
    [vidformer_igni_bin, "user", "add", "--name", "test", "--api-key", "test"],
    check=True,
    capture_output=True,
)
test_user_id = test_user.stdout.decode().strip().split("\n")[0]

source = sp.run(
    [
        vidformer_igni_bin,
        "source",
        "add",
        "--user-id",
        test_user_id,
        "--name",
        "../tos_720p.mp4",
        "--stream-idx",
        "0",
        "--storage-service",
        "fs",
        "--storage-config",
        '{"root":"."}',
    ],
    capture_output=True,
    cwd=igni_dir,
    check=True,
)
source_id = source.stdout.decode().strip()
sp.run([vidformer_igni_bin, "source", "ls"], check=True)
sp.run([vidformer_igni_bin, "source", "rm", source_id], check=True)

spec = sp.run(
    [
        vidformer_igni_bin,
        "spec",
        "add",
        "--user-id",
        test_user_id,
        "--width",
        "1280",
        "--height",
        "720",
        "--pix-fmt",
        "yuv420p",
        "--segment-length",
        "2/1",
        "--frame-rate",
        "30/1",
    ],
    capture_output=True,
    check=True,
)
spec_id = spec.stdout.decode().strip()
sp.run([vidformer_igni_bin, "spec", "ls"], check=True)

tmp_user = sp.run(
    [
        vidformer_igni_bin,
        "user",
        "add",
        "--name",
        "test",
    ],
    check=True,
    capture_output=True,
)
assert len(tmp_user.stdout.decode().strip().split("\n")) == 2
user_id = tmp_user.stdout.decode().strip().split("\n")[0]
sp.run([vidformer_igni_bin, "user", "ls"], check=True)
sp.run([vidformer_igni_bin, "user", "rm", user_id], check=True)

# Igni server
print("Starting Igni...")
igni_env = {**os.environ, "RUST_LOG": "warn"}
igni_proc = sp.Popen(
    [
        vidformer_igni_bin,
        "server",
        "--config",
        f"{project_dir}/vidformer-igni/igni.toml",
    ],
    cwd=igni_dir,
    env=igni_env,
)


# Wait for the server to start up, try to GET localhost:8080 until it returns 200
def wait_for_it(endpoint, timeout):
    start = time.time()
    while time.time() - start < timeout:
        try:
            response = requests.get(endpoint)
            if response.status_code == 200:
                return
        except requests.exceptions.ConnectionError:
            pass
        time.sleep(0.1)
    raise Exception("Timeout waiting for server to start")


wait_for_it("http://localhost:8080/", 10)
print("Igni server started")

# Run the tests
viper_den_script = os.path.join(current_dir, "viper-den.sh")
viper_den_response = sp.run([viper_den_script])

# Cleanup (always run, even if tests failed)
print("Cleaning up")
igni_proc.terminate()
igni_proc.wait()

sp.run(["docker-compose", "-f", igni_docker_compose, "down"], check=True)

if viper_den_response.returncode != 0:
    print("Tests failed!")
    exit(1)
else:
    print("Done!")
