import { useEffect, useState } from 'react';

import type { SolverResponse } from '../types/api';
import { solverApi } from '../services/api';

interface SolversListProps {
  onSelectSolver: (solverId: string) => void;
}

export default function SolversList({ onSelectSolver }: SolversListProps) {
  const [solvers, setSolvers] = useState<SolverResponse[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string>('');
  const [totalSolvers, setTotalSolvers] = useState(0);

  useEffect(() => {
    fetchSolvers();
  }, []);

  const fetchSolvers = async () => {
    setIsLoading(true);
    setError('');
    try {
      const response = await solverApi.getSolvers();
      setSolvers(response.solvers);
      setTotalSolvers(response.totalSolvers);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch solvers');
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

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'active':
        return '✓';
      case 'inactive':
        return '⏸';
      case 'circuit-breaker-open':
        return '⚠';
      default:
        return '?';
    }
  };

  const getSupportedAssetsCount = (solver: SolverResponse): string => {
    if (solver.supportedAssets.type === 'assets') {
      return `${solver.supportedAssets.assets?.length || 0} assets`;
    } else {
      return `${solver.supportedAssets.routes?.length || 0} routes`;
    }
  };

  const getSupportedChainsCount = (solver: SolverResponse): number => {
    const chains = new Set<number>();
    if (solver.supportedAssets.type === 'assets') {
      solver.supportedAssets.assets?.forEach((asset) =>
        chains.add(asset.chainId)
      );
    } else {
      solver.supportedAssets.routes?.forEach((route) => {
        chains.add(route.originChainId);
        chains.add(route.destinationChainId);
      });
    }
    return chains.size;
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="text-center">
          <div className="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-primary-500 mb-4"></div>
          <p className="text-slate-600 dark:text-slate-400">
            Loading solvers...
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
            ⚠️ Error Loading Solvers
          </p>
          <p className="text-red-300 text-sm">{error}</p>
        </div>
        <button onClick={fetchSolvers} className="btn-secondary">
          Try Again
        </button>
      </div>
    );
  }

  if (solvers.length === 0) {
    return (
      <div className="card py-4">
        <p className="text-slate-600 dark:text-slate-400 text-center">
          No solvers available
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="card py-4">
        <div className="flex items-center justify-between mb-4">
          <div>
            <h2 className="text-2xl font-bold text-slate-900 dark:text-white">
              Solvers
            </h2>
            <p className="text-slate-600 dark:text-slate-400 text-sm mt-1">
              {totalSolvers} solver{totalSolvers !== 1 ? 's' : ''} available
            </p>
          </div>
          <button
            onClick={fetchSolvers}
            className="btn-secondary text-sm px-3 py-1.5"
            title="Refresh solvers list"
          >
            🔄 Refresh
          </button>
        </div>

        <div className="overflow-x-auto">
          <table className="w-full">
            <thead>
              <tr className="border-b border-slate-300 dark:border-slate-700">
                <th className="text-left py-3 px-4 text-sm font-semibold text-slate-700 dark:text-slate-300">
                  Status
                </th>
                <th className="text-left py-3 px-4 text-sm font-semibold text-slate-700 dark:text-slate-300">
                  Solver
                </th>
                <th className="text-left py-3 px-4 text-sm font-semibold text-slate-700 dark:text-slate-300">
                  Adapter
                </th>
                <th className="text-left py-3 px-4 text-sm font-semibold text-slate-700 dark:text-slate-300">
                  Coverage
                </th>
                <th className="text-left py-3 px-4 text-sm font-semibold text-slate-700 dark:text-slate-300">
                  Type
                </th>
                <th className="text-left py-3 px-4 text-sm font-semibold text-slate-700 dark:text-slate-300">
                  Last Seen
                </th>
                <th className="text-left py-3 px-4 text-sm font-semibold text-slate-700 dark:text-slate-300">
                  Actions
                </th>
              </tr>
            </thead>
            <tbody>
              {solvers.map((solver) => (
                <tr
                  key={solver.solverId}
                  className="border-b border-slate-200 dark:border-slate-700/50 hover:bg-slate-100 dark:hover:bg-slate-800/30 transition-colors"
                >
                  <td className="py-3 px-4">
                    <span
                      className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium border ${getStatusBadgeClass(
                        solver.status
                      )}`}
                    >
                      <span className="text-sm">
                        {getStatusIcon(solver.status)}
                      </span>
                      {solver.status}
                    </span>
                  </td>
                  <td className="py-3 px-4">
                    <div>
                      <p className="text-slate-900 dark:text-white font-medium">
                        {solver.name || solver.solverId}
                      </p>
                      {solver.name && (
                        <p className="text-slate-600 dark:text-slate-400 text-xs font-mono">
                          {solver.solverId}
                        </p>
                      )}
                      {solver.description && (
                        <p className="text-slate-600 dark:text-slate-400 text-xs mt-1 max-w-xs truncate">
                          {solver.description}
                        </p>
                      )}
                    </div>
                  </td>
                  <td className="py-3 px-4">
                    <p className="text-slate-700 dark:text-slate-300 text-sm font-mono">
                      {solver.adapterId}
                    </p>
                  </td>
                  <td className="py-3 px-4">
                    <div className="text-sm">
                      <p className="text-slate-700 dark:text-slate-300">
                        {getSupportedAssetsCount(solver)}
                      </p>
                      <p className="text-slate-600 dark:text-slate-400 text-xs">
                        {getSupportedChainsCount(solver)} chain
                        {getSupportedChainsCount(solver) !== 1 ? 's' : ''}
                      </p>
                    </div>
                  </td>
                  <td className="py-3 px-4">
                    <span
                      className={`text-xs px-2 py-1 rounded ${
                        solver.supportedAssets.type === 'assets'
                          ? 'bg-blue-900/30 text-blue-400'
                          : 'bg-purple-900/30 text-purple-400'
                      }`}
                    >
                      {solver.supportedAssets.type}
                    </span>
                  </td>
                  <td className="py-3 px-4">
                    <p className="text-slate-600 dark:text-slate-400 text-xs">
                      {solver.lastSeen
                        ? new Date(solver.lastSeen).toLocaleString()
                        : 'Never'}
                    </p>
                  </td>
                  <td className="py-3 px-4">
                    <button
                      onClick={() => onSelectSolver(solver.solverId)}
                      className="text-primary-600 dark:text-primary-400 hover:text-primary-700 dark:hover:text-primary-300 text-sm font-medium transition-colors"
                    >
                      View Details →
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
