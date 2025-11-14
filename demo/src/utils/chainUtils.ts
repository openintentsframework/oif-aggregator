/**
 * Get the RPC URL for a specific chain ID
 * Uses chain-specific public RPC endpoints with fallback to env var or default
 */
export const getRpcUrlForChain = (chainId: number): string => {
  const publicRpcs: Record<number, string> = {
    1: 'https://eth.llamarpc.com',
    10: 'https://mainnet.optimism.io',
    42161: 'https://arb1.arbitrum.io/rpc',
    8453: 'https://mainnet.base.org',
    11155111: 'https://sepolia.llamarpc.com',
    11155420: 'https://sepolia.optimism.io',
    84532: 'https://sepolia.base.org',
    421614: 'https://sepolia-rollup.arbitrum.io/rpc',
  };
  
  // Return chain-specific RPC or fallback to env var or default
  return publicRpcs[chainId] || import.meta.env.VITE_RPC_URL || `https://rpc.ankr.com/eth`;
};

