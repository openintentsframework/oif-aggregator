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
  toBytes,
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
  MandateOutput: [
    { name: 'oracle', type: 'bytes32' },
    { name: 'settler', type: 'bytes32' },
    { name: 'chainId', type: 'uint256' },
    { name: 'token', type: 'bytes32' },
    { name: 'amount', type: 'uint256' },
    { name: 'recipient', type: 'bytes32' },
    { name: 'call', type: 'bytes' }, // TODO: check if we need to change this to callbackData
    { name: 'context', type: 'bytes' },
  ],
  Permit2Witness: [
    { name: 'expires', type: 'uint32' },
    { name: 'inputOracle', type: 'address' },
    { name: 'outputs', type: 'MandateOutput[]' },
  ],
  TokenPermissions: [
    { name: 'token', type: 'address' },
    { name: 'amount', type: 'uint256' },
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

function normalizeTypedDataValue(value: any): any {
  if (Array.isArray(value)) {
    return value.map((item) => normalizeTypedDataValue(item));
  }

  if (value !== null && typeof value === 'object') {
    const normalized: Record<string, any> = {};
    for (const [key, nestedValue] of Object.entries(value)) {
      normalized[key] = normalizeTypedDataValue(nestedValue);
    }
    return normalized;
  }

  if (typeof value === 'string') {
    if (value.startsWith('0x') || value.startsWith('0X')) {
      return value as Hex;
    }

    if (/^\d+$/.test(value)) {
      try {
        return BigInt(value);
      } catch {
        return value;
      }
    }
  }

  return value;
}

function normalizeTypedDataMessage(message: Record<string, any>): Record<string, any> {
  return normalizeTypedDataValue(message);
}

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
  // Always use canonical type definitions based on primaryType to ensure consistent ordering
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
      // For unknown types, fall back to payload types if provided
      if (payload.types) {
        // Filter out EIP712Domain - wallets add this automatically
        const filteredTypes: Record<string, Array<{ name: string; type: string }>> = {};
        for (const [typeName, typeFields] of Object.entries(payload.types)) {
          if (typeName !== 'EIP712Domain') {
            filteredTypes[typeName] = typeFields;
          }
        }
        return filteredTypes;
      }
      
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

// ============================================================================
// Manual ABI Encoding Helpers (for Permit2 digest reconstruction)
// ============================================================================

/**
 * Encode a 32-byte value for EIP-712 struct hashing
 */
function encodeBytes32(value: Hex): Uint8Array {
  const bytes = hexToBytes(value);
  if (bytes.length !== 32) {
    throw new Error(`Expected 32 bytes, got ${bytes.length}`);
  }
  return bytes;
}

/**
 * Encode an address for EIP-712 struct hashing (right-aligned in 32 bytes)
 */
function encodeAddress(address: Address): Uint8Array {
  const result = new Uint8Array(32);
  const addressBytes = hexToBytes(address);
  // Address is right-aligned: 12 zero bytes + 20 address bytes
  result.set(addressBytes, 12);
  return result;
}

/**
 * Encode a uint256 for EIP-712 struct hashing (big-endian 32 bytes)
 */
function encodeUint256(value: bigint): Uint8Array {
  const result = new Uint8Array(32);
  let v = value;
  for (let i = 31; i >= 0; i--) {
    result[i] = Number(v & 0xFFn);
    v >>= 8n;
  }
  return result;
}

/**
 * Encode a uint32 for EIP-712 struct hashing (right-aligned in 32 bytes)
 */
function encodeUint32(value: number): Uint8Array {
  const result = new Uint8Array(32);
  // uint32 is right-aligned: 28 zero bytes + 4 value bytes
  result[28] = (value >> 24) & 0xFF;
  result[29] = (value >> 16) & 0xFF;
  result[30] = (value >> 8) & 0xFF;
  result[31] = value & 0xFF;
  return result;
}

/**
 * Concatenate multiple Uint8Arrays
 */
function concatBytes(...arrays: Uint8Array[]): Uint8Array {
  const totalLength = arrays.reduce((sum, arr) => sum + arr.length, 0);
  const result = new Uint8Array(totalLength);
  let offset = 0;
  for (const arr of arrays) {
    result.set(arr, offset);
    offset += arr.length;
  }
  return result;
}

/**
 * Parse a bytes32 hex string to Uint8Array
 */
function parseBytes32(hexStr: string): Uint8Array {
  const hex = hexStr.startsWith('0x') ? hexStr.slice(2) : hexStr;
  if (hex.length !== 64) {
    throw new Error(`Expected 64 hex chars for bytes32, got ${hex.length}`);
  }
  return hexToBytes(`0x${hex}` as Hex);
}

/**
 * Manually reconstruct Permit2 digest for exact byte-for-byte encoding
 * Based on: solver-types/src/utils/eip712.rs::reconstruct_permit2_digest
 */
function reconstructPermit2Digest(payload: OrderPayload): Hex {
  const domain = payload.domain as any;
  const message = payload.message as any;

  // 1. Compute domain hash
  const chainId = BigInt(domain.chainId);
  const name = domain.name;
  const verifyingContract = domain.verifyingContract as Address;

  const domainTypeHash = keccak256(toBytes('EIP712Domain(string name,uint256 chainId,address verifyingContract)'));
  const nameHash = keccak256(toBytes(name));

  const domainEncoded = concatBytes(
    encodeBytes32(domainTypeHash),
    encodeBytes32(nameHash),
    encodeUint256(chainId),
    encodeAddress(verifyingContract)
  );
  const domainHash = keccak256(domainEncoded);

  // 2. Build type hash for PermitBatchWitnessTransferFrom
  const permitType = 'PermitBatchWitnessTransferFrom(TokenPermissions[] permitted,address spender,uint256 nonce,uint256 deadline,Permit2Witness witness)MandateOutput(bytes32 oracle,bytes32 settler,uint256 chainId,bytes32 token,uint256 amount,bytes32 recipient,bytes call,bytes context)Permit2Witness(uint32 expires,address inputOracle,MandateOutput[] outputs)TokenPermissions(address token,uint256 amount)';
  const typeHash = keccak256(toBytes(permitType));

  // 3. Extract message fields
  const spender = message.spender as Address;
  const nonce = BigInt(message.nonce);
  const deadline = BigInt(message.deadline);

  // 4. Build permitted array hash (TokenPermissions[])
  const permitted = message.permitted as Array<{ token: string; amount: string }>;
  const tokenTypeHash = keccak256(toBytes('TokenPermissions(address token,uint256 amount)'));
  const tokenHashes: Uint8Array[] = [];

  for (const perm of permitted) {
    const token = perm.token as Address;
    const amount = BigInt(perm.amount);

    const tokenEncoded = concatBytes(
      encodeBytes32(tokenTypeHash),
      encodeAddress(token),
      encodeUint256(amount)
    );
    tokenHashes.push(encodeBytes32(keccak256(tokenEncoded)));
  }

  const permittedHash = keccak256(concatBytes(...tokenHashes));

  // 5. Build witness hash
  const witness = message.witness as any;
  const expires = witness.expires as number;
  const inputOracle = witness.inputOracle as Address;
  const outputs = witness.outputs as Array<any>;

  // Build outputs array hash (MandateOutput[])
  const outputTypeHash = keccak256(toBytes('MandateOutput(bytes32 oracle,bytes32 settler,uint256 chainId,bytes32 token,uint256 amount,bytes32 recipient,bytes callbackData,bytes context)'));
  const outputHashes: Uint8Array[] = [];

  for (const output of outputs) {
    const oracle = parseBytes32(output.oracle);
    const settler = parseBytes32(output.settler);
    const outputChainId = BigInt(output.chainId);
    const token = parseBytes32(output.token);
    const amount = BigInt(output.amount);
    const recipient = parseBytes32(output.recipient);
    
    // Hash call and context data
    const callStr = output.callbackData || '0x';
    const contextStr = output.context || '0x';
    const callBytes = callStr === '0x' ? new Uint8Array(0) : hexToBytes(callStr as Hex);
    const contextBytes = contextStr === '0x' ? new Uint8Array(0) : hexToBytes(contextStr as Hex);
    const callHash = encodeBytes32(keccak256(callBytes));
    const contextHash = encodeBytes32(keccak256(contextBytes));

    const outputEncoded = concatBytes(
      encodeBytes32(outputTypeHash),
      oracle,
      settler,
      encodeUint256(outputChainId),
      token,
      encodeUint256(amount),
      recipient,
      callHash,
      contextHash
    );
    outputHashes.push(encodeBytes32(keccak256(outputEncoded)));
  }

  const outputsHash = keccak256(concatBytes(...outputHashes));

  // Build witness struct hash
  const witnessTypeHash = keccak256(toBytes('Permit2Witness(uint32 expires,address inputOracle,MandateOutput[] outputs)MandateOutput(bytes32 oracle,bytes32 settler,uint256 chainId,bytes32 token,uint256 amount,bytes32 recipient,bytes call,bytes context)'));
  const witnessEncoded = concatBytes(
    encodeBytes32(witnessTypeHash),
    encodeUint32(expires),
    encodeAddress(inputOracle),
    encodeBytes32(outputsHash)
  );
  const witnessHash = keccak256(witnessEncoded);

  // 6. Build final struct hash
  const structEncoded = concatBytes(
    encodeBytes32(typeHash),
    encodeBytes32(permittedHash),
    encodeAddress(spender),
    encodeUint256(nonce),
    encodeUint256(deadline),
    encodeBytes32(witnessHash)
  );
  const structHash = keccak256(structEncoded);

  // 7. Compute final EIP-712 digest
  return computeEip712Digest(domainHash, structHash);
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
 * Reconstruct Compact digest with manual encoding
 * 
 * Based on: solver-types/src/utils/eip712.rs::reconstruct_compact_digest
 */
async function reconstructCompactDigest(
  payload: OrderPayload,
  client?: PublicClient
): Promise<Hex> {
  const domain = payload.domain as any;
  const message = payload.message as any;

  const normalizedDomain = {
    name: domain.name as string,
    chainId: BigInt(domain.chainId),
    verifyingContract: domain.verifyingContract as Address,
    version: domain.version ?? '1',
  };

  // 1. Compute domain separator
  let domainSeparator: Hex;
  if (client) {
    domainSeparator = await fetchDomainSeparator(normalizedDomain.verifyingContract, client);
  } else {
    console.warn('⚠️  WARNING: Computing domain separator locally (no client provided)');
    console.warn('   This may not match the on-chain domain separator!');
    console.warn('   Domain:', normalizedDomain);
    domainSeparator = computeDomainSeparator(normalizedDomain);
    console.warn('   Computed domain separator:', domainSeparator);
    console.warn('   Expected from solver:', '0xc521956382ee2d10650715bb8cfd09c6d2fec7e3e32da559068ba06354306ad3');
  }

  // 2. Extract message fields
  const arbiter = message.arbiter as Address;
  const sponsor = message.sponsor as Address;
  const nonce = BigInt(message.nonce);
  const expires = BigInt(message.expires);

  // 3. Build commitments (Lock[]) hash
  const lockTypeHash = keccak256(toBytes('Lock(bytes12 lockTag,address token,uint256 amount)'));
  const commitments = message.commitments as Array<any>;
  const lockHashes: Hex[] = [];

  for (const commitment of commitments) {
    const lockTagStr = commitment.lockTag as string;
    const token = commitment.token as Address;
    const amount = BigInt(commitment.amount);

    // Parse lock tag as bytes12 (left-aligned in 32-byte word)
    const lockTagBytes = hexToBytes(lockTagStr as Hex);
    if (lockTagBytes.length !== 12) {
      throw new Error(`Invalid lockTag length: ${lockTagBytes.length}`);
    }
    const lockTagWord = new Uint8Array(32);
    lockTagWord.set(lockTagBytes, 0); // Left-aligned

    const lockEncoded = concatBytes(
      encodeBytes32(lockTypeHash),
      lockTagWord,
      encodeAddress(token),
      encodeUint256(amount)
    );
    lockHashes.push(keccak256(lockEncoded));
  }

  // Hash all lock hashes together
  const commitmentsHash = lockHashes.length > 0
    ? keccak256(concatBytes(...lockHashes.map(h => encodeBytes32(h))))
    : keccak256('0x');

  // 4. Build mandate hash
  const mandate = message.mandate as any;
  const fillDeadline = Number(mandate.fillDeadline);
  const inputOracle = mandate.inputOracle as Address;
  const outputs = mandate.outputs as Array<any>;

  // Build outputs (MandateOutput[]) hash
  const outputTypeHash = keccak256(toBytes('MandateOutput(bytes32 oracle,bytes32 settler,uint256 chainId,bytes32 token,uint256 amount,bytes32 recipient,bytes callbackData,bytes context)'));
  const outputHashes: Hex[] = [];

  for (const output of outputs) {
    const oracle = parseBytes32(output.oracle);
    const settler = parseBytes32(output.settler);
    const outputChainId = BigInt(output.chainId);
    const token = parseBytes32(output.token);
    const amount = BigInt(output.amount);
    const recipient = parseBytes32(output.recipient);

    // Hash call and context data
    const callStr = output.callbackData || '0x';
    const contextStr = output.context || '0x';
    const callBytes = callStr === '0x' ? new Uint8Array(0) : hexToBytes(callStr as Hex);
    const contextBytes = contextStr === '0x' ? new Uint8Array(0) : hexToBytes(contextStr as Hex);
    const callHash = encodeBytes32(keccak256(callBytes));
    const contextHash = encodeBytes32(keccak256(contextBytes));

    const outputEncoded = concatBytes(
      encodeBytes32(outputTypeHash),
      oracle,
      settler,
      encodeUint256(outputChainId),
      token,
      encodeUint256(amount),
      recipient,
      callHash,
      contextHash
    );
    outputHashes.push(keccak256(outputEncoded));
  }

  const outputsHash = outputHashes.length > 0
    ? keccak256(concatBytes(...outputHashes.map(h => encodeBytes32(h))))
    : keccak256('0x');

  // Build mandate struct hash
  const mandateTypeHash = keccak256(toBytes('Mandate(uint32 fillDeadline,address inputOracle,MandateOutput[] outputs)MandateOutput(bytes32 oracle,bytes32 settler,uint256 chainId,bytes32 token,uint256 amount,bytes32 recipient,bytes call,bytes context)'));
  const mandateEncoded = concatBytes(
    encodeBytes32(mandateTypeHash),
    encodeUint32(fillDeadline),
    encodeAddress(inputOracle),
    encodeBytes32(outputsHash)
  );
  const mandateHash = keccak256(mandateEncoded);

  // 5. Build final BatchCompact struct hash
  const batchCompactTypeHash = keccak256(toBytes('BatchCompact(address arbiter,address sponsor,uint256 nonce,uint256 expires,Lock[] commitments,Mandate mandate)Lock(bytes12 lockTag,address token,uint256 amount)Mandate(uint32 fillDeadline,address inputOracle,MandateOutput[] outputs)MandateOutput(bytes32 oracle,bytes32 settler,uint256 chainId,bytes32 token,uint256 amount,bytes32 recipient,bytes call,bytes context)'));
  const structEncoded = concatBytes(
    encodeBytes32(batchCompactTypeHash),
    encodeAddress(arbiter),
    encodeAddress(sponsor),
    encodeUint256(nonce),
    encodeUint256(expires),
    encodeBytes32(commitmentsHash),
    encodeBytes32(mandateHash)
  );
  const structHash = keccak256(structEncoded);

  // 6. Compute final EIP-712 digest
  const finalDigest = computeEip712Digest(domainSeparator, structHash);
  
  return finalDigest;
}

// ============================================================================
// Signature Scheme Implementations
// ============================================================================

/**
 * Sign Permit2 order (oif-escrow-v0)
 * Implements Permit2 batch witness transfer signature with 0x00 prefix
 * 
 * @param payload - The EIP-712 order payload
 * @param account - Private key account for private key signing
 * @param walletSignTypedDataFn - Wallet signing function for wallet signing
 */
async function signPermit2(
  payload: OrderPayload, 
  account?: PrivateKeyAccount,
  walletSignTypedDataFn?: (args: { domain: any; types: any; primaryType: string; message: any }) => Promise<Hex>
): Promise<Hex> {
  let signature: Hex;

  if (walletSignTypedDataFn && !account) {
    const domain = extractDomain(payload);
    const types = getTypesForPayload(payload);
    const { version, ...cleanDomain } = domain as any;
    
    // Transform message: rename 'callbackData' to 'call' to match PERMIT2_TYPES field name
    // The API returns 'callbackData' but our type definition uses 'call' to match reconstructPermit2Digest
    // TODO: Check if we need to do this or keep it callbackData
    const transformedMessage = {
      ...payload.message,
      witness: {
        ...(payload.message.witness as any),
        outputs: (payload.message.witness as any).outputs.map((output: any) => {
          const { callbackData, ...rest } = output;
          return { ...rest, call: callbackData };
        })
      }
    };
    
    // Normalize message to convert string values to proper types (BigInt, Hex, etc.)
    const normalizedMessage = normalizeTypedDataMessage(transformedMessage as Record<string, any>);
    
    signature = await walletSignTypedDataFn({ 
      domain: cleanDomain, 
      types,
      primaryType: payload.primaryType, 
      message: normalizedMessage 
    });
  } else if (account) {
    const digest = reconstructPermit2Digest(payload);
    signature = await account.sign({ hash: digest });
  } else {
    throw new Error('Either account or walletSignTypedDataFn must be provided');
  }

  // Add Permit2 prefix (0x00) and return
  return `0x00${signature.slice(2)}` as Hex;
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
    const normalizedMessage = normalizeTypedDataMessage(payload.message as Record<string, any>);
    
    signature = await walletSignFn({
      domain,
      types,
      primaryType: payload.primaryType,
      message: normalizedMessage,
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
  const walletSignTypedDataFn = config?.walletSignTypedData;

  // Require either private key or wallet signing function
  if (!account && !walletSignTypedDataFn) {
    throw new Error('Either privateKey or config.walletSignTypedData must be provided');
  }

  // Setup public client if RPC URL provided and not already configured
  let effectiveConfig = config;
  if (config?.rpcUrl && !config.publicClient) {
    // Get chain ID from the order payload domain
    const payloadDomain = quote.order.payload.domain as any;
    const chainId = typeof payloadDomain.chainId === 'string' 
      ? parseInt(payloadDomain.chainId) 
      : Number(payloadDomain.chainId);
    
    effectiveConfig = {
      ...config,
      publicClient: createPublicClient({
        chain: {
          id: chainId,
          name: `Chain ${chainId}`,
          nativeCurrency: { name: 'ETH', symbol: 'ETH', decimals: 18 },
          rpcUrls: {
            default: { http: [config.rpcUrl] },
            public: { http: [config.rpcUrl] },
          },
        },
        transport: http(config.rpcUrl),
      }),
    };
  }

  // Route to appropriate signing function based on order type
  switch (quote.order.type) {
    case 'oif-escrow-v0':
      // Permit2: Supports both wallet and private key signing
      return signPermit2(quote.order.payload, account, walletSignTypedDataFn);

    case 'oif-resource-lock-v0':
      return signCompact(quote.order.payload, account, effectiveConfig, walletSignTypedDataFn);

    case 'oif-3009-v0':
      return signEip3009(quote.order.payload, quote.order.metadata, account, effectiveConfig, walletSignTypedDataFn);

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

  // Setup public client if needed (extract chain ID from payload)
  let effectiveConfig = config;
  if (config?.rpcUrl && !config.publicClient) {
    // Get chain ID from the order payload domain
    const payloadDomain = payload.domain as any;
    const chainId = typeof payloadDomain.chainId === 'string' 
      ? parseInt(payloadDomain.chainId) 
      : Number(payloadDomain.chainId);
    
    effectiveConfig = {
      ...config,
      publicClient: createPublicClient({
        chain: {
          id: chainId,
          name: `Chain ${chainId}`,
          nativeCurrency: { name: 'ETH', symbol: 'ETH', decimals: 18 },
          rpcUrls: {
            default: { http: [config.rpcUrl] },
            public: { http: [config.rpcUrl] },
          },
        },
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
  reconstructPermit2Digest,
  reconstructEip3009Digest,
  reconstructCompactDigest,
  fetchDomainSeparator,
  computeDomainSeparator,
  computeEip712Digest,
};