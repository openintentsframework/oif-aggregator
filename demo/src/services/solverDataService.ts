import type {
  AssetInfo,
  ChainInfo,
  RouteInfo,
  SolverDataCache,
} from '../types/solverData';

import { api } from './api';

const CACHE_KEY = 'oif-solver-data-cache';
const CACHE_DURATION_MS = 5 * 60 * 1000; // 5 minutes

// Chain ID to name mapping (can be expanded)
const CHAIN_NAMES: Record<number, string> = {
  1: 'Ethereum',
  10: 'Optimism',
  137: 'Polygon',
  42161: 'Arbitrum One',
  8453: 'Base',
  56: 'BNB Chain',
  43114: 'Avalanche',
};

class SolverDataService {
  private cache: SolverDataCache | null = null;

  /**
   * Fetch solver data from API and process it
   * @param force - If true, bypass cache and fetch fresh data
   */
  async fetchSolverData(force: boolean = false): Promise<SolverDataCache> {
    try {
      // Check cache first (unless force refresh)
      if (!force) {
        const cached = this.loadFromCache();
        if (cached && Date.now() - cached.lastFetched < CACHE_DURATION_MS) {
          this.cache = cached;
          return cached;
        }
      }

      // Fetch fresh data
      const response = await api.get('/v1/solvers');
      const data = response.data;

      const cache = this.processSolverData(data);
      this.cache = cache;
      this.saveToCache(cache);

      return cache;
    } catch (error) {
      console.error('Failed to fetch solver data:', error);
      // Try to return cached data even if stale (unless it was a forced refresh)
      if (!force) {
        const cached = this.loadFromCache();
        if (cached) {
          console.warn('Using stale cached data due to fetch error');
          this.cache = cached;
          return cached;
        }
      }
      throw error;
    }
  }

  /**
   * Process raw solver response into structured data
   */
  private processSolverData(data: any): SolverDataCache {
    const assets = new Map<string, AssetInfo>();
    const assetsByChain = new Map<number, AssetInfo[]>();
    const routes: RouteInfo[] = [];
    const chainSet = new Set<number>();

    if (!data.solvers || !Array.isArray(data.solvers)) {
      throw new Error('Invalid solver data format');
    }

    // Process each solver
    for (const solver of data.solvers) {
      const solverId = solver.solverId;
      const supportedAssets = solver.supportedAssets;

      if (!supportedAssets) continue;

      if (supportedAssets.type === 'assets') {
        // Asset-based solver - supports any-to-any within asset list
        for (const asset of supportedAssets.assets || []) {
          const key = `${asset.chainId}-${asset.address.toLowerCase()}`;
          chainSet.add(asset.chainId);

          if (assets.has(key)) {
            // Add solver to existing asset
            const existing = assets.get(key)!;
            if (!existing.solvers.includes(solverId)) {
              existing.solvers.push(solverId);
            }
          } else {
            // Create new asset entry
            assets.set(key, {
              address: asset.address,
              chainId: asset.chainId,
              symbol: asset.symbol,
              name: asset.name,
              decimals: asset.decimals,
              solvers: [solverId],
            });
          }
        }
      } else if (supportedAssets.type === 'routes') {
        // Route-based solver - supports specific origin->destination pairs
        for (const route of supportedAssets.routes || []) {
          chainSet.add(route.originChainId);
          chainSet.add(route.destinationChainId);

          // Add origin asset
          const originKey = `${route.originChainId}-${route.originTokenAddress.toLowerCase()}`;
          if (!assets.has(originKey)) {
            assets.set(originKey, {
              address: route.originTokenAddress,
              chainId: route.originChainId,
              symbol: route.originTokenSymbol || 'Unknown',
              name: route.originTokenSymbol || 'Unknown Token',
              decimals: 18, // Default, may not be accurate
              solvers: [solverId],
            });
          } else {
            const existing = assets.get(originKey)!;
            if (!existing.solvers.includes(solverId)) {
              existing.solvers.push(solverId);
            }
          }

          // Add destination asset
          const destKey = `${route.destinationChainId}-${route.destinationTokenAddress.toLowerCase()}`;
          if (!assets.has(destKey)) {
            assets.set(destKey, {
              address: route.destinationTokenAddress,
              chainId: route.destinationChainId,
              symbol: route.destinationTokenSymbol || 'Unknown',
              name: route.destinationTokenSymbol || 'Unknown Token',
              decimals: 18, // Default
              solvers: [solverId],
            });
          } else {
            const existing = assets.get(destKey)!;
            if (!existing.solvers.includes(solverId)) {
              existing.solvers.push(solverId);
            }
          }

          // Add route
          const existingRoute = routes.find(
            (r) =>
              r.originChainId === route.originChainId &&
              r.originAsset.toLowerCase() ===
                route.originTokenAddress.toLowerCase() &&
              r.destinationChainId === route.destinationChainId &&
              r.destinationAsset.toLowerCase() ===
                route.destinationTokenAddress.toLowerCase()
          );

          if (existingRoute) {
            if (!existingRoute.solvers.includes(solverId)) {
              existingRoute.solvers.push(solverId);
            }
          } else {
            routes.push({
              originChainId: route.originChainId,
              originAsset: route.originTokenAddress,
              originAssetSymbol: route.originTokenSymbol,
              destinationChainId: route.destinationChainId,
              destinationAsset: route.destinationTokenAddress,
              destinationAssetSymbol: route.destinationTokenSymbol,
              solvers: [solverId],
            });
          }
        }
      }
    }

    // Group assets by chain
    for (const asset of assets.values()) {
      if (!assetsByChain.has(asset.chainId)) {
        assetsByChain.set(asset.chainId, []);
      }
      assetsByChain.get(asset.chainId)!.push(asset);
    }

    // Sort assets by symbol within each chain
    for (const chainAssets of assetsByChain.values()) {
      chainAssets.sort((a, b) => a.symbol.localeCompare(b.symbol));
    }

    // Create chain info
    const chains: ChainInfo[] = Array.from(chainSet)
      .map((chainId) => ({
        chainId,
        name: CHAIN_NAMES[chainId] || `Chain ${chainId}`,
        assetCount: assetsByChain.get(chainId)?.length || 0,
      }))
      .sort((a, b) => a.name.localeCompare(b.name));

    return {
      assets,
      assetsByChain,
      routes,
      chains,
      lastFetched: Date.now(),
    };
  }

