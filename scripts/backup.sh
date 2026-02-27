#!/bin/bash
#
# backup.sh - Backup Rushomon D1 database to SQL file
#
# Usage:
#   ./scripts/backup.sh [options]
#
# Options:
#   -c, --config FILE       Wrangler config file (default: wrangler.toml)
#   -o, --output FILE       Output SQL file (default: auto-generated timestamp)
#   -r, --r2-bucket BUCKET  Upload backup to R2 bucket
#   -z, --compress          Compress backup with gzip
#   -q, --quiet             Quiet mode (no progress, output to stdout)
#   -m, --metadata          Generate metadata file
#   -h, --help              Show this help message
#
# Examples:
#   ./scripts/backup.sh
#   ./scripts/backup.sh -c wrangler.staging.toml -o staging_backup.sql
#   ./scripts/backup.sh -r rushomon-backups -z
#   ./scripts/backup.sh -q > backup.sql
#

set -euo pipefail

# Default values
CONFIG_FILE="wrangler.toml"
OUTPUT_FILE=""
R2_BUCKET=""
COMPRESS=false
QUIET=false
GENERATE_METADATA=false
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors (disabled in quiet mode)
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
  if [ "$QUIET" = false ]; then
    echo -e "${BLUE}[INFO]${NC} $*" >&2
  fi
}

log_success() {
  if [ "$QUIET" = false ]; then
    echo -e "${GREEN}[SUCCESS]${NC} $*" >&2
  fi
}

log_error() {
  echo -e "${RED}[ERROR]${NC} $*" >&2
}

log_warning() {
  if [ "$QUIET" = false ]; then
    echo -e "${YELLOW}[WARNING]${NC} $*" >&2
  fi
}

# Show help
show_help() {
  sed -n '2,/^$/p' "$0" | sed 's/^# \?//'
}

# Parse command-line arguments
parse_arguments() {
  while [[ $# -gt 0 ]]; do
    case $1 in
      -c|--config)
        CONFIG_FILE="$2"
        shift 2
        ;;
      -o|--output)
        OUTPUT_FILE="$2"
        shift 2
        ;;
      -r|--r2-bucket)
        R2_BUCKET="$2"
        shift 2
        ;;
      -z|--compress)
        COMPRESS=true
        shift
        ;;
      -q|--quiet)
        QUIET=true
        shift
        ;;
      -m|--metadata)
        GENERATE_METADATA=true
        shift
        ;;
      -h|--help)
        show_help
        exit 0
        ;;
      *)
        log_error "Unknown option: $1"
        show_help
        exit 1
        ;;
    esac
  done
}

# Extract database name from wrangler config
get_database_name() {
  local config_file="$1"

  if [ ! -f "$config_file" ]; then
    log_error "Config file not found: $config_file"
    exit 1
  fi

  # Extract database name from [[d1_databases]] section
  # This is a simple grep-based parser for TOML
  local db_name=$(grep -A 2 '\[\[d1_databases\]\]' "$config_file" | grep 'binding' | head -1 | cut -d'"' -f2)

  if [ -z "$db_name" ]; then
    log_error "Could not find database binding in $config_file"
    exit 1
  fi

  echo "$db_name"
}

# Get database ID from wrangler config
get_database_id() {
  local config_file="$1"

  if [ ! -f "$config_file" ]; then
    log_error "Config file not found: $config_file"
    exit 1
  fi

  # Extract database_id from [[d1_databases]] section
  local db_id=$(grep -A 2 '\[\[d1_databases\]\]' "$config_file" | grep 'database_id' | head -1 | cut -d'"' -f2)

  if [ -z "$db_id" ]; then
    log_error "Could not find database_id in $config_file"
    exit 1
  fi

  echo "$db_id"
}

# Generate output filename with timestamp
generate_output_filename() {
  local config_file="$1"
  local env="production"

  # Extract environment from config filename
  if [[ "$config_file" == *"staging"* ]]; then
    env="staging"
  elif [[ "$config_file" == *"production"* ]]; then
    env="production"
  fi

  local timestamp=$(date +%Y%m%d_%H%M%S)
  echo "rushomon_${env}_${timestamp}.sql"
}

