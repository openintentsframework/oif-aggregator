/**
 * OIF Quote Signer - TypeScript/JavaScript implementation
 * 
 * This module provides EIP-712 signature generation for OIF (Open Intent Framework) quotes.
 * It supports multiple signature schemes: Permit2, EIP-3009, and Compact (TheCompact).
 * 
 * Based on the Rust implementation from:
 * - crates/solver-demo/src/core/signing.rs
 * - crates/solver-types/src/utils/eip712.rs
 * 
 * @version 1.0.0
 * @license MIT
 */

import {
  type Address,
  type Hex,
  type PrivateKeyAccount,
  type PublicClient,
  createPublicClient,
  http,
  encodeAbiParameters,
} from 'viem';
import { privateKeyToAccount } from 'viem/accounts';

// ============================================================================
// Type Definitions
// ============================================================================

/**
 * Signature type for EIP-712
 */
export type SignatureType = 'eip712';

/**
 * Order payload structure for EIP-712 signing
 */
export interface OrderPayload {
  signatureType: SignatureType;
  domain: {
    name: string;
    version?: string;
    chainId: string | number;
    verifyingContract: string;
  };
  primaryType: string;
  message: Record<string, any>;
  types?: Record<string, Array<{ name: string; type: string }>>;
}

/**
 * OIF Order union type
 */
export type OifOrder =
  | { type: 'oif-escrow-v0'; payload: OrderPayload }
  | { type: 'oif-resource-lock-v0'; payload: OrderPayload }
  | { type: 'oif-3009-v0'; payload: OrderPayload; metadata: Record<string, any> }
  | { type: 'oif-generic-v0'; payload: Record<string, any> };

/**
 * Quote structure from API response
 */
export interface Quote {
  order: OifOrder;
  failureHandling: string;
  partialFill: boolean;
  validUntil: number;
  eta?: number;
  quoteId: string;
  provider?: string;
  preview: Record<string, any>;
}

/**
 * Configuration for the quote signer
 */
export interface SignerConfig {
  /** RPC endpoint URL for fetching domain separators (optional) */
  rpcUrl?: string;
  /** Custom public client for provider operations (optional) */
  publicClient?: PublicClient;
}

// ============================================================================
// EIP-712 Type Definitions
// ============================================================================

/**
 * Permit2 batch witness transfer type definitions
 * Based on Uniswap Permit2 specification
 */
const PERMIT2_TYPES = {
  PermitBatchWitnessTransferFrom: [
    { name: 'permitted', type: 'TokenPermissions[]' },
    { name: 'spender', type: 'address' },
    { name: 'nonce', type: 'uint256' },
    { name: 'deadline', type: 'uint256' },
    { name: 'witness', type: 'Permit2Witness' },
  ],
  TokenPermissions: [
    { name: 'token', type: 'address' },
    { name: 'amount', type: 'uint256' },
  ],
  Permit2Witness: [
    { name: 'expires', type: 'uint32' },
    { name: 'inputOracle', type: 'address' },
    { name: 'outputs', type: 'MandateOutput[]' },
  ],
  MandateOutput: [
    { name: 'oracle', type: 'bytes32' },
    { name: 'settler', type: 'bytes32' },
    { name: 'chainId', type: 'uint256' },
    { name: 'token', type: 'bytes32' },
    { name: 'amount', type: 'uint256' },
    { name: 'recipient', type: 'bytes32' },
    { name: 'call', type: 'bytes' },
    { name: 'context', type: 'bytes' },
  ],
};

/**
 * EIP-3009 authorization type definitions
 * Used for TransferWithAuthorization and ReceiveWithAuthorization
 */
const EIP3009_TYPES = {
  TransferWithAuthorization: [
    { name: 'from', type: 'address' },
    { name: 'to', type: 'address' },
    { name: 'value', type: 'uint256' },
    { name: 'validAfter', type: 'uint256' },
    { name: 'validBefore', type: 'uint256' },
    { name: 'nonce', type: 'bytes32' },
  ],
  ReceiveWithAuthorization: [
    { name: 'from', type: 'address' },
    { name: 'to', type: 'address' },
    { name: 'value', type: 'uint256' },
    { name: 'validAfter', type: 'uint256' },
    { name: 'validBefore', type: 'uint256' },
    { name: 'nonce', type: 'bytes32' },
  ],
  CancelAuthorization: [
    { name: 'authorizer', type: 'address' },
    { name: 'nonce', type: 'bytes32' },
  ],
};

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * Extract and normalize domain from payload
 */
function extractDomain(payload: OrderPayload) {
  return {
    name: payload.domain.name,
    version: payload.domain.version,
    chainId: BigInt(payload.domain.chainId),
    verifyingContract: payload.domain.verifyingContract as Address,
  };
}

