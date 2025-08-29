#!/bin/bash
set -euo pipefail

# Script to generate OpenAPI specification from the Rust project using utoipa

echo "🚀 Generating OpenAPI specification..."

# Create docs directory if it doesn't exist
mkdir -p docs/api

# Use the built-in binary to generate the OpenAPI spec
echo "📋 Running OpenAPI generator binary..."
cargo run --bin generate_openapi --features openapi -p oif-api -- docs/api/openapi.json

echo "✅ OpenAPI generation completed!"
