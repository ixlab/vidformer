#!/usr/bin/env bash

set -e

cd "$(dirname "$0")/../viper-den"

if ! curl -s localhost:8080/ > /dev/null; then
    echo "Igni server not running."
    exit 1
fi

pytest . -vv