/**
 * Fetch domain separator from contract by calling DOMAIN_SEPARATOR()
 * This matches the Rust implementation in signing.rs:272-294
 */
async function fetchDomainSeparator(
  verifyingContract: Address,
  client: PublicClient
): Promise<Hex> {
  // DOMAIN_SEPARATOR() function selector: 0x3644e515
  const callData = '0x3644e515' as Hex;

  const result = await client.call({
    to: verifyingContract,
    data: callData,
  });

  if (!result.data) {
    throw new Error('Failed to fetch domain separator from contract');
  }

  if (result.data.length !== 66) {
    // 0x + 64 hex chars = 66
    throw new Error(`Invalid domain separator length: ${result.data.length}`);
  }

  return result.data;
}

/**
 * Get or build EIP-712 types from payload
 */
function getTypesForPayload(payload: OrderPayload): Record<string, Array<{ name: string; type: string }>> {
  // If types are provided in payload, use them
  if (payload.types) {
    return payload.types;
  }

  // Otherwise, construct based on primaryType
  switch (payload.primaryType) {
    case 'PermitBatchWitnessTransferFrom':
    case 'PermitWitnessTransferFrom':
      return PERMIT2_TYPES;

    case 'TransferWithAuthorization':
    case 'ReceiveWithAuthorization':
    case 'CancelAuthorization':
      return {
        [payload.primaryType]: EIP3009_TYPES[payload.primaryType as keyof typeof EIP3009_TYPES],
      };

    default:
      throw new Error(`Unknown primary type: ${payload.primaryType}. Please provide types in payload.`);
  }
}

// ============================================================================
// Signature Scheme Implementations
// ============================================================================

/**
 * Sign Permit2 order (oif-escrow-v0)
 * Implements Permit2 batch witness transfer signature with 0x00 prefix
 * 
 * Based on signing.rs:176-179 and eip712.rs:132-342
 */
async function signPermit2(payload: OrderPayload, account: PrivateKeyAccount): Promise<Hex> {
  const domain = extractDomain(payload);
  const types = getTypesForPayload(payload);

  // Sign using EIP-712
  const signature = await account.signTypedData({
    domain,
    types,
    primaryType: payload.primaryType,
    message: payload.message,
  });

  // Add Permit2 prefix (0x00) - matches signing.rs:220-223
  return `0x00${signature.slice(2)}` as Hex;
}

/**
 * Sign EIP-3009 order (oif-3009-v0)
 * Implements EIP-3009 authorization signature with 0x01 prefix
 * 
 * Supports TransferWithAuthorization and ReceiveWithAuthorization
 * Based on signing.rs:164-174 and eip712.rs:603-700
 */
async function signEip3009(
  payload: OrderPayload,
  metadata: Record<string, any>,
  account: PrivateKeyAccount,
  config?: SignerConfig
): Promise<Hex> {
  const domain = extractDomain(payload);

  // Check if we need to fetch domain separator from contract
  // This matches the logic in signing.rs:414-463
  if (!metadata.domain_separator && config?.publicClient) {
    const domainSeparator = await fetchDomainSeparator(
      domain.verifyingContract,
      config.publicClient
    );

    console.log('Fetched domain separator from contract:', domainSeparator);
  }

  const types = getTypesForPayload(payload);

  // Sign using EIP-712
  const signature = await account.signTypedData({
    domain,
    types,
    primaryType: payload.primaryType,
    message: payload.message,
  });

  // Add EIP-3009 prefix (0x01) - matches signing.rs:232-235
  return `0x01${signature.slice(2)}` as Hex;
}

/**
 * Sign Compact order (oif-resource-lock-v0)
 * Implements TheCompact signature with ABI-encoded tuple
 * 
 * Encodes signature as (bytes sponsorSig, bytes allocatorSig)
 * Based on signing.rs:180-183 and signing.rs:591-646
 */
async function signCompact(
  payload: OrderPayload,
  account: PrivateKeyAccount,
  config?: SignerConfig
): Promise<Hex> {
  const domain = extractDomain(payload);
  const types = getTypesForPayload(payload);

  // Fetch domain separator from TheCompact contract if provider available
  // This matches signing.rs:535-548
  if (config?.publicClient) {
    const domainSeparator = await fetchDomainSeparator(
      domain.verifyingContract,
      config.publicClient
    );

    console.log('Fetched domain separator from Compact contract:', domainSeparator);
  }

  // Sign using EIP-712
  const signature = await account.signTypedData({
    domain,
    types,
    primaryType: payload.primaryType,
    message: payload.message,
  });

  // Compact signatures require special ABI encoding: (bytes, bytes)
  // sponsorSig = actual signature, allocatorSig = empty for basic flows
  // This matches signing.rs:591-646
  const sponsorSig = signature;
  const allocatorSig = '0x' as Hex;

  // ABI encode as tuple: (bytes sponsorSig, bytes allocatorSig)
  const encoded = encodeAbiParameters(
    [{ type: 'bytes' }, { type: 'bytes' }],
    [sponsorSig, allocatorSig]
  );

  return encoded;
}

