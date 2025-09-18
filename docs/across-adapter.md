# Across Adapter Guide

The Across Adapter provides seamless integration with the Across Protocol, enabling cross-chain bridge operations with advanced configuration options through request metadata.

## 🎯 Overview

The Across Adapter (`across-v1`) implements the Across Protocol API and provides:

- **Cross-Chain Bridge Quotes** - Get quotes for bridging tokens across different chains
- **Dynamic Query Parameters** - Flexible configuration through `requestParams` metadata
- **HTTP Client Caching** - Optimized connection pooling and keep-alive
- **Robust Error Handling** - Comprehensive validation and error reporting

> **📋 Note**: Order submission and tracking are not currently supported by this adapter. These features will be added in future releases via the official Across SDK integration.

## 📋 Basic Configuration

### Minimal Setup

```json
{
  "solvers": {
    "across-solver": {
      "solver_id": "across-solver",
      "adapter_id": "across-v1", 
      "endpoint": "https://app.across.to/api",
      "enabled": true,
      "name": "Across Protocol",
      "description": "Across Protocol"
    }
  }
}
```

## 🔧 Request Parameters

The Across Adapter supports extensive customization through the `metadata.requestParams` field in your `GetQuoteRequest`. These parameters are passed directly to the Across Protocol API.

### Basic Request with Parameters

```json
{
  "user": "",
  "available_inputs": [...],
  "requested_outputs": [...],
  "metadata": {
    "requestParams": {
      "tradeType": "exactInput",
      "slippage": 1,
      "integratorId": "my-dapp"
    }
  }
}
```

> **💡 Note**: Numeric parameters like `slippage` and `appFee` accept both numbers (`5`) and strings (`"5"`) for flexibility.

### All Available Request Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `tradeType` | `string` | Trade type: `"exactInput"`, `"exactOutput"`, or `"minOutput"` |
| `integratorId` | `string` | 2-byte hex-string that identifies the integrator. E.g., "0xdead". |
| `refundAddress` | `string` | Address to receive refunds. |
| `refundOnOrigin` | `boolean` | Specifies whether refund should be sent on the origin chain. |
| `slippage` | `number` | Slippage tolerance percentage (e.g., 1 for 1%, 0.5 for 0.5%) |
| `skipOriginTxEstimation` | `boolean` | Used to define whether you want to calculate the transaction details (swap) on origin chain. |
| `excludeSources` | `string` | Comma-separated list of sources to exclude |
| `includeSources` | `string` | Comma-separated list of sources to include (exclusive with `excludeSources`) |
| `appFee` | `string` | App fee percentage (e.g., `"0.001"` for 0.1%) |
| `appFeeRecipient` | `string` | Address to receive app fees |
| `strictTradeType` | `boolean` | Used to define whether you want to strictly follow the defined tradeType |

> **📖 Complete API Reference**: For detailed parameter documentation, default values, max values, and additional options, see the [official Across API documentation](https://docs.across.to/reference/api-reference#get-swap-approval).


## 🔗 Additional Resources

### Across Protocol Documentation

- **[Across Protocol Docs](https://docs.across.to/)** - Official documentation
- **[API Reference](https://docs.across.to/reference/api-reference)** - Complete API specification
- **[SDK Documentation](https://docs.across.to/reference/app-sdk-reference)** - Official TypeScript SDK

### OIF Aggregator Documentation

- **[Custom Adapter Guide](custom-adapters.md)** - Building your own adapters
- **[OIF Adapter Guide](oif-adapter.md)** - JWT authentication and OIF protocol integration
- **[Configuration Guide](configuration.md)** - Complete configuration reference  
- **[API Documentation](api/)** - OIF protocol specification
- **[Quotes and Aggregation](quotes-and-aggregation.md)** - Understanding quote aggregation

## 🛠️ Troubleshooting

### Common Issues

**Invalid Response Error**
```
Failed to parse Across swap response
```
- Enable debug logging to see full API response
- Verify your chain IDs and token addresses are valid
- Check that tokens are supported on both chains

**Missing Request Parameters**  
```
Using default query parameters (no requestParams in metadata)
```
- Add `requestParams` object to your request metadata
- Verify parameter names match the documented format

### Debug Mode

Enable debug logging to see detailed request/response information:

```bash
RUST_LOG=debug cargo run
```

This will show:
- Built query parameters
- Full API response bodies
- Client cache statistics  
- Error details with context

---

**Source Code**: Check `crates/adapters/src/across_adapter.rs` for the complete implementation.
