#!/bin/bash
#
# restore.sh - Restore Rushomon D1 database from SQL backup
#
# Usage:
#   ./scripts/restore.sh [options] <backup_file>
#
# Options:
#   -c, --config FILE    Wrangler config file (default: wrangler.toml)
#   -f, --force          Force restore without confirmation
#   -n, --dry-run        Preview what would be restored (don't actually restore)
#   -b, --backup         Create safety backup before restore (default: true)
#   --no-backup          Skip safety backup creation
#   -v, --verify         Verify restoration by comparing row counts
#   -h, --help           Show this help message
#
# Examples:
#   ./scripts/restore.sh backups/rushomon_20260226_120000.sql
#   ./scripts/restore.sh -f backup.sql
#   ./scripts/restore.sh -c wrangler.staging.toml prod_backup.sql
#   ./scripts/restore.sh --dry-run backup.sql
#

set -euo pipefail

# Default values
CONFIG_FILE="wrangler.toml"
BACKUP_FILE=""
FORCE=false
DRY_RUN=false
CREATE_BACKUP=true
VERIFY=true
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
  echo -e "${BLUE}[INFO]${NC} $*" >&2
}

log_success() {
  echo -e "${GREEN}[SUCCESS]${NC} $*" >&2
}

log_error() {
  echo -e "${RED}[ERROR]${NC} $*" >&2
}

log_warning() {
  echo -e "${YELLOW}[WARNING]${NC} $*" >&2
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
      -f|--force)
        FORCE=true
        shift
        ;;
      -n|--dry-run)
        DRY_RUN=true
        shift
        ;;
      -b|--backup)
        CREATE_BACKUP=true
        shift
        ;;
      --no-backup)
        CREATE_BACKUP=false
        shift
        ;;
      -v|--verify)
        VERIFY=true
        shift
        ;;
      -h|--help)
        show_help
        exit 0
        ;;
      -*)
        log_error "Unknown option: $1"
        show_help
        exit 1
        ;;
      *)
        BACKUP_FILE="$1"
        shift
        ;;
    esac
  done

  if [ -z "$BACKUP_FILE" ]; then
    log_error "No backup file specified"
    show_help
    exit 1
  fi
}

# Extract database name from wrangler config
get_database_name() {
  local config_file="$1"

  if [ ! -f "$config_file" ]; then
    log_error "Config file not found: $config_file"
    exit 1
  fi

  local db_name=$(grep -A 2 '\[\[d1_databases\]\]' "$config_file" | grep 'binding' | head -1 | cut -d'"' -f2)

  if [ -z "$db_name" ]; then
    log_error "Could not find database binding in $config_file"
    exit 1
  fi

  echo "$db_name"
}

# Get row counts for verification
get_row_counts() {
  local db_name="$1"
  local config_file="$2"

  # Get list of tables
  local tables=$(wrangler d1 execute "$db_name" --remote --config "$config_file" --command "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '_cf_%';" --json 2>/dev/null | jq -r '.[0].results[].name' 2>/dev/null || echo "")

  local counts=""

  if [ -n "$tables" ]; then
    while IFS= read -r table; do
      if [ -n "$table" ]; then
        local count=$(wrangler d1 execute "$db_name" --remote --config "$config_file" --command "SELECT COUNT(*) as count FROM $table;" --json 2>/dev/null | jq -r '.[0].results[0].count' 2>/dev/null || echo "0")
        counts+="$table:$count "
      fi
    done <<< "$tables"
  fi

  echo "$counts"
}

# Decompress file if gzipped
decompress_if_needed() {
  local file="$1"

  if [[ "$file" == *.gz ]]; then
    log_info "Decompressing backup file..."
    local decompressed="${file%.gz}"

    gunzip -c "$file" > "$decompressed" || {
      log_error "Decompression failed"
      exit 1
    }

    echo "$decompressed"
  else
    echo "$file"
  fi
}

# Preview backup contents
preview_backup() {
  local file="$1"

  log_info "Previewing backup contents..."

  # Show first few lines of SQL
  echo ""
  echo "First 20 lines of backup:"
  echo "========================"
  head -20 "$file"
  echo "========================"
  echo ""

  # Count statements
  local create_count=$(grep -c "CREATE TABLE" "$file" || echo "0")
  local insert_count=$(grep -c "INSERT INTO" "$file" || echo "0")

  log_info "Backup contains:"
  log_info "  - CREATE TABLE statements: $create_count"
  log_info "  - INSERT INTO statements: $insert_count"
}

# Create safety backup
create_safety_backup() {
  local config_file="$1"

  log_info "Creating safety backup before restore..."

  local timestamp=$(date +%Y%m%d_%H%M%S)
  local safety_backup="safety_backup_${timestamp}.sql"

  if [ -f "$SCRIPT_DIR/backup.sh" ]; then
    "$SCRIPT_DIR/backup.sh" -c "$config_file" -o "$safety_backup" > /dev/null || {
      log_error "Safety backup failed"
      exit 1
    }
    log_success "Safety backup created: $safety_backup"
  else
    log_warning "Backup script not found, skipping safety backup"
  fi
}

