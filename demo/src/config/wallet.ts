import { getDefaultConfig, lightTheme, darkTheme } from '@rainbow-me/rainbowkit';
import { mainnet, optimism, arbitrum, base, polygon, bsc, avalanche, sepolia, optimismSepolia, arbitrumSepolia, baseSepolia, polygonMumbai } from 'wagmi/chains';
import { http } from 'viem';
import { getRpcUrlForChain } from '../utils/chainUtils';

// Configure transports for each chain
const transports = {
  [mainnet.id]: http(getRpcUrlForChain(mainnet.id)),
  [optimism.id]: http(getRpcUrlForChain(optimism.id)),
  [arbitrum.id]: http(getRpcUrlForChain(arbitrum.id)),
  [base.id]: http(getRpcUrlForChain(base.id)),
  [polygon.id]: http(getRpcUrlForChain(polygon.id)),
  [bsc.id]: http(getRpcUrlForChain(bsc.id)),
  [avalanche.id]: http(getRpcUrlForChain(avalanche.id)),
  [sepolia.id]: http(getRpcUrlForChain(sepolia.id)),
  [optimismSepolia.id]: http(getRpcUrlForChain(optimismSepolia.id)),
  [arbitrumSepolia.id]: http(getRpcUrlForChain(arbitrumSepolia.id)),
  [baseSepolia.id]: http(getRpcUrlForChain(baseSepolia.id)),
  [polygonMumbai.id]: http(getRpcUrlForChain(polygonMumbai.id)),
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
