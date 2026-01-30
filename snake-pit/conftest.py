"""
Pytest configuration for vidformer integration tests.

This replaces the snake-pit.py orchestration script with proper pytest fixtures.
Run tests with: pytest snake-pit/ -vv
"""

import os
import subprocess as sp
import time

import pytest
import requests

# Paths
PROJECT_DIR = os.path.dirname(os.path.dirname(os.path.realpath(__file__)))

# Fail fast if running from wrong directory
_cwd = os.getcwd()
_snake_pit_dir = os.path.join(PROJECT_DIR, "snake-pit")
if _cwd != _snake_pit_dir:
    pytest.exit(
        f"Tests must be run from snake-pit directory.\n"
        f"  Current directory: {_cwd}\n"
        f"  Expected directory: {_snake_pit_dir}\n"
        f"  Run: cd {_snake_pit_dir} && pytest -vv",
        returncode=1,
    )
IGNI_DIR = os.path.join(PROJECT_DIR, "vidformer-igni")
IGNI_BIN = os.path.join(PROJECT_DIR, "target", "debug", "vidformer-igni")
DOCKER_COMPOSE_FILE = os.path.join(IGNI_DIR, "docker-compose-db.yaml")
IGNI_CONFIG = os.path.join(IGNI_DIR, "igni.toml")


def _port_in_use(port: int) -> bool:
    """Check if a port is already in use."""
    try:
        requests.get(f"http://localhost:{port}/", timeout=1)
        return True
    except requests.exceptions.ConnectionError:
        return False
    except requests.exceptions.ReadTimeout:
        return True


def _wait_for_postgres(timeout: int = 30) -> None:
    """Wait for PostgreSQL to be ready and accepting connections."""
    start = time.time()
    while time.time() - start < timeout:
        result = sp.run(
            ["docker", "exec", "igni_db", "pg_isready", "-U", "igni"],
            capture_output=True,
        )
        if result.returncode == 0:
            return
        time.sleep(0.5)
    raise TimeoutError(f"PostgreSQL not ready after {timeout}s")


@pytest.fixture(scope="session")
def igni_env():
    """Environment variables for Igni CLI commands."""
    return {
        **os.environ,
        "IGNI_DB": "postgres://igni:igni@localhost:5432/igni",
        "RUST_LOG": "warn",
    }


@pytest.fixture(scope="session")
def docker_services(igni_env):
    """Start Postgres + Valkey containers via docker-compose."""
    if _port_in_use(8080):
        pytest.exit("Port 8080 is already in use", returncode=1)

    print("\nStarting Postgres + Valkey containers...")
    sp.run(["docker-compose", "-f", DOCKER_COMPOSE_FILE, "down"], check=True)
    sp.run(["docker-compose", "-f", DOCKER_COMPOSE_FILE, "up", "-d"], check=True)

    # Wait for database to be ready
    _wait_for_postgres()

    yield

    # Cleanup
    print("\nStopping containers...")
    sp.run(["docker-compose", "-f", DOCKER_COMPOSE_FILE, "down"], check=True)


@pytest.fixture(scope="session")
def igni_cli_setup(docker_services, igni_env):
    """
    Run Igni admin CLI checks and create test fixtures.

    This is a canary to ensure schema changes don't break the admin CLI.
    """
    print("\nRunning Igni admin CLI checks...")

    # Check CLI connects to the database
    sp.run([IGNI_BIN, "ping"], check=True, capture_output=True, env=igni_env)

    # Add a test user
    test_user = sp.run(
        [
            IGNI_BIN,
            "user",
            "add",
            "--name",
            "test",
            "--api-key",
            "test",
            "--permissions",
            "test",
        ],
        capture_output=True,
        check=True,
        env=igni_env,
    )
    test_user_id = test_user.stdout.decode().strip().split("\n")[0]

    # Test source CRUD
    source = sp.run(
        [
            IGNI_BIN,
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
        cwd=IGNI_DIR,
        check=True,
        env=igni_env,
    )
    source_id = source.stdout.decode().strip()
    sp.run([IGNI_BIN, "source", "ls"], check=True, capture_output=True, env=igni_env)
    sp.run(
        [IGNI_BIN, "source", "rm", source_id],
        check=True,
        capture_output=True,
        env=igni_env,
    )

    # Test spec CRUD
    spec = sp.run(
        [
            IGNI_BIN,
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
        env=igni_env,
    )
    sp.run([IGNI_BIN, "spec", "ls"], check=True, capture_output=True, env=igni_env)

    # Test user CRUD
    tmp_user = sp.run(
        [
            IGNI_BIN,
            "user",
            "add",
            "--name",
            "tmp_user",
            "--permissions",
            "regular",
        ],
        check=True,
        capture_output=True,
        env=igni_env,
    )
    assert len(tmp_user.stdout.decode().strip().split("\n")) == 2
    user_id = tmp_user.stdout.decode().strip().split("\n")[0]
    sp.run([IGNI_BIN, "user", "ls"], check=True, capture_output=True, env=igni_env)
    sp.run(
        [IGNI_BIN, "user", "rm", user_id], check=True, capture_output=True, env=igni_env
    )

    yield test_user_id


@pytest.fixture(scope="session")
def igni_server(igni_cli_setup, igni_env):
    """Start the Igni HTTP server."""
    print("\nStarting Igni server...")

    proc = sp.Popen(
        [IGNI_BIN, "server", "--config", IGNI_CONFIG],
        cwd=IGNI_DIR,
        env=igni_env,
    )

    # Wait for server to be ready
    sp.run(["wait-for-it", "localhost:8080", "--timeout=15"], check=True)
    print("Igni server started")

    # Set environment variables for tests
    os.environ["VF_IGNI_ENDPOINT"] = "http://localhost:8080"
    os.environ["VF_IGNI_API_KEY"] = "test"

    yield proc

    # Cleanup
    print("\nStopping Igni server...")
    proc.terminate()
    proc.wait()


@pytest.fixture(autouse=True)
def _require_igni(igni_server):
    """
    Auto-use fixture that ensures the Igni server is running for all tests.

    This fixture is automatically applied to every test, triggering the
    entire fixture chain (docker_services -> igni_cli_setup -> igni_server).
    """
    pass
