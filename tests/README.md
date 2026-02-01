# Integration Tests

This directory contains integration tests for the Rushomon URL shortener API.

## Test Structure

- `common/mod.rs` - Shared utilities and helper functions
- `links_test.rs` - Link CRUD operations tests
- `redirect_test.rs` - URL redirection functionality tests
- `validation_test.rs` - Input validation tests (URLs and short codes)
- `run_integration_tests.sh` - Test runner script

## Running the Tests

### Prerequisites

1. Start the Wrangler dev server:
   ```bash
   wrangler dev --port 8787
   ```

2. Ensure D1 migrations are applied locally:
   ```bash
   wrangler d1 migrations apply rurl_db --local
   ```

### Run All Tests

Using the provided script:
```bash
./tests/run_integration_tests.sh
```

Or directly with cargo:
```bash
cargo test --test '*'
```

### Run Specific Test Files

```bash
# Link CRUD tests
cargo test --test links_test

# Redirect tests
cargo test --test redirect_test

# Validation tests
cargo test --test validation_test
```

### Run Specific Tests

```bash
# Run a single test by name
cargo test --test links_test test_create_link_with_random_short_code

# Run with output
cargo test --test links_test -- --nocapture
```

## Test Coverage

### Link CRUD Tests (links_test.rs)
- ✅ Create link with random short code
- ✅ Create link with custom short code
- ✅ Duplicate short code handling (409 Conflict)
- ✅ List all links
- ✅ Get single link by ID
- ✅ Delete link (soft delete)

### Redirect Tests (redirect_test.rs)
- ✅ Redirect with HTTP 301 status
- ✅ Non-existent short code returns 404
- ✅ Click count increments on redirect
- ✅ Inactive (deleted) links return 404

### Validation Tests (validation_test.rs)
- ✅ Reject dangerous URL schemes (javascript:, file:, data:)
- ✅ Reject malformed URLs
- ✅ Accept valid HTTP/HTTPS URLs
- ✅ Reject short codes that are too short (< 4 chars)
- ✅ Reject short codes that are too long (> 10 chars)
- ✅ Reject short codes with special characters
- ✅ Reject reserved words (api, auth, etc.)
- ✅ Reserved word check is case-insensitive
- ✅ Accept valid alphanumeric short codes

## Notes

- Tests run sequentially (`--test-threads=1`) to avoid race conditions with the shared dev server
- Some tests have slight delays (`tokio::time::sleep`) to allow async operations to complete
- The dev server must be running on port 8787 before executing tests
- Tests create real data in the local D1 database

### Async Analytics Testing

The redirect handler logs analytics in the background using `spawn_local`, which means:
- Analytics operations don't block the redirect response
- Tests must poll/wait for background operations to complete
- `test_redirect_increments_click_count` uses polling with a 2-second timeout
- If this test fails, the background analytics may be taking longer than expected

## Troubleshooting

**Tests fail with connection errors:**
- Verify `wrangler dev` is running on port 8787
- Check that the dev server started successfully

**Tests fail with database errors:**
- Ensure migrations are applied: `wrangler d1 migrations apply rurl_db --local`
- Try restarting the dev server

**Compilation errors:**
- Run `cargo build` to check for syntax errors
- Ensure all dependencies are installed: `cargo fetch`