  /**
   * Get all assets for a specific chain
   */
  getAssetsByChain(chainId: number): AssetInfo[] {
    if (!this.cache) return [];
    return this.cache.assetsByChain.get(chainId) || [];
  }

  /**
   * Get compatible destination chains/assets for a given origin
   */
  getCompatibleDestinations(
    originChainId: number,
    originAsset: string
  ): {
    chains: number[];
    assetsByChain: Map<number, AssetInfo[]>;
  } {
    if (!this.cache) return { chains: [], assetsByChain: new Map() };

    const destChains = new Set<number>();
    const assetsByChain = new Map<number, AssetInfo[]>();

    // First, check for specific routes from this origin
    const compatibleRoutes = this.cache.routes.filter(
      (route) =>
        route.originChainId === originChainId &&
        route.originAsset.toLowerCase() === originAsset.toLowerCase()
    );

    for (const route of compatibleRoutes) {
      destChains.add(route.destinationChainId);
      const destAsset = this.cache.assets.get(
        `${route.destinationChainId}-${route.destinationAsset.toLowerCase()}`
      );
      if (destAsset) {
        if (!assetsByChain.has(route.destinationChainId)) {
          assetsByChain.set(route.destinationChainId, []);
        }
        const chainAssets = assetsByChain.get(route.destinationChainId)!;
        if (
          !chainAssets.find(
            (a) => a.address.toLowerCase() === destAsset.address.toLowerCase()
          )
        ) {
          chainAssets.push(destAsset);
        }
      }
    }

    // If no specific routes found, check if this is an asset-based solver
    // (supports any-to-any within its asset list)
    if (compatibleRoutes.length === 0) {
      const originAssetKey = `${originChainId}-${originAsset.toLowerCase()}`;
      const originAssetInfo = this.cache.assets.get(originAssetKey);

      if (originAssetInfo) {
        // Check if any solver supports this asset
        for (const solver of originAssetInfo.solvers) {
          // Find all assets supported by the same solver
          for (const asset of this.cache.assets.values()) {
            if (asset.solvers.includes(solver)) {
              destChains.add(asset.chainId);
              if (!assetsByChain.has(asset.chainId)) {
                assetsByChain.set(asset.chainId, []);
              }
              const chainAssets = assetsByChain.get(asset.chainId)!;
              if (
                !chainAssets.find(
                  (a) => a.address.toLowerCase() === asset.address.toLowerCase()
                )
              ) {
                chainAssets.push(asset);
              }
            }
          }
        }
      }
    }

    // If still no compatible destinations, allow all chains as fallback
    if (destChains.size === 0) {
      // Return all available chains and their assets
      for (const [chainId, assets] of this.cache.assetsByChain.entries()) {
        destChains.add(chainId);
        assetsByChain.set(chainId, [...assets]);
      }
    }

    return {
      chains: Array.from(destChains),
      assetsByChain,
    };
  }

