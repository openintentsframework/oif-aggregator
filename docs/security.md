# Security Configuration

This document covers security configuration for the OIF Aggregator.

## Integrity Secret

The integrity secret is used to generate HMAC-SHA256 checksums for quotes and orders to ensure they haven't been tampered with.

**⚠️ REQUIRED CONFIGURATION**: This secret is mandatory and must be configured before starting the application. No default value is provided for security reasons.

### Configuration

#### Method 1: Environment Variable (Recommended for Production)
```bash
export INTEGRITY_SECRET="your-secure-random-key-here-minimum-32-characters"
```

#### Method 2: Configuration File

**Option A: Environment Variable Reference (Recommended)**
```json
{
  "security": {
    "integrity_secret": {
      "type": "env",
      "value": "INTEGRITY_SECRET"
    }
  }
}
```

**Option B: Plain Value (Development Only)**
```json
{
  "security": {
    "integrity_secret": {
      "type": "plain", 
      "value": "your-secure-random-key-here-minimum-32-characters"
    }
  }
}
```

### Generating a Secure Secret

Use a cryptographically secure random string generator:

```bash
# Using openssl (recommended)
openssl rand -hex 32

# Using /dev/urandom on Unix systems
head -c 32 /dev/urandom | base64

# Using Python
python3 -c "import secrets; print(secrets.token_hex(32))"
```

### Security Requirements

- **Length**: Minimum 32 characters (64 recommended)
- **Randomness**: Use a cryptographically secure random generator
- **Uniqueness**: Different secret per environment (dev/staging/prod)
- **Storage**: Store in environment variables, not in version control
- **Rotation**: Rotate secrets periodically

### Production Checklist

- [ ] Set `INTEGRITY_SECRET` environment variable
- [ ] Secret is at least 32 characters long
- [ ] Secret is randomly generated
- [ ] Secret is different from development
- [ ] Secret is not committed to version control
- [ ] Application starts without security warnings

### How It Works

1. **Quote Generation**: When the aggregator generates quotes, it creates an HMAC-SHA256 checksum of the quote data and includes it in the `integrity_checksum` field.

2. **Order Submission**: When orders are submitted, the system verifies the quote's integrity checksum to ensure the quote hasn't been tampered with.

3. **Verification**: If verification fails, the order is rejected with a `INTEGRITY_VERIFICATION_FAILED` error.

### Example Usage

**Environment Variable:**
```bash
# Set secure secret
export INTEGRITY_SECRET="7f3e8b2a9c1d4e6f8a5b2c9d1e4f7a8b3c6e9f2a5d8b1e4f7a2d5e8f1a4b7c"

# Start the server (config.json should reference env var)
cargo run --bin oif-aggregator
```

**Plain Value (Development):**
```bash
# No environment variable needed, secret is in config file
cargo run --bin oif-aggregator
```

If the integrity secret is not configured, the application will fail to start with a clear error message:
```
Failed to resolve integrity secret: Environment variable 'INTEGRITY_SECRET' not found. Please set the INTEGRITY_SECRET environment variable with a secure random string (minimum 32 characters).
```

This ensures that the application cannot run without a properly configured integrity secret, preventing security vulnerabilities.