# Export database to SQL file
export_database() {
  local config_file="$1"
  local output_file="$2"
  local db_name=$(get_database_name "$config_file")

  log_info "Exporting database '$db_name' from config: $config_file"

  # Use wrangler d1 export
  if [ "$QUIET" = true ]; then
    # In quiet mode, output to stdout
    wrangler d1 export "$db_name" --remote --config "$config_file" 2>/dev/null || {
      log_error "Database export failed"
      exit 1
    }
  else
    # Normal mode, save to file (hide download URL but preserve errors)
    wrangler d1 export "$db_name" --remote --output "$output_file" --config "$config_file" 2>&1 | grep -v "You can also download your export from the following URL" || {
      log_error "Database export failed"
      exit 1
    }
    log_success "Database exported to: $output_file"
  fi
}

# Compress backup file
compress_backup() {
  local file="$1"

  log_info "Compressing backup..."
  gzip "$file" || {
    log_error "Compression failed"
    exit 1
  }

  log_success "Backup compressed: ${file}.gz"
  echo "${file}.gz"
}

# Upload backup to R2
upload_to_r2() {
  local file="$1"
  local bucket="$2"
  local config_file="$3"
  local filename=$(basename "$file")

  log_info "Uploading backup to R2 bucket: $bucket"

  wrangler r2 object put "${bucket}/${filename}" --file "$file" --remote --config "$config_file" || {
    log_error "R2 upload failed"
    exit 1
  }

  log_success "Backup uploaded to R2: ${bucket}/${filename}"
}

# Generate metadata file
generate_metadata() {
  local backup_file="$1"
  local config_file="$2"
  local metadata_file="${backup_file%.sql}.meta.json"
  local db_name=$(get_database_name "$config_file")

  log_info "Generating metadata..."

  # Get row counts for all tables
  local tables=$(wrangler d1 execute "$db_name" --remote --config "$config_file" --command "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '_cf_%';" --json 2>/dev/null | jq -r '.[0].results[].name' 2>/dev/null || echo "")

  local row_counts="{"
  local first=true

  if [ -n "$tables" ]; then
    while IFS= read -r table; do
      if [ -n "$table" ]; then
        local count=$(wrangler d1 execute "$db_name" --remote --config "$config_file" --command "SELECT COUNT(*) as count FROM $table;" --json 2>/dev/null | jq -r '.[0].results[0].count' 2>/dev/null || echo "0")

        if [ "$first" = true ]; then
          first=false
        else
          row_counts+=","
        fi
        row_counts+="\"$table\":$count"
      fi
    done <<< "$tables"
  fi

  row_counts+="}"

  # Create metadata JSON
  cat > "$metadata_file" <<EOF
{
  "backup_file": "$(basename "$backup_file")",
  "timestamp": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "database_name": "$db_name",
  "config_file": "$config_file",
  "file_size": $(stat -f%z "$backup_file" 2>/dev/null || stat -c%s "$backup_file" 2>/dev/null || echo "0"),
  "row_counts": $row_counts
}
EOF

  log_success "Metadata generated: $metadata_file"
}

# Main function
main() {
  parse_arguments "$@"

  cd "$PROJECT_ROOT"

  # Check if wrangler is installed
  if ! command -v wrangler &>/dev/null; then
    log_error "wrangler CLI not found"
    log_info "Install with: npm install -g wrangler"
    exit 1
  fi

  # Check if config file exists
  if [ ! -f "$CONFIG_FILE" ]; then
    log_error "Config file not found: $CONFIG_FILE"
    exit 1
  fi

  # Generate output filename if not specified
  if [ -z "$OUTPUT_FILE" ]; then
    OUTPUT_FILE=$(generate_output_filename "$CONFIG_FILE")
  fi

  # Export database
  export_database "$CONFIG_FILE" "$OUTPUT_FILE"

  # In quiet mode, we're done (output went to stdout)
  if [ "$QUIET" = true ]; then
    exit 0
  fi

  # Compress if requested
  local final_file="$OUTPUT_FILE"
  if [ "$COMPRESS" = true ]; then
    final_file=$(compress_backup "$OUTPUT_FILE")
  fi

  # Generate metadata if requested
  if [ "$GENERATE_METADATA" = true ]; then
    generate_metadata "$OUTPUT_FILE" "$CONFIG_FILE"
  fi

  # Upload to R2 if requested
  if [ -n "$R2_BUCKET" ]; then
    upload_to_r2 "$final_file" "$R2_BUCKET" "$CONFIG_FILE"
  fi

  log_success "Backup completed successfully"
  echo "$final_file"
}

# Run main function
main "$@"
