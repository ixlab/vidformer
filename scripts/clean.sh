#!/bin/bash
set -e

echo "Removing temporary files..."
# Remove all .ipynb_checkpoints and __pycache__ directories
find . -type d \( -name ".ipynb_checkpoints" -o -name "__pycache__" \) -exec rm -rf {} +

echo "Removing cell outputs from all .ipynb files..."
# Remove cell outputs from all .ipynb files
find . -name "*.ipynb" -print0 | xargs -0 -I {} jupyter nbconvert --clear-output --inplace {}

echo "Using black to format all .py files..."
black .

echo "Using cargo-fmt to format all .rs files..."
cargo fmt