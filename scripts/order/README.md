# OIF Order Signer

This script generates signed order payloads for the OIF (Open Intent Framework) solver.

## Usage

```bash
./generate-order-request.sh <config.json> [options]
```

### Options

- `--help` - Show help

### Examples

```bash
# Generate payload for manual submission
./generate-order-request.sh config.json

```

## Config File Format

**⚠️ Important**: This script requires a **different config file** than the main aggregator's `config/config.json`.

The order signer needs a config with account and quote data:

```json
{
  "account": {
    "address": "0x70997970C51812dc3A010C7d01b50e0d17dc79C8",
    "private_key": "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d"
  },
  "quote": {}
}
```

See `example_config.json` for a complete working example.

## Dependencies

- `jq` - JSON processing
- `cast` - From Foundry toolkit (for signing and ABI encoding)
- `curl` - For HTTP requests (only if using `--submit-to`)

## Config vs Aggregator Config

| File | Purpose | Structure |
|------|---------|-----------|
| `config/config.json` | **Aggregator server config** | Server, solvers, aggregation settings |
| `scripts/order/config.json` | **Order signer config** | Account and quote data for signing |

**Don't mix these up!** The order signer script cannot use the main aggregator config file.
