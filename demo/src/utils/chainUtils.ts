/**
 * Get the RPC URL for a specific chain ID
 * Uses chain-specific public RPC endpoints with fallback to env var or default
 */
export const getRpcUrlForChain = (chainId: number): string => {
  const publicRpcs: Record<number, string> = {
    1: 'https://eth.llamarpc.com',
    10: 'https://optimism.llamarpc.com',
    42161: 'https://arbitrum.llamarpc.com',
    8453: 'https://base.llamarpc.com',
    11155111: 'https://sepolia.llamarpc.com',
    11155420: 'https://sepolia.optimism.io',
    84532: 'https://sepolia.base.org',
    421614: 'https://sepolia-rollup.arbitrum.io/rpc',
  };
  
  // Return chain-specific RPC or fallback to env var or default
  return publicRpcs[chainId] || import.meta.env.VITE_RPC_URL || `https://rpc.ankr.com/eth`;
};

