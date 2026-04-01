#!/bin/bash
# Generate OpenAPI specification for Rushomon API
# This script builds the project and extracts the OpenAPI JSON

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_DIR"

echo "📝 Generating OpenAPI specification..."

# Create output directory
OUTPUT_DIR="${PROJECT_DIR}/docs/api"
mkdir -p "$OUTPUT_DIR"

# Generate OpenAPI JSON using the binary (redirect stderr to /dev/null to avoid mixing with JSON)
echo "🔨 Building and running OpenAPI generator..."
cargo build --bin generate_openapi --features openapi-gen 2>/dev/null

# Run the binary and capture output
./target/debug/generate_openapi > "$OUTPUT_DIR/openapi.json" 2>/dev/null

if [ $? -eq 0 ]; then
    echo "✅ OpenAPI spec generated successfully!"
    echo "📁 Output: $OUTPUT_DIR/openapi.json"
    
    # Show some stats
    LINES=$(wc -l < "$OUTPUT_DIR/openapi.json")
    SIZE=$(du -h "$OUTPUT_DIR/openapi.json" | cut -f1)
    echo "📊 Spec size: $SIZE ($LINES lines)"
else
    echo "❌ Failed to generate OpenAPI spec"
    exit 1
fi
