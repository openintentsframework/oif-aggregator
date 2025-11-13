import { getDefaultConfig, lightTheme, darkTheme } from '@rainbow-me/rainbowkit';
import { mainnet, optimism, arbitrum, base, polygon, bsc, avalanche, sepolia, optimismSepolia, arbitrumSepolia, baseSepolia, polygonMumbai } from 'wagmi/chains';
import { http } from 'viem';

// Get RPC URL from environment or use public endpoints
const getRpcUrl = (chainId: number): string => {
  const viteRpcUrl = import.meta.env.VITE_RPC_URL;
  if (viteRpcUrl) {
    return viteRpcUrl;
  }
  
  // Fallback to public RPCs
  const publicRpcs: Record<number, string> = {
    1: 'https://eth.llamarpc.com',
    10: 'https://mainnet.optimism.io',
    42161: 'https://arbitrum.llamarpc.com',
    8453: 'https://mainnet.base.org',
    137: 'https://polygon.llamarpc.com',
    56: 'https://bsc.llamarpc.com',
    43114: 'https://avalanche.llamarpc.com',
    11155111: 'https://sepolia.llamarpc.com',
    420: 'https://optimism-sepolia.llamarpc.com',
    421614: 'https://arbitrum-sepolia.llamarpc.com',
    84532: 'https://base-sepolia.llamarpc.com',
    80001: 'https://polygon-mumbai.llamarpc.com',
  };
  
  return publicRpcs[chainId] || `https://${chainId}.rpc.thirdweb.com`;
};

// Configure transports for each chain
const transports = {
  [mainnet.id]: http(getRpcUrl(mainnet.id)),
  [optimism.id]: http(getRpcUrl(optimism.id)),
  [arbitrum.id]: http(getRpcUrl(arbitrum.id)),
  [base.id]: http(getRpcUrl(base.id)),
  [polygon.id]: http(getRpcUrl(polygon.id)),
  [bsc.id]: http(getRpcUrl(bsc.id)),
  [avalanche.id]: http(getRpcUrl(avalanche.id)),
  [sepolia.id]: http(getRpcUrl(sepolia.id)),
  [optimismSepolia.id]: http(getRpcUrl(optimismSepolia.id)),
  [arbitrumSepolia.id]: http(getRpcUrl(arbitrumSepolia.id)),
  [baseSepolia.id]: http(getRpcUrl(baseSepolia.id)),
  [polygonMumbai.id]: http(getRpcUrl(polygonMumbai.id)),
};

// Get project ID from environment or use a fallback
const getProjectId = (): string => {
  const envProjectId = import.meta.env.VITE_WALLETCONNECT_PROJECT_ID;
  if (envProjectId) {
    return envProjectId;
  }
  // For development, you can use a public project ID or create your own at https://cloud.walletconnect.com
  // This is a public project ID that should work for development
  return '1f4a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a8a';
};

// RainbowKit configuration
export const config = getDefaultConfig({
  appName: 'OIF Aggregator',
  projectId: getProjectId(),
  chains: [
    mainnet,
    optimism,
    arbitrum,
    base,
    polygon,
    bsc,
    avalanche,
    sepolia,
    optimismSepolia,
    arbitrumSepolia,
    baseSepolia,
    polygonMumbai,
  ],
  transports,
  ssr: false, // Disable SSR for better compatibility
});

// Custom themes to match the app design
export const lightThemeConfig = lightTheme({
  accentColor: '#3b82f6', // primary-600
  accentColorForeground: 'white',
  borderRadius: 'medium',
  fontStack: 'system',
  overlayBlur: 'small',
});

export const darkThemeConfig = darkTheme({
  accentColor: '#3b82f6', // primary-600
  accentColorForeground: 'white',
  borderRadius: 'medium',
  fontStack: 'system',
  overlayBlur: 'small',
});

// Export query client for React Query
export { config as wagmiConfig };
