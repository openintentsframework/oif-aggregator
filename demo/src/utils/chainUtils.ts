const MAINNET_CHAIN_ID = 1;

/**
 * Get the RPC URL for a specific chain ID.
 * Uses chain-specific public RPC endpoints first so old global RPC values do not
 * route known chains through an unreliable endpoint.
 */
export const getRpcUrlForChain = (chainId: number): string => {
  if (chainId === MAINNET_CHAIN_ID && import.meta.env.VITE_MAINNET_RPC_URL) {
    return import.meta.env.VITE_MAINNET_RPC_URL;
  }

  const publicRpcs: Record<number, string> = {
    [MAINNET_CHAIN_ID]: 'https://ethereum-rpc.publicnode.com',
    10: 'https://mainnet.optimism.io',
    42161: 'https://arb1.arbitrum.io/rpc',
    8453: 'https://mainnet.base.org',
    137: 'https://polygon-rpc.com',
    56: 'https://bsc-dataseed.binance.org',
    43114: 'https://api.avax.network/ext/bc/C/rpc',
    11155111: 'https://ethereum-sepolia-rpc.publicnode.com',
    11155420: 'https://sepolia.optimism.io',
    420: 'https://optimism-sepolia-rpc.publicnode.com',
    84532: 'https://sepolia.base.org',
    421614: 'https://sepolia-rollup.arbitrum.io/rpc',
    80001: 'https://polygon-mumbai-bor-rpc.publicnode.com',
  };

  // Return chain-specific RPC or fallback to env var or default
  return publicRpcs[chainId] || import.meta.env.VITE_RPC_URL || `https://rpc.ankr.com/eth`;
};
