import { createPublicClient, http } from 'viem';
import type { Address, Chain, Hex, WalletClient } from 'viem';
import { optimismSepolia, baseSepolia } from 'viem/chains';

const CHAIN_CONFIGS: Partial<Record<number, Chain>> = {
  [optimismSepolia.id]: optimismSepolia,
  [baseSepolia.id]: baseSepolia,
};

const ERC20_ABI = [
  {
    name: 'allowance',
    type: 'function',
    stateMutability: 'view',
    inputs: [
      { name: 'owner', type: 'address' },
      { name: 'spender', type: 'address' },
    ],
    outputs: [{ name: '', type: 'uint256' }],
  },
  {
    name: 'approve',
    type: 'function',
    stateMutability: 'nonpayable',
    inputs: [
      { name: 'spender', type: 'address' },
      { name: 'amount', type: 'uint256' },
    ],
    outputs: [{ name: '', type: 'bool' }],
  },
] as const;

const THE_COMPACT_ABI = [
  {
    name: 'depositERC20',
    type: 'function',
    stateMutability: 'nonpayable',
    inputs: [
      { name: 'token', type: 'address' },
      { name: 'lockTag', type: 'bytes12' },
      { name: 'amount', type: 'uint256' },
      { name: 'recipient', type: 'address' },
    ],
    outputs: [{ name: 'id', type: 'uint256' }],
  },
] as const;

const ERC6909_ABI = [
  {
    name: 'balanceOf',
    type: 'function',
    stateMutability: 'view',
    inputs: [
      { name: 'account', type: 'address' },
      { name: 'id', type: 'uint256' },
    ],
    outputs: [{ name: '', type: 'uint256' }],
  },
] as const;

interface PublicClientConfig {
  chainId: number;
  rpcUrl?: string;
}

export function createCompactPublicClient({ chainId, rpcUrl }: PublicClientConfig) {
  const chain = CHAIN_CONFIGS[chainId];

  return createPublicClient({
    ...(chain ? { chain } : {}),
    transport: rpcUrl ? http(rpcUrl) : http(),
  });
}

export function computeCompactLockId(lockTag: Hex, tokenAddress: Address): bigint {
  if (!lockTag.startsWith('0x') || !tokenAddress.startsWith('0x')) {
    throw new Error('Lock tag and token address must be hex strings with 0x prefix');
  }

  const lockTagValue = BigInt(lockTag);
  const tokenValue = BigInt(tokenAddress);

  return (lockTagValue << 160n) | tokenValue;
}

interface CheckCompactAllowanceParams {
  owner: Address;
  tokenAddress: Address;
  contractAddress: Address;
  requiredAmount: bigint;
  chainId: number;
  rpcUrl?: string;
}

export async function checkCompactAllowance({
  owner,
  tokenAddress,
  contractAddress,
  requiredAmount,
  chainId,
  rpcUrl,
}: CheckCompactAllowanceParams): Promise<{ hasApproval: boolean; currentAllowance: bigint }> {
  const publicClient = createCompactPublicClient({ chainId, rpcUrl });

  try {
    const allowance = (await publicClient.readContract({
      address: tokenAddress,
      abi: ERC20_ABI,
      functionName: 'allowance',
      args: [owner, contractAddress],
    })) as bigint;

    return {
      hasApproval: allowance >= requiredAmount,
      currentAllowance: allowance,
    };
  } catch (error) {
    console.error('Failed to check TheCompact allowance:', error);
    throw new Error(`Failed to check TheCompact approval: ${(error as Error).message}`);
  }
}

interface RequestCompactApprovalParams {
  tokenAddress: Address;
  contractAddress: Address;
  amount: bigint;
  walletClient: WalletClient;
}

export async function requestCompactApproval({
  tokenAddress,
  contractAddress,
  amount,
  walletClient,
}: RequestCompactApprovalParams) {
  try {
    return await walletClient.writeContract({
      address: tokenAddress,
      abi: ERC20_ABI,
      functionName: 'approve',
      args: [contractAddress, amount],
    });
  } catch (error) {
    console.error('Failed to approve TheCompact contract:', error);
    throw new Error(`Approval failed: ${(error as Error).message}`);
  }
}

interface CheckCompactDepositParams {
  sponsor: Address;
  contractAddress: Address;
  lockTag: Hex;
  tokenAddress: Address;
  requiredAmount: bigint;
  chainId: number;
  rpcUrl?: string;
}

export async function checkCompactDeposit({
  sponsor,
  contractAddress,
  lockTag,
  tokenAddress,
  requiredAmount,
  chainId,
  rpcUrl,
}: CheckCompactDepositParams): Promise<{ hasDeposit: boolean; depositedAmount: bigint; lockId: bigint }> {
  const publicClient = createCompactPublicClient({ chainId, rpcUrl });

  try {
    const lockId = computeCompactLockId(lockTag, tokenAddress);
    const depositedAmount = (await publicClient.readContract({
      address: contractAddress,
      abi: ERC6909_ABI,
      functionName: 'balanceOf',
      args: [sponsor, lockId],
    })) as bigint;

    return {
      hasDeposit: depositedAmount >= requiredAmount,
      depositedAmount,
      lockId,
    };
  } catch (error) {
    console.error('Failed to check TheCompact deposit:', error);
    throw new Error(`Failed to check TheCompact deposit: ${(error as Error).message}`);
  }
}

interface RequestCompactDepositParams {
  contractAddress: Address;
  tokenAddress: Address;
  lockTag: Hex;
  amount: bigint;
  recipient: Address;
  walletClient: WalletClient;
}

export async function requestCompactDeposit({
  contractAddress,
  tokenAddress,
  lockTag,
  amount,
  recipient,
  walletClient,
}: RequestCompactDepositParams) {
  try {
    return await walletClient.writeContract({
      address: contractAddress,
      abi: THE_COMPACT_ABI,
      functionName: 'depositERC20',
      args: [tokenAddress, lockTag, amount, recipient],
    });
  } catch (error) {
    console.error('Failed to deposit into TheCompact:', error);
    throw new Error(`Deposit failed: ${(error as Error).message}`);
  }
}

interface WaitForTransactionParams {
  chainId: number;
  rpcUrl?: string;
  hash: Hex;
}

export async function waitForCompactTransaction({ chainId, rpcUrl, hash }: WaitForTransactionParams) {
  const publicClient = createCompactPublicClient({ chainId, rpcUrl });
  return publicClient.waitForTransactionReceipt({ hash });
}

export type { Address, Hex };

