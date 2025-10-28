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
 * @version 1.1.0
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
  keccak256,
  concat,
  toHex,
  encodeAbiParameters as encode,
  hexToBytes,
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
  /** Wallet signing function for EIP-712 (optional - if not provided, uses private key) */
  walletSignTypedData?: (args: { domain: any; types: any; primaryType: string; message: any }) => Promise<Hex>;
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
 * Extract and normalize domain from payload for viem signing
 * Only includes fields that are defined to avoid viem encoding issues
 */
function extractDomain(payload: OrderPayload) {
  const domain: any = {
    name: payload.domain.name,
    chainId: BigInt(payload.domain.chainId),
    verifyingContract: payload.domain.verifyingContract as Address,
  };
  
  // Only include version if it exists in the payload
  if (payload.domain.version !== undefined && payload.domain.version !== null) {
    domain.version = payload.domain.version;
  }
  
  return domain;
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

/**
 * Compute EIP-712 domain separator hash
 */
function computeDomainSeparator(domain: {
  name: string;
  version?: string;
  chainId: bigint;
  verifyingContract: Address;
}): Hex {
  const domainTypeHash = keccak256(
    toHex('EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)')
  );

  const encoded = encode(
    [
      { type: 'bytes32' },
      { type: 'bytes32' },
      { type: 'bytes32' },
      { type: 'uint256' },
      { type: 'address' },
    ],
    [
      domainTypeHash,
      keccak256(toHex(domain.name)),
      keccak256(toHex(domain.version || '1')),
      domain.chainId,
      domain.verifyingContract,
    ]
  );

  return keccak256(encoded);
}

/**
 * Compute EIP-712 digest from domain separator and struct hash
 */
function computeEip712Digest(domainSeparator: Hex, structHash: Hex): Hex {
  // EIP-712 encoding: 0x1901 || domainSeparator || structHash
  const digestInput = concat([
    '0x1901' as Hex,
    domainSeparator,
    structHash,
  ]);

  return keccak256(digestInput);
}

/**
 * Reconstruct EIP-3009 digest
 */
async function reconstructEip3009Digest(
  payload: OrderPayload,
  metadata: Record<string, any>,
  client?: PublicClient
): Promise<Hex> {
  const domain = extractDomain(payload);

  // Try to get domain separator from metadata first
  let domainSeparator: Hex;

  if (metadata.domain_separator) {
    // Use domain separator from metadata
    domainSeparator = metadata.domain_separator.startsWith('0x')
      ? metadata.domain_separator
      : `0x${metadata.domain_separator}`;

    if (hexToBytes(domainSeparator).length !== 32) {
      throw new Error(
        `Invalid domain separator length in metadata: expected 32 bytes, got ${hexToBytes(domainSeparator).length}`
      );
    }
  } else if (client) {
    // Fetch domain separator from token contract
    domainSeparator = await fetchDomainSeparator(domain.verifyingContract, client);
  } else {
    // Fallback: compute domain separator (may not match contract's actual value!)
    domainSeparator = computeDomainSeparator(domain);
  }

  // Get the struct hash for the message
  const types = getTypesForPayload(payload);
  const primaryTypeFields = types[payload.primaryType];

  if (!primaryTypeFields) {
    throw new Error(`Missing type definition for ${payload.primaryType}`);
  }

  // Compute type hash
  const typeString = `${payload.primaryType}(${primaryTypeFields
    .map((f) => `${f.type} ${f.name}`)
    .join(',')})`;
  const typeHash = keccak256(toHex(typeString));

  // Encode the struct fields
  const fieldTypes = primaryTypeFields.map((f) => ({ type: f.type }));
  const fieldValues = primaryTypeFields.map((f) => payload.message[f.name]);

  const structEncoded = encode(
    [{ type: 'bytes32' }, ...fieldTypes],
    [typeHash, ...fieldValues]
  );

  const structHash = keccak256(structEncoded);

  // Compute final EIP-712 digest
  return computeEip712Digest(domainSeparator, structHash);
}

/**
 * Reconstruct Compact digest
 */
async function reconstructCompactDigest(
  payload: OrderPayload,
  client?: PublicClient
): Promise<Hex> {
  const domain = extractDomain(payload);

  let domainSeparator: Hex;

  if (client) {
    domainSeparator = await fetchDomainSeparator(domain.verifyingContract, client);
  } else {
    // Fallback: compute domain separator
    domainSeparator = computeDomainSeparator(domain);
  }

  // Get the struct hash for the message
  const types = getTypesForPayload(payload);
  const primaryTypeFields = types[payload.primaryType];

  if (!primaryTypeFields) {
    throw new Error(`Missing type definition for ${payload.primaryType}`);
  }

  // Compute type hash
  const typeString = `${payload.primaryType}(${primaryTypeFields
    .map((f) => `${f.type} ${f.name}`)
    .join(',')})`;
  const typeHash = keccak256(toHex(typeString));

  // Encode the struct fields
  const fieldTypes = primaryTypeFields.map((f) => ({ type: f.type }));
  const fieldValues = primaryTypeFields.map((f) => payload.message[f.name]);

  const structEncoded = encode(
    [{ type: 'bytes32' }, ...fieldTypes],
    [typeHash, ...fieldValues]
  );

  const structHash = keccak256(structEncoded);

  // Compute final EIP-712 digest
  return computeEip712Digest(domainSeparator, structHash);
}

// ============================================================================
// Signature Scheme Implementations
// ============================================================================

/**
 * Sign Permit2 order (oif-escrow-v0)
 * Implements Permit2 batch witness transfer signature with 0x00 prefix
 */
async function signPermit2(
  payload: OrderPayload, 
  account?: PrivateKeyAccount,
  walletSignFn?: (args: { domain: any; types: any; primaryType: string; message: any }) => Promise<Hex>
): Promise<Hex> {
  const domain = extractDomain(payload);
  const types = getTypesForPayload(payload);

  // Sign using wallet or private key
  const signature = walletSignFn
    ? await walletSignFn({ domain, types, primaryType: payload.primaryType, message: payload.message })
    : await account!.signTypedData({ domain, types, primaryType: payload.primaryType, message: payload.message });

  // Add Permit2 prefix (0x00)
  const prefixed = `0x00${signature.slice(2)}` as Hex;

  return prefixed;
}

/**
 * Sign EIP-3009 order (oif-3009-v0)
 * Implements EIP-3009 authorization signature with 0x01 prefix
 * 
 * Supports TransferWithAuthorization and ReceiveWithAuthorization
 */
async function signEip3009(
  payload: OrderPayload,
  metadata: Record<string, any>,
  account?: PrivateKeyAccount,
  config?: SignerConfig,
  walletSignFn?: (args: { domain: any; types: any; primaryType: string; message: any }) => Promise<Hex>
): Promise<Hex> {
  let signature: Hex;

  if (walletSignFn) {
    // Wallet signing: use signTypedData with cleaned types
    const payloadDomain = extractDomain(payload);
    let types = getTypesForPayload(payload);
    
    // Remove EIP712Domain from types if present
    if (types.EIP712Domain) {
      const { EIP712Domain, ...typesWithoutDomain } = types;
      types = typesWithoutDomain;
    }
    
    // Verify domain separator if possible
    if (metadata.domain_separator && config?.publicClient) {
      const payloadDomainSep = computeDomainSeparator(payloadDomain);
      const metadataDomainSep = metadata.domain_separator.startsWith('0x')
        ? metadata.domain_separator
        : `0x${metadata.domain_separator}`;
      
      if (payloadDomainSep !== metadataDomainSep) {
        throw new Error('EIP-3009 domain mismatch: Please use Private Key Override.');
      }
      
      // Verify against token contract
      const contractDomainSep = await fetchDomainSeparator(
        payloadDomain.verifyingContract as Address,
        config.publicClient
      );
      
      if (contractDomainSep !== payloadDomainSep) {
        throw new Error('EIP-3009 domain mismatch: Please use Private Key Override.');
      }
    }
    
    // Ensure domain has version field
    const domainForSigning = {
      ...payloadDomain,
      version: payloadDomain.version || '1',
    };
    
    signature = await walletSignFn({
      domain: domainForSigning,
      types,
      primaryType: payload.primaryType,
      message: payload.message,
    });
  } else {
    // Private key signing: reconstruct and sign digest
    const digest = await reconstructEip3009Digest(payload, metadata, config?.publicClient);
    signature = await account!.sign({ hash: digest });
  }

  // Add EIP-3009 prefix (0x01)
  const prefixed = `0x01${signature.slice(2)}` as Hex;

  const maybeInputs = metadata?.inputs ?? payload.message?.inputs ?? [];
  if (Array.isArray(maybeInputs) && maybeInputs.length > 1) {
    const encoded = encodeAbiParameters([{ type: 'bytes[]' }], [[prefixed]]);
    return encoded as Hex;
  }

  return prefixed;
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
  account?: PrivateKeyAccount,
  config?: SignerConfig,
  walletSignFn?: (args: { domain: any; types: any; primaryType: string; message: any }) => Promise<Hex>
): Promise<Hex> {
  let signature: Hex;

  if (walletSignFn) {
    // Wallet signing: use standard EIP-712
    const domain = extractDomain(payload);
    const types = getTypesForPayload(payload);
    
    signature = await walletSignFn({
      domain,
      types,
      primaryType: payload.primaryType,
      message: payload.message,
    });
  } else {
    // Private key signing: reconstruct and sign digest
    const digest = await reconstructCompactDigest(payload, config?.publicClient);
    signature = await account!.sign({ hash: digest });
  }

  // Compact signatures require special ABI encoding: (bytes, bytes)
  const sponsorSig = signature;
  const allocatorSig = '0x' as Hex;

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
  privateKey?: Hex,
  config?: SignerConfig
): Promise<Hex> {
  // Parse private key into account if provided
  const account = privateKey ? privateKeyToAccount(privateKey) : undefined;
  const walletSignFn = config?.walletSignTypedData;

  // Require either private key or wallet signing function
  if (!account && !walletSignFn) {
    throw new Error('Either privateKey or config.walletSignTypedData must be provided');
  }

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
  switch (quote.order.type) {
    case 'oif-escrow-v0':
      return signPermit2(quote.order.payload, account, walletSignFn);

    case 'oif-resource-lock-v0':
      return signCompact(quote.order.payload, account, effectiveConfig, walletSignFn);

    case 'oif-3009-v0':
      return signEip3009(quote.order.payload, quote.order.metadata, account, effectiveConfig, walletSignFn);

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
  reconstructEip3009Digest,
  reconstructCompactDigest,
  fetchDomainSeparator,
  computeDomainSeparator,
  computeEip712Digest,
};