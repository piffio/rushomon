# Testing Quick Start Guide

## TL;DR - Run Tests Now

```bash
# Terminal 1 - Start dev server
wrangler dev --port 8787

# Terminal 2 - Run all tests
./tests/run_integration_tests.sh
```

---

## What Tests Do We Have?

### Unit Tests (46 tests)
Already passing! Test core utilities and models.

```bash
cargo test --lib
```

### Integration Tests (25 tests) - NEW!
Test the actual API via HTTP requests.

```bash
cargo test --test '*'
```

---

## Integration Test Breakdown

### Link CRUD (7 tests)
- Create with random code
- Create with custom code
- Duplicate code handling
- List all links
- Get single link
- Delete link

### Redirects (4 tests)
- 301 redirect works
- 404 for non-existent codes
- Click count increments
- Deleted links return 404

### Validation (14 tests)
- URL security (XSS prevention)
- Short code validation
- Reserved word blocking

**Total: 25 integration tests**

---

## Quick Commands

```bash
# All tests (unit + integration)
cargo test

# Only unit tests
cargo test --lib

# Only integration tests
cargo test --test '*'

# Specific test file
cargo test --test links_test
cargo test --test redirect_test
cargo test --test validation_test

# Single test with output
cargo test --test links_test test_create_link_with_random_short_code -- --nocapture

# Watch mode (auto-run on code changes)
cargo watch -x test
```

---

## Expected Output

When all tests pass, you should see:

```
🚀 Starting Rushomon Integration Tests

✓ Dev server is running

Running integration tests...
running 25 tests
test test_accept_valid_alphanumeric_code ... ok
test test_accept_valid_http_url ... ok
test test_accept_valid_https_url ... ok
test test_create_duplicate_short_code_fails ... ok
test test_create_link_with_custom_short_code ... ok
test test_create_link_with_random_short_code ... ok
test test_delete_link ... ok
test test_get_link_by_id ... ok
test test_inactive_link_returns_404 ... ok
test test_list_links ... ok
test test_nonexistent_short_code_returns_404 ... ok
test test_redirect_increments_click_count ... ok
test test_redirect_with_301 ... ok
test test_reject_data_uri ... ok
test test_reject_file_url ... ok
test test_reject_javascript_url ... ok
test test_reject_malformed_url ... ok
test test_reject_reserved_word_api ... ok
test test_reject_reserved_word_auth ... ok
test test_reject_short_code_too_long ... ok
test test_reject_short_code_too_short ... ok
test test_reject_short_code_with_special_chars ... ok
test test_reject_short_code_with_underscore ... ok
test test_reserved_word_case_insensitive ... ok

test result: ok. 25 passed; 0 failed; 0 ignored

✨ All tests passed!
```

---

## Troubleshooting

**"Connection refused"**
→ Start wrangler dev first: `wrangler dev --port 8787`

**"Database errors"**
→ Apply migrations: `wrangler d1 migrations apply rurl_db --local`

**"Tests fail sporadically"**
→ Tests run sequentially to avoid race conditions. If still failing, restart dev server.

---

## What's Next?

Once all 25 integration tests pass:
✅ Phase 2 is complete and validated
→ Ready to move to Phase 3: Authentication System

Phase 3 will add:
- JWT session management
- GitHub OAuth login
- Authentication middleware
- Protected API routes

---

## Documentation

For detailed information:
- `tests/README.md` - Full test documentation
- `PHASE2.5_STATUS.md` - Implementation status and details
- `TEST_COVERAGE_REPORT.md` - Unit test coverage (Phase 1)