/**
 * Sign generic EIP-712 order
 * For orders that don't require special signature encoding
 */
async function signGenericEip712(payload: OrderPayload, account: PrivateKeyAccount): Promise<Hex> {
  const domain = extractDomain(payload);
  const types = getTypesForPayload(payload);

  return await account.signTypedData({
    domain,
    types,
    primaryType: payload.primaryType,
    message: payload.message,
  });
}

// ============================================================================
// Main API
// ============================================================================

/**
 * Sign a quote with the appropriate signature scheme
 * 
 * This is the main entry point that routes to the correct signing function
 * based on the order type. Matches the logic in signing.rs:103-132
 * 
 * @param quote - Quote object from API response
 * @param privateKey - Private key hex string (with or without 0x prefix)
 * @param config - Optional configuration for RPC provider
 * @returns Hex-encoded signature with scheme-specific encoding
 * 
 * @example
 * ```typescript
 * const signature = await signQuote(quote, '0x123...', {
 *   rpcUrl: 'https://mainnet.infura.io/v3/YOUR_KEY'
 * });
 * ```
 */
export async function signQuote(
  quote: Quote,
  privateKey: Hex,
  config?: SignerConfig
): Promise<Hex> {
  // Parse private key into account
  const account = privateKeyToAccount(privateKey);

  // Setup public client if RPC URL provided and not already configured
  let effectiveConfig = config;
  if (config?.rpcUrl && !config.publicClient) {
    effectiveConfig = {
      ...config,
      publicClient: createPublicClient({
        transport: http(config.rpcUrl),
      }),
    };
  }

  // Route to appropriate signing function based on order type
  // This matches the switch statement in signing.rs:115-131
  switch (quote.order.type) {
    case 'oif-escrow-v0':
      return signPermit2(quote.order.payload, account);

    case 'oif-resource-lock-v0':
      return signCompact(quote.order.payload, account, effectiveConfig);

    case 'oif-3009-v0':
      return signEip3009(quote.order.payload, quote.order.metadata, account, effectiveConfig);

    case 'oif-generic-v0':
      throw new Error('Generic orders (oif-generic-v0) are not supported for signing');

    default:
      throw new Error(`Unsupported order type: ${(quote.order as any).type}`);
  }
}

/**
 * Sign an order payload directly without a full quote object
 * Useful for testing or when you have the payload structure already
 * 
 * @param orderType - Type of order ('oif-escrow-v0', 'oif-resource-lock-v0', 'oif-3009-v0')
 * @param payload - Order payload with EIP-712 structure
 * @param privateKey - Private key hex string
 * @param metadata - Optional metadata (required for EIP-3009)
 * @param config - Optional configuration for RPC provider
 */
export async function signOrderPayload(
  orderType: string,
  payload: OrderPayload,
  privateKey: Hex,
  metadata?: Record<string, any>,
  config?: SignerConfig
): Promise<Hex> {
  const account = privateKeyToAccount(privateKey);

  // Setup public client if needed
  let effectiveConfig = config;
  if (config?.rpcUrl && !config.publicClient) {
    effectiveConfig = {
      ...config,
      publicClient: createPublicClient({
        transport: http(config.rpcUrl),
      }),
    };
  }

  switch (orderType) {
    case 'oif-escrow-v0':
      return signPermit2(payload, account);

    case 'oif-resource-lock-v0':
      return signCompact(payload, account, effectiveConfig);

    case 'oif-3009-v0':
      if (!metadata) {
        throw new Error('Metadata is required for EIP-3009 orders');
      }
      return signEip3009(payload, metadata, account, effectiveConfig);

    default:
      throw new Error(`Unsupported order type: ${orderType}`);
  }
}

/**
 * Get the signer address from a private key
 * Useful for verifying which address will sign the quote
 * 
 * @param privateKey - Private key hex string
 * @returns Ethereum address derived from the private key
 */
export function getSignerAddress(privateKey: Hex): Address {
  const account = privateKeyToAccount(privateKey);
  return account.address;
}

// ============================================================================
// Additional Exports
// ============================================================================

export type { Address, Hex };

export {
  signPermit2,
  signEip3009,
  signCompact,
  signGenericEip712,
};

