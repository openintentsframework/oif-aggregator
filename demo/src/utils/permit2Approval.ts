/**
 * Permit2 approval utilities
 * Handles checking and requesting token approvals for Permit2 contract
 */

import { createPublicClient, http, type Address, type Hex } from 'viem';
import { optimismSepolia, baseSepolia } from 'viem/chains';

// Permit2 contract addresses per chain
const PERMIT2_ADDRESSES: Record<number, Address> = {
  11155420: '0x000000000022D473030F116dDEE9F6B43aC78BA3', // Optimism Sepolia
  84532: '0x000000000022D473030F116dDEE9F6B43aC78BA3', // Base Sepolia
  // Add more chains as needed
};

// Minimum allowance threshold (if below this, consider it needs approval)
const MIN_ALLOWANCE = BigInt('1000000000000'); // 1M tokens in smallest unit

/**
 * Check if user has approved Permit2 to spend a token
 */
export async function checkPermit2Approval(
  userAddress: Address,
  tokenAddress: Address,
  chainId: number,
  rpcUrl?: string
): Promise<{ hasApproval: boolean; currentAllowance: bigint }> {
  const permit2Address = PERMIT2_ADDRESSES[chainId];
  
  if (!permit2Address) {
    throw new Error(`Permit2 not supported on chain ${chainId}`);
  }

  // Create public client for reading
  const chain = chainId === 11155420 ? optimismSepolia : baseSepolia;
  const publicClient = createPublicClient({
    chain,
    transport: rpcUrl ? http(rpcUrl) : http(),
  });

  try {
    // Call allowance(owner, spender) on the token contract
    const allowance = await publicClient.readContract({
      address: tokenAddress,
      abi: [{
        name: 'allowance',
        type: 'function',
        stateMutability: 'view',
        inputs: [
          { name: 'owner', type: 'address' },
          { name: 'spender', type: 'address' }
        ],
        outputs: [{ name: '', type: 'uint256' }]
      }],
      functionName: 'allowance',
      args: [userAddress, permit2Address],
    }) as bigint;

    return {
      hasApproval: allowance >= MIN_ALLOWANCE,
      currentAllowance: allowance,
    };
  } catch (error) {
    console.error('Failed to check Permit2 approval:', error);
    throw new Error(`Failed to check approval: ${(error as Error).message}`);
  }
}

/**
 * Request approval for Permit2 using wallet
 */
export async function requestPermit2Approval(
  tokenAddress: Address,
  chainId: number,
  walletClient: any // From wagmi
): Promise<Hex> {
  const permit2Address = PERMIT2_ADDRESSES[chainId];
  
  if (!permit2Address) {
    throw new Error(`Permit2 not supported on chain ${chainId}`);
  }

  try {
    // Send approve transaction (max uint256 for infinite approval)
    const hash = await walletClient.writeContract({
      address: tokenAddress,
      abi: [{
        name: 'approve',
        type: 'function',
        stateMutability: 'nonpayable',
        inputs: [
          { name: 'spender', type: 'address' },
          { name: 'amount', type: 'uint256' }
        ],
        outputs: [{ name: '', type: 'bool' }]
      }],
      functionName: 'approve',
      args: [permit2Address, BigInt('115792089237316195423570985008687907853269984665640564039457584007913129639935')], // max uint256
    });

    return hash as Hex;
  } catch (error) {
    console.error('Failed to approve Permit2:', error);
    throw new Error(`Approval failed: ${(error as Error).message}`);
  }
}

/**
 * Get Permit2 address for a chain
 */
export function getPermit2Address(chainId: number): Address | undefined {
  return PERMIT2_ADDRESSES[chainId];
}

