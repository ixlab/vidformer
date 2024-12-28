#!/usr/bin/env python3

import subprocess as sp
import os
import requests
import time

current_dir = os.path.dirname(os.path.realpath(__file__))
project_dir = os.path.join(current_dir, "..")
igni_dir = os.path.join(project_dir, "vidformer-igni")

# Postgres
print("Starting Postgres")
igni_docker_compose = os.path.join(igni_dir, "docker-compose.yaml")
sp.run(["docker-compose", "-f", igni_docker_compose, "down"], check=True)
sp.run(["docker-compose", "-f", igni_docker_compose, "up", "-d"], check=True)

# Igni
print("Starting Igni...")
vidformer_igni_bin = os.path.join(project_dir, "target", "debug", "vidformer-igni")
igni_env = {**os.environ, "RUST_LOG": "info"}
igni_proc = sp.Popen([vidformer_igni_bin, "server"], cwd=igni_dir, env=igni_env)


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
sp.run([viper_den_script], check=True)

# Cleanup
print("Cleaning up")
igni_proc.terminate()
igni_proc.wait()

sp.run(["docker-compose", "-f", igni_docker_compose, "down"], check=True)

print("Done!")
