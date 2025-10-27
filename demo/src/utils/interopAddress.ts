/**
 * InteropAddress utilities for converting between standard Ethereum addresses
 * and ERC-7930 interoperable address format
 *
 * Note: The InteropAddress is serialized as a hex string in the API, not as a JSON object.
 * Format: 0x[version][chain_type][chain_ref_length][addr_length][chain_reference][address]
 */

// Type alias for InteropAddress - it's a hex-encoded string
export type InteropAddress = string;

/**
 * Convert a hex address string to byte array
 */
function hexToBytes(hex: string): number[] {
  // Remove 0x prefix if present
  const cleanHex = hex.startsWith('0x') ? hex.slice(2) : hex;
  const bytes: number[] = [];

  for (let i = 0; i < cleanHex.length; i += 2) {
    bytes.push(parseInt(cleanHex.slice(i, i + 2), 16));
  }

  return bytes;
}

/**
 * Convert byte array to hex string
 */
function bytesToHex(bytes: number[]): string {
  return '0x' + bytes.map((b) => b.toString(16).padStart(2, '0')).join('');
}

/**
 * Convert chain ID number to byte array (big-endian)
 */
function chainIdToBytes(chainId: number): number[] {
  const bytes: number[] = [];
  let value = chainId;

  // Convert to big-endian bytes
  while (value > 0) {
    bytes.unshift(value & 0xff);
    value >>= 8;
  }

  // Ensure at least one byte
  if (bytes.length === 0) {
    bytes.push(0);
  }

  return bytes;
}

/**
 * Convert byte array to chain ID number (big-endian)
 */
function bytesToChainId(bytes: number[]): number {
  return bytes.reduce((acc, byte) => (acc << 8) | byte, 0);
}

/**
 * Convert standard Ethereum address and chain ID to InteropAddress hex string
 *
 * Format: 0x[version][chain_type][chain_ref_length][addr_length][chain_reference][address]
 *
 * @param address - Ethereum address (0x prefixed hex string)
 * @param chainId - Chain ID number
 * @returns InteropAddress as hex string
 */
export function toInteropAddress(
  address: string,
  chainId: number
): InteropAddress {
  // Version byte
  const version = '01';

  // Chain type: EIP-155 (0x0000)
  const chainType = '0000';

  // Encode chain ID as minimal big-endian
  const chainRefBytes = chainIdToBytes(chainId);
  const chainRefHex = bytesToHex(chainRefBytes).slice(2); // Remove 0x prefix
  const chainRefLength = chainRefBytes.length.toString(16).padStart(2, '0');

  // Address bytes
  const addressBytes = hexToBytes(address);
  const addressHex = bytesToHex(addressBytes).slice(2); // Remove 0x prefix
  const addressLength = addressBytes.length.toString(16).padStart(2, '0');

  // Combine all parts
  return `0x${version}${chainType}${chainRefLength}${addressLength}${chainRefHex}${addressHex}`;
}

/**
 * Convert InteropAddress hex string to standard Ethereum address and chain ID
 *
 * @param interopHex - InteropAddress as hex string
 * @returns Object with address (hex string) and chainId (number)
 */
export function fromInteropAddress(interopHex: InteropAddress): {
  address: string;
  chainId: number;
} {
  // Remove 0x prefix
  const hex = interopHex.startsWith('0x') ? interopHex.slice(2) : interopHex;

  let offset = 0;

  // Parse version (1 byte) - skip for now
  offset += 2;

  // Parse chain type (2 bytes)
  offset += 4; // Skip chain type

  // Parse chain reference length (1 byte)
  const chainRefLength = parseInt(hex.slice(offset, offset + 2), 16);
  offset += 2;

  // Parse address length (1 byte)
  const addressLength = parseInt(hex.slice(offset, offset + 2), 16);
  offset += 2;

  // Parse chain reference
  const chainRefHex = hex.slice(offset, offset + chainRefLength * 2);
  const chainRefBytes = hexToBytes('0x' + chainRefHex);
  const chainId = bytesToChainId(chainRefBytes);
  offset += chainRefLength * 2;

  // Parse address
  const addressHex = hex.slice(offset, offset + addressLength * 2);
  const address = '0x' + addressHex;

  return { address, chainId };
}

/**
 * Format InteropAddress for display (shortened)
 */
export function formatInteropAddress(addr: InteropAddress): string {
  const { address, chainId } = fromInteropAddress(addr);

  if (address.length > 10) {
    return `${address.slice(0, 6)}...${address.slice(-4)} (Chain ${chainId})`;
  }

  return `${address} (Chain ${chainId})`;
}

/**
 * Validate if a string is a valid Ethereum address
 */
export function isValidAddress(address: string): boolean {
  return /^0x[a-fA-F0-9]{40}$/.test(address);
}

/**
 * Validate if a chain ID is valid (positive integer)
 */
export function isValidChainId(chainId: number): boolean {
  return Number.isInteger(chainId) && chainId > 0;
}
