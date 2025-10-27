import type { AssetInfo, RouteInfo, SolverResponse } from '../types/api';
import { useEffect, useState } from 'react';

import { solverApi } from '../services/api';

interface SolverDetailProps {
  solverId: string;
  onBack: () => void;
}

export default function SolverDetail({ solverId, onBack }: SolverDetailProps) {
  const [solver, setSolver] = useState<SolverResponse | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string>('');

  useEffect(() => {
    fetchSolver();
  }, [solverId]);

  const fetchSolver = async () => {
    setIsLoading(true);
    setError('');
    try {
      const response = await solverApi.getSolverById(solverId);
      setSolver(response);
    } catch (err) {
      setError(
        err instanceof Error ? err.message : 'Failed to fetch solver details'
      );
    } finally {
      setIsLoading(false);
    }
  };

  const getStatusBadgeClass = (status: string) => {
    switch (status) {
      case 'active':
        return 'bg-green-100 text-green-700 border-green-300 dark:bg-green-900/30 dark:text-green-400 dark:border-green-700';
      case 'inactive':
        return 'bg-gray-100 text-gray-700 border-gray-300 dark:bg-gray-900/30 dark:text-gray-400 dark:border-gray-700';
      case 'circuit-breaker-open':
        return 'bg-red-100 text-red-700 border-red-300 dark:bg-red-900/30 dark:text-red-400 dark:border-red-700';
      default:
        return 'bg-slate-200 text-slate-700 border-slate-400 dark:bg-slate-900/30 dark:text-slate-400 dark:border-slate-700';
    }
  };

  const getChainName = (chainId: number): string => {
    const chainNames: Record<number, string> = {
      1: 'Ethereum',
      10: 'Optimism',
      56: 'BSC',
      137: 'Polygon',
      8453: 'Base',
      42161: 'Arbitrum',
      43114: 'Avalanche',
    };
    return chainNames[chainId] || `Chain ${chainId}`;
  };

  const groupAssetsByChain = (
    assets: AssetInfo[]
  ): Map<number, AssetInfo[]> => {
    const grouped = new Map<number, AssetInfo[]>();
    assets.forEach((asset) => {
      if (!grouped.has(asset.chainId)) {
        grouped.set(asset.chainId, []);
      }
      grouped.get(asset.chainId)!.push(asset);
    });
    return grouped;
  };

  const groupRoutesByOriginChain = (
    routes: RouteInfo[]
  ): Map<number, RouteInfo[]> => {
    const grouped = new Map<number, RouteInfo[]>();
    routes.forEach((route) => {
      if (!grouped.has(route.originChainId)) {
        grouped.set(route.originChainId, []);
      }
      grouped.get(route.originChainId)!.push(route);
    });
    return grouped;
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="text-center">
          <div className="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-primary-500 mb-4"></div>
          <p className="text-slate-600 dark:text-slate-400">
            Loading solver details...
          </p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="card py-4">
        <div className="bg-red-900/20 border border-red-700 rounded-lg p-4 mb-4">
          <p className="text-red-400 font-semibold mb-2">
            ⚠️ Error Loading Solver
          </p>
          <p className="text-red-300 text-sm">{error}</p>
        </div>
        <button onClick={onBack} className="btn-secondary">
          ← Back to Solvers
        </button>
      </div>
    );
  }

  if (!solver) {
    return (
      <div className="card py-4">
        <p className="text-slate-600 dark:text-slate-400 text-center">
          Solver not found
        </p>
        <button onClick={onBack} className="btn-secondary mt-4">
          ← Back to Solvers
        </button>
      </div>
    );
  }

  return (
    <div className="space-y-4 max-w-6xl mx-auto">
      {/* Header */}
      <div className="card py-4">
        <button
          onClick={onBack}
          className="btn-secondary mb-4 text-sm px-3 py-1.5"
        >
          ← Back to Solvers
        </button>

        <div className="flex items-start justify-between">
          <div className="flex-1">
            <div className="flex items-center gap-3 mb-2">
              <h2 className="text-2xl font-bold text-slate-900 dark:text-white">
                {solver.name || solver.solverId}
              </h2>
              <span
                className={`inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-medium border ${getStatusBadgeClass(
                  solver.status
                )}`}
              >
                {solver.status}
              </span>
            </div>
            {solver.description && (
              <p className="text-slate-700 dark:text-slate-300 mb-2">
                {solver.description}
              </p>
            )}
            <p className="text-slate-600 dark:text-slate-400 text-sm font-mono">
              {solver.solverId}
            </p>
          </div>
        </div>
      </div>

      {/* Overview */}
      <div className="card py-4">
        <h3 className="text-lg font-semibold text-slate-900 dark:text-white mb-3">
          Overview
        </h3>
        <div className="grid grid-cols-2 gap-4">
          <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-3 border border-slate-300 dark:border-slate-700">
            <p className="text-xs text-slate-600 dark:text-slate-400 mb-1">
              Adapter ID
            </p>
            <p className="text-slate-900 dark:text-white font-mono text-sm">
              {solver.adapterId}
            </p>
          </div>
          <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-3 border border-slate-300 dark:border-slate-700">
            <p className="text-xs text-slate-600 dark:text-slate-400 mb-1">
              Endpoint
            </p>
            <p className="text-slate-900 dark:text-white font-mono text-sm truncate">
              {solver.endpoint}
            </p>
          </div>
          <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-3 border border-slate-300 dark:border-slate-700">
            <p className="text-xs text-slate-600 dark:text-slate-400 mb-1">
              Created
            </p>
            <p className="text-slate-900 dark:text-white text-sm">
              {new Date(solver.createdAt).toLocaleString()}
            </p>
          </div>
          <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-3 border border-slate-300 dark:border-slate-700">
            <p className="text-xs text-slate-600 dark:text-slate-400 mb-1">
              Last Seen
            </p>
            <p className="text-slate-900 dark:text-white text-sm">
              {solver.lastSeen
                ? new Date(solver.lastSeen).toLocaleString()
                : 'Never'}
            </p>
          </div>
          <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-3 border border-slate-300 dark:border-slate-700">
            <p className="text-xs text-slate-600 dark:text-slate-400 mb-1">
              Asset Type
            </p>
            <div className="flex items-center gap-2">
              <span
                className={`text-xs px-2 py-1 rounded ${
                  solver.supportedAssets.type === 'assets'
                    ? 'bg-blue-900/30 text-blue-400'
                    : 'bg-purple-900/30 text-purple-400'
                }`}
              >
                {solver.supportedAssets.type}
              </span>
              <p className="text-slate-600 dark:text-slate-400 text-xs">
                ({solver.supportedAssets.source})
              </p>
            </div>
          </div>
        </div>
      </div>

      {/* Supported Assets/Routes */}
      {solver.supportedAssets.type === 'assets' &&
        solver.supportedAssets.assets && (
          <div className="card py-4">
            <h3 className="text-lg font-semibold text-slate-900 dark:text-white mb-3">
              Supported Assets ({solver.supportedAssets.assets.length})
            </h3>
            <p className="text-slate-600 dark:text-slate-400 text-sm mb-4">
              This solver supports any-to-any transfers within the following
              asset list.
            </p>

            {Array.from(groupAssetsByChain(solver.supportedAssets.assets)).map(
              ([chainId, assets]) => (
                <div key={chainId} className="mb-4 last:mb-0">
                  <h4 className="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-2 flex items-center gap-2">
                    <span className="bg-primary-900/30 text-primary-400 px-2 py-0.5 rounded text-xs">
                      {getChainName(chainId)}
                    </span>
                    <span className="text-slate-500 text-xs">
                      ({assets.length} asset{assets.length !== 1 ? 's' : ''})
                    </span>
                  </h4>
                  <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-3 border border-slate-300 dark:border-slate-700">
                    <div className="space-y-2">
                      {assets.map((asset, idx) => (
                        <div
                          key={idx}
                          className="flex items-center justify-between py-2 border-b border-slate-200 dark:border-slate-800 last:border-0"
                        >
                          <div className="flex items-center gap-3">
                            <div className="bg-slate-200 dark:bg-slate-800 rounded-full w-8 h-8 flex items-center justify-center text-xs font-bold text-primary-600 dark:text-primary-400">
                              {asset.symbol?.slice(0, 2)}
                            </div>
                            <div>
                              <p className="text-slate-900 dark:text-white font-medium">
                                {asset.symbol}
                              </p>
                              <p className="text-slate-600 dark:text-slate-400 text-xs">
                                {asset.name}
                              </p>
                            </div>
                          </div>
                          <div className="text-right">
                            <p className="text-slate-700 dark:text-slate-300 text-xs font-mono">
                              {asset.address.slice(0, 10)}...
                              {asset.address.slice(-8)}
                            </p>
                            <p className="text-slate-500 dark:text-slate-500 text-xs">
                              Decimals: {asset.decimals}
                            </p>
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>
                </div>
              )
            )}
          </div>
        )}

      {solver.supportedAssets.type === 'routes' &&
        solver.supportedAssets.routes && (
          <div className="card py-4">
            <h3 className="text-lg font-semibold text-slate-900 dark:text-white mb-3">
              Supported Routes ({solver.supportedAssets.routes.length})
            </h3>
            <p className="text-slate-600 dark:text-slate-400 text-sm mb-4">
              This solver supports specific origin to destination routes.
            </p>

            {Array.from(
              groupRoutesByOriginChain(solver.supportedAssets.routes)
            ).map(([originChainId, routes]) => (
              <div key={originChainId} className="mb-4 last:mb-0">
                <h4 className="text-sm font-semibold text-slate-700 dark:text-slate-300 mb-2 flex items-center gap-2">
                  <span className="bg-primary-900/30 text-primary-400 px-2 py-0.5 rounded text-xs">
                    From {getChainName(originChainId)}
                  </span>
                  <span className="text-slate-500 text-xs">
                    ({routes.length} route{routes.length !== 1 ? 's' : ''})
                  </span>
                </h4>
                <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-3 border border-slate-300 dark:border-slate-700">
                  <div className="space-y-3">
                    {routes.map((route, idx) => (
                      <div
                        key={idx}
                        className="flex items-center gap-3 py-2 border-b border-slate-200 dark:border-slate-800 last:border-0"
                      >
                        <div className="flex-1">
                          <div className="flex items-center gap-2 mb-1">
                            <span className="text-slate-900 dark:text-white font-medium text-sm">
                              {route.originTokenSymbol || 'Unknown'}
                            </span>
                            <span className="text-slate-500 dark:text-slate-500">
                              →
                            </span>
                            <span className="text-slate-900 dark:text-white font-medium text-sm">
                              {route.destinationTokenSymbol || 'Unknown'}
                            </span>
                          </div>
                          <div className="flex items-center gap-2 text-xs">
                            <span className="text-slate-600 dark:text-slate-400 font-mono">
                              {route.originTokenAddress.slice(0, 8)}...
                            </span>
                            <span className="text-slate-500 dark:text-slate-500">
                              →
                            </span>
                            <span className="text-slate-600 dark:text-slate-400 font-mono">
                              {route.destinationTokenAddress.slice(0, 8)}...
                            </span>
                          </div>
                        </div>
                        <div className="text-right">
                          <span className="bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400 px-2 py-0.5 rounded text-xs">
                            to {getChainName(route.destinationChainId)}
                          </span>
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
    </div>
  );
}