# Confirm restoration
confirm_restore() {
  local db_name="$1"
  local backup_file="$2"

  if [ "$FORCE" = true ]; then
    return 0
  fi

  log_warning "This will restore database '$db_name' from: $backup_file"
  log_warning "ALL EXISTING DATA WILL BE REPLACED!"
  echo ""
  read -p "Are you sure you want to continue? (yes/no): " confirmation

  if [ "$confirmation" != "yes" ]; then
    log_info "Restore cancelled by user"
    exit 0
  fi
}

# Drop all existing tables (except system tables)
drop_all_tables() {
  local config_file="$1"
  local db_name=$(get_database_name "$config_file")

  log_info "Dropping all existing tables in database '$db_name'..."

  # Get list of user tables (exclude system tables) using JSON output for reliable parsing
  local tables_output=$(wrangler d1 execute "$db_name" --remote --config "$config_file" --command "
    SELECT name FROM sqlite_master 
    WHERE type='table' 
    AND name NOT IN ('_cf_KV', 'sqlite_sequence')
    ORDER BY name;
  " --json 2>/dev/null)

  # Extract table names from JSON using jq
  local tables=$(echo "$tables_output" | jq -r '.[0].results[].name' 2>/dev/null || true)

  if [ -n "$tables" ]; then
    log_info "Found tables to drop: $(echo "$tables" | tr '\n' ' ')"
    
    # Define table drop order to avoid foreign key constraints
    # Drop tables with foreign keys first, then tables they reference
    local drop_order=(
      "analytics_events"
      "link_tags"
      "link_reports"
      "monthly_counters"
      "destination_blacklist"
      "links"
      "users"
      "organizations"
      "settings"
      "d1_migrations"
    )
    
    # Generate DROP statements in the correct order
    local drop_statements=""
    for table in "${drop_order[@]}"; do
      if echo "$tables" | grep -q "^$table$"; then
        drop_statements="${drop_statements}DROP TABLE IF EXISTS \`$table\`; "
      fi
    done

    # Add any remaining tables that weren't in our predefined order
    while IFS= read -r table; do
      if [ -n "$table" ] && ! echo "${drop_order[@]}" | grep -q "$table"; then
        drop_statements="${drop_statements}DROP TABLE IF EXISTS \`$table\`; "
      fi
    done <<< "$tables"

    if [ -n "$drop_statements" ]; then
      log_info "Executing drop statements in correct order..."
      wrangler d1 execute "$db_name" --remote --config "$config_file" --command "$drop_statements" || {
        log_error "Failed to drop existing tables"
        return 1
      }
      log_success "All existing tables dropped"
    else
      log_info "No user tables found to drop"
    fi
  else
    log_info "No tables found in database"
  fi
}

# Restore database
restore_database() {
  local config_file="$1"
  local backup_file="$2"
  local db_name=$(get_database_name "$config_file")

  log_info "Restoring database '$db_name' from: $backup_file"

  # Drop all existing tables first
  drop_all_tables "$config_file" || {
    log_error "Failed to drop existing tables"
    return 1
  }

  # Execute SQL file
  wrangler d1 execute "$db_name" --remote --config "$config_file" --file "$backup_file" || {
    log_error "Database restore failed"
    log_error "The database may be in an inconsistent state"
    log_error "Consider restoring from the safety backup"
    return 1
  }

  log_success "Database restored successfully"
}

# Verify restoration
verify_restore() {
  local config_file="$1"
  local before_counts="$2"
  local db_name=$(get_database_name "$config_file")

  log_info "Verifying restoration..."

  local after_counts=$(get_row_counts "$db_name" "$config_file")

  echo ""
  log_info "Row count comparison:"
  log_info "Before restore: $before_counts"
  log_info "After restore:  $after_counts"
  echo ""

  if [ "$before_counts" = "$after_counts" ]; then
    log_warning "Row counts unchanged - this may indicate the restore didn't work as expected"
  else
    log_success "Row counts changed - restoration appears successful"
  fi
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

  # Check if backup file exists
  if [ ! -f "$BACKUP_FILE" ]; then
    log_error "Backup file not found: $BACKUP_FILE"
    exit 1
  fi

  # Check if config file exists
  if [ ! -f "$CONFIG_FILE" ]; then
    log_error "Config file not found: $CONFIG_FILE"
    exit 1
  fi

  # Decompress if needed
  local restore_file=$(decompress_if_needed "$BACKUP_FILE")

  # Get database name
  local db_name=$(get_database_name "$CONFIG_FILE")

  # Dry run mode
  if [ "$DRY_RUN" = true ]; then
    log_info "DRY RUN MODE - No changes will be made"
    log_info "Would restore to database: $db_name"
    log_info "From backup file: $restore_file"
    preview_backup "$restore_file"
    exit 0
  fi

  # Get current row counts for verification
  local before_counts=""
  if [ "$VERIFY" = true ]; then
    before_counts=$(get_row_counts "$db_name" "$CONFIG_FILE")
  fi

  # Confirm restoration
  confirm_restore "$db_name" "$restore_file"

  # Create safety backup
  if [ "$CREATE_BACKUP" = true ]; then
    create_safety_backup "$CONFIG_FILE"
  fi

  # Restore database
  restore_database "$CONFIG_FILE" "$restore_file"

  # Verify restoration
  if [ "$VERIFY" = true ]; then
    verify_restore "$CONFIG_FILE" "$before_counts"
  fi

  log_success "Restore completed successfully"
}

# Run main function
main "$@"
