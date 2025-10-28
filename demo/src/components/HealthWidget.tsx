import { useEffect, useState } from 'react';

import type { HealthResponse } from '../types/api';
import { healthApi } from '../services/api';

export default function HealthWidget() {
  const [health, setHealth] = useState<HealthResponse | null>(null);
  const [isExpanded, setIsExpanded] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string>('');

  useEffect(() => {
    fetchHealth();
    // Poll every 30 seconds
    const intervalId = setInterval(fetchHealth, 30000);
    return () => clearInterval(intervalId);
  }, []);

  const fetchHealth = async () => {
    setIsLoading(true);
    setError(''); // Clear any previous errors

    try {
      const response = await healthApi.getHealth();
      setHealth(response);
      setIsLoading(false);
    } catch (err) {
      // Handle network/API errors
      setError(err instanceof Error ? err.message : 'Failed to fetch health');
      setIsLoading(false);
    }
  };

  const getStatusColor = (status: string) => {
    switch (status.toLowerCase()) {
      case 'healthy':
        return 'bg-green-500';
      case 'degraded':
        return 'bg-yellow-500';
      case 'unhealthy':
        return 'bg-red-500';
      default:
        return 'bg-gray-500';
    }
  };

  const getStatusTextColor = (status: string) => {
    switch (status.toLowerCase()) {
      case 'healthy':
        return 'text-green-400';
      case 'degraded':
        return 'text-yellow-400';
      case 'unhealthy':
        return 'text-red-400';
      default:
        return 'text-gray-400';
    }
  };

  if (isLoading) {
    return (
      <div className="fixed z-50 p-3 bg-white border rounded-lg shadow-lg bottom-16 right-4 dark:bg-slate-800 border-slate-200 dark:border-slate-700">
        <div className="flex items-center gap-2">
          <div className="w-3 h-3 border-2 rounded-full animate-spin border-primary-500 border-t-transparent"></div>
          <span className="text-xs text-slate-600 dark:text-slate-400">Loading...</span>
        </div>
      </div>
    );
  }

  if (error && !health) {
    return (
      <div className="fixed z-50 p-3 bg-white border border-red-300 rounded-lg shadow-lg bottom-16 right-4 dark:bg-slate-800 dark:border-red-700">
        <div className="flex items-center gap-2">
          <div className="w-2 h-2 bg-red-500 rounded-full"></div>
          <span className="text-xs text-red-600 dark:text-red-400">Health check failed</span>
        </div>
      </div>
    );
  }

  if (!health) return null;

  const isUnhealthy = health.status.toLowerCase() === 'unhealthy';

  return (
    <div
      className={`fixed bottom-16 right-4 bg-white dark:bg-slate-800 border rounded-lg shadow-lg z-50 transition-all ${
        isExpanded ? 'w-80' : 'w-auto'
      } ${
        isUnhealthy
          ? 'border-red-300 dark:border-red-700'
          : 'border-slate-200 dark:border-slate-700'
      }`}
    >
      {/* Collapsed View */}
      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className="flex items-center justify-between w-full p-3 transition-colors rounded-lg hover:bg-slate-100 dark:hover:bg-slate-700/30"
      >
        <div className="flex items-center gap-2">
          <div className={`w-2 h-2 rounded-full ${getStatusColor(health.status)} animate-pulse`}></div>
          <span className={`text-xs font-medium ${getStatusTextColor(health.status)}`}>
            {health.status.toUpperCase()}
          </span>
          <span className="text-xs text-slate-400 dark:text-slate-500">•</span>
          <span className="text-xs text-slate-600 dark:text-slate-400">{health.solvers.active}/{health.solvers.total} solvers</span>
        </div>
        <span className="text-xs text-slate-400 dark:text-slate-500">{isExpanded ? '▼' : '▲'}</span>
      </button>

      {/* Expanded View */}
      {isExpanded && (
        <div className="px-3 pb-3 border-t border-slate-200 dark:border-slate-700">
          <div className="pt-3 space-y-2">
            {/* Version */}
            <div className="flex items-center justify-between">
              <span className="text-xs text-slate-600 dark:text-slate-400">Version</span>
              <span className="font-mono text-xs text-slate-900 dark:text-white">{health.version}</span>
            </div>

            {/* Solver Stats */}
            <div className="pt-2 border-t border-slate-200 dark:border-slate-700">
              <p className="mb-2 text-xs font-semibold text-slate-700 dark:text-slate-300">Solvers</p>
              <div className="space-y-1">
                <div className="flex items-center justify-between">
                  <span className="text-xs text-slate-600 dark:text-slate-400">Total</span>
                  <span className="text-xs text-slate-900 dark:text-white">{health.solvers.total}</span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="flex items-center gap-1 text-xs text-slate-600 dark:text-slate-400">
                    <span className="w-2 h-2 bg-green-500 rounded-full"></span>
                    Active
                  </span>
                  <span className="text-xs text-green-600 dark:text-green-400">{health.solvers.active}</span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="flex items-center gap-1 text-xs text-slate-600 dark:text-slate-400">
                    <span className="w-2 h-2 bg-gray-400 rounded-full dark:bg-gray-500"></span>
                    Inactive
                  </span>
                  <span className="text-xs text-slate-600 dark:text-slate-400">{health.solvers.inactive}</span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-xs text-slate-600 dark:text-slate-400">Healthy</span>
                  <span className="text-xs text-green-600 dark:text-green-400">{health.solvers.healthy}</span>
                </div>
                {health.solvers.unhealthy > 0 && (
                  <div className="flex items-center justify-between">
                    <span className="text-xs text-slate-600 dark:text-slate-400">Unhealthy</span>
                    <span className="text-xs text-red-600 dark:text-red-400">{health.solvers.unhealthy}</span>
                  </div>
                )}
              </div>
            </div>

            {/* Storage */}
            <div className="pt-2 border-t border-slate-200 dark:border-slate-700">
              <p className="mb-2 text-xs font-semibold text-slate-700 dark:text-slate-300">Storage</p>
              <div className="space-y-1">
                <div className="flex items-center justify-between">
                  <span className="text-xs text-slate-600 dark:text-slate-400">Backend</span>
                  <span className="font-mono text-xs text-slate-900 dark:text-white">{health.storage.backend}</span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-xs text-slate-600 dark:text-slate-400">Status</span>
                  <span className={`text-xs ${health.storage.healthy ? 'text-green-600 dark:text-green-400' : 'text-red-600 dark:text-red-400'}`}>
                    {health.storage.healthy ? '✓ Healthy' : '✗ Unhealthy'}
                  </span>
                </div>
              </div>
            </div>

            {/* Refresh */}
            <div className="pt-2 border-t border-slate-200 dark:border-slate-700">
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  fetchHealth();
                }}
                className="w-full text-xs transition-colors text-primary-600 dark:text-primary-400 hover:text-primary-700 dark:hover:text-primary-300"
              >
                🔄 Refresh
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

