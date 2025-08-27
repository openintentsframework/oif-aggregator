# OIF Order Signer

This script generates signed order payloads for the OIF (Open Intent Framework) solver.

## Usage

```bash
./order-signer.sh <config.json> [options]
```

### Options

- `--help` - Show help

### Examples

```bash
# Generate payload for manual submission
./order-signer.sh config.json

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
  "quote": {
    "digest": "0xb1ed9ae12486e22b734481a09975e08467a2dea499bd1e3415518b85de46eb11",
    "nonce": "1755778387750",
    "deadline": "1755778687",
    "expiry": "1755778687",
    "oracle_address": "0xdc64a140aa3e981100a9beca4e685f962f0cf6c9",
    "origin_chain_id": "31337",
    "origin_token": "0x5fbdb2315678afecb367f032d93f642f64180aa3",
    "amount": "1000000000000000000",
    "dest_chain_id": "31338",
    "output_settler_bytes32": "0x000000000000000000000000cf7ed3acca5a467e9e704c703e8d87f634fb0fc9",
    "dest_token_bytes32": "0x0000000000000000000000005fbdb2315678afecb367f032d93f642f64180aa3",
    "recipient_bytes32": "0x0000000000000000000000003c44cdddb6a900fa2b585dd299e03d12fa4293bc"
  }
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
