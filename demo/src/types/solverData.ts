/**
 * Types for processed solver and asset data
 */

export interface AssetInfo {
  address: string;
  chainId: number;
  symbol: string;
  name: string;
  decimals: number;
  solvers: string[]; // Solver IDs that support this asset
}

export interface RouteInfo {
  originChainId: number;
  originAsset: string;
  originAssetSymbol?: string;
  destinationChainId: number;
  destinationAsset: string;
  destinationAssetSymbol?: string;
  solvers: string[]; // Solver IDs that support this route
}

export interface ChainInfo {
  chainId: number;
  name: string;
  assetCount: number;
}

export interface SolverDataCache {
  assets: Map<string, AssetInfo>; // Key: "chainId-address"
  assetsByChain: Map<number, AssetInfo[]>;
  routes: RouteInfo[];
  chains: ChainInfo[];
  lastFetched: number;
}

export interface SimpleQuoteFormData {
  amount: string;
  fromAsset: AssetInfo | null;
  fromChain: number;
  toChain: number;
  toAsset: AssetInfo | null;
  userAddress: string;
  receiverAddress?: string; // Optional separate receiver
}