  /**
   * Get solvers that support a specific route
   * Checks both route-based solvers and asset-based solvers
   */
  getSolversForRoute(
    originChainId: number,
    originAsset: string,
    destChainId: number,
    destAsset: string
  ): string[] {
    if (!this.cache) return [];

    const solvers = new Set<string>();

    // Check route-based solvers (explicit routes)
    const route = this.cache.routes.find(
      (r) =>
        r.originChainId === originChainId &&
        r.originAsset.toLowerCase() === originAsset.toLowerCase() &&
        r.destinationChainId === destChainId &&
        r.destinationAsset.toLowerCase() === destAsset.toLowerCase()
    );

    if (route) {
      route.solvers.forEach((s) => solvers.add(s));
    }

    // Check asset-based solvers (any-to-any within asset list)
    const originKey = `${originChainId}-${originAsset.toLowerCase()}`;
    const destKey = `${destChainId}-${destAsset.toLowerCase()}`;

    const originAssetInfo = this.cache.assets.get(originKey);
    const destAssetInfo = this.cache.assets.get(destKey);

    if (originAssetInfo && destAssetInfo) {
      // Find solvers that support both assets
      const commonSolvers = originAssetInfo.solvers.filter((solver) =>
        destAssetInfo.solvers.includes(solver)
      );
      commonSolvers.forEach((s) => solvers.add(s));
    }

    return Array.from(solvers);
  }

  /**
   * Get all chains
   */
  getChains(): ChainInfo[] {
    return this.cache?.chains || [];
  }

  /**
   * Get cached data (or null if not loaded)
   */
  getCachedData(): SolverDataCache | null {
    return this.cache;
  }

  /**
   * Clear cache
   */
  clearCache(): void {
    this.cache = null;
    localStorage.removeItem(CACHE_KEY);
  }

  /**
   * Save to localStorage
   */
  private saveToCache(cache: SolverDataCache): void {
    try {
      // Convert Maps to arrays for JSON serialization
      const serializable = {
        assets: Array.from(cache.assets.entries()),
        assetsByChain: Array.from(cache.assetsByChain.entries()),
        routes: cache.routes,
        chains: cache.chains,
        lastFetched: cache.lastFetched,
      };
      localStorage.setItem(CACHE_KEY, JSON.stringify(serializable));
    } catch (error) {
      console.warn('Failed to save cache to localStorage:', error);
    }
  }

  /**
   * Load from localStorage
   */
  private loadFromCache(): SolverDataCache | null {
    try {
      const cached = localStorage.getItem(CACHE_KEY);
      if (!cached) return null;

      const parsed = JSON.parse(cached);
      return {
        assets: new Map(parsed.assets),
        assetsByChain: new Map(parsed.assetsByChain),
        routes: parsed.routes,
        chains: parsed.chains,
        lastFetched: parsed.lastFetched,
      };
    } catch (error) {
      console.warn('Failed to load cache from localStorage:', error);
      return null;
    }
  }
}

// Export singleton instance
export const solverDataService = new SolverDataService();
