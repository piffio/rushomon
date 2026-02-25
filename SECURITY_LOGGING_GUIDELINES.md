# Secure Logging Guidelines

## Overview

This document outlines the secure logging practices that must be followed when adding log statements to the Rushomon codebase.

## 🚨 Rule: Never Log Sensitive User Data Directly

**Sensitive information includes:**
- Email addresses
- User IDs (sub, id fields)
- Personal names
- IP addresses
- Authentication tokens
- Session identifiers
- Any personally identifiable information (PII)

## ✅ Approved Approach: Hash Sensitive Data

When logging is necessary for debugging, **always hash sensitive information** using SHA256:

```rust
use sha2::{Digest, Sha256};

// ✅ CORRECT: Hash sensitive information
let user_id_hash = {
    let mut hasher = Sha256::new();
    hasher.update(user.id.to_string().as_bytes());
    format!("{:x}", hasher.finalize())
};

let email_hash = user.email.as_ref().map(|email| {
    let mut hasher = Sha256::new();
    hasher.update(email.as_bytes());
    format!("{:x}", hasher.finalize())
});

println!(
    "User action: id_hash={}, email_hash={:?}",
    user_id_hash, email_hash
);
```

## ❌ Forbidden: Direct Logging of Sensitive Data

```rust
// ❌ FORBIDDEN: Never log sensitive data directly
println!(
    "User action: id={}, email={:?}",
    user.id, user.email  // This exposes sensitive information
);
```

## 📋 Implementation Checklist

When adding log statements that might involve user data:

1. **Identify Sensitive Data**: Review all variables being logged
2. **Hash When Necessary**: Apply SHA256 hashing to any sensitive fields
3. **Use Generic Messages**: When possible, log high-level events without specific data
4. **Code Review**: All logging changes must be reviewed for security compliance

## 🔍 Examples by Context

### Authentication Events
```rust
// ✅ Good: Hash user identifiers
println!(
    "Auth attempt: user_hash={}, provider={}",
    hash_user_id(&user_id), provider
);

// ❌ Bad: Expose user identifiers
println!("Auth attempt: user_id={}, provider={}", user_id, provider);
```

### API Requests
```rust
// ✅ Good: Generic success/failure messages
println!("API request processed: status={}, endpoint={}", status, endpoint);

// ❌ Bad: Include user-specific data
println!("API request: user_email={}, status={}", email, status);
```

### Error Handling
```rust
// ✅ Good: Hash context identifiers
println!(
    "Error in user context: context_hash={}, error_type={}",
    hash_context(&context), error_type
);

// ❌ Bad: Include user context directly
println!("Error for user {}: {}", user.email, error);
```

## 🛠️ Helper Functions

Create reusable hashing utilities:

```rust
pub fn hash_user_id(id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(id.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn hash_email(email: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(email.as_bytes());
    format!("{:x}", hasher.finalize())
}
```

## 🔧 Enforcement

### Automated Checks
- **Clippy Rules**: Configured in `Cargo.toml` with strict linting
- **Security Review**: All PRs must pass security review for logging practices

### Manual Review
- **Code Review**: Reviewers must check for sensitive data exposure
- **Security Audit**: Regular audits of logging statements

## 📝 When in Doubt

If you're unsure whether data is sensitive:
1. **Assume it's sensitive** and hash it
2. **Ask for review** before committing
3. **Use generic messages** when possible

## 🚀 Benefits

- **User Privacy**: Protects user information from logs
- **Security**: Reduces risk of data exposure
- **Compliance**: Helps meet privacy regulations (GDPR, CCPA)
- **Debugging**: Still provides useful debugging information without exposing data

---

**Remember: Hash it, don't expose it!** 🔐
