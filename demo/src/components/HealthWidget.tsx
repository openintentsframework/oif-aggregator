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
    try {
      const response = await healthApi.getHealth();
      setHealth(response);
      setError('');
      setIsLoading(false);
    } catch (err) {
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
      <div className="fixed bottom-16 right-4 bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 rounded-lg shadow-lg p-3 z-50">
        <div className="flex items-center gap-2">
          <div className="animate-spin rounded-full h-3 w-3 border-2 border-primary-500 border-t-transparent"></div>
          <span className="text-slate-600 dark:text-slate-400 text-xs">Loading...</span>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="fixed bottom-16 right-4 bg-white dark:bg-slate-800 border border-red-300 dark:border-red-700 rounded-lg shadow-lg p-3 z-50">
        <div className="flex items-center gap-2">
          <div className="w-2 h-2 rounded-full bg-red-500"></div>
          <span className="text-red-600 dark:text-red-400 text-xs">Health check failed</span>
        </div>
      </div>
    );
  }

  if (!health) return null;

  return (
    <div
      className={`fixed bottom-16 right-4 bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 rounded-lg shadow-lg z-50 transition-all ${
        isExpanded ? 'w-80' : 'w-auto'
      }`}
    >
      {/* Collapsed View */}
      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className="w-full p-3 flex items-center justify-between hover:bg-slate-100 dark:hover:bg-slate-700/30 transition-colors rounded-lg"
      >
        <div className="flex items-center gap-2">
          <div className={`w-2 h-2 rounded-full ${getStatusColor(health.status)} animate-pulse`}></div>
          <span className={`text-xs font-medium ${getStatusTextColor(health.status)}`}>
            {health.status.toUpperCase()}
          </span>
          <span className="text-slate-400 dark:text-slate-500 text-xs">•</span>
          <span className="text-slate-600 dark:text-slate-400 text-xs">{health.solvers.active}/{health.solvers.total} solvers</span>
        </div>
        <span className="text-slate-400 dark:text-slate-500 text-xs">{isExpanded ? '▼' : '▲'}</span>
      </button>

      {/* Expanded View */}
      {isExpanded && (
        <div className="px-3 pb-3 border-t border-slate-200 dark:border-slate-700">
          <div className="space-y-2 pt-3">
            {/* Version */}
            <div className="flex items-center justify-between">
              <span className="text-xs text-slate-600 dark:text-slate-400">Version</span>
              <span className="text-xs text-slate-900 dark:text-white font-mono">{health.version}</span>
            </div>

            {/* Solver Stats */}
            <div className="pt-2 border-t border-slate-200 dark:border-slate-700">
              <p className="text-xs font-semibold text-slate-700 dark:text-slate-300 mb-2">Solvers</p>
              <div className="space-y-1">
                <div className="flex items-center justify-between">
                  <span className="text-xs text-slate-600 dark:text-slate-400">Total</span>
                  <span className="text-xs text-slate-900 dark:text-white">{health.solvers.total}</span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-xs text-slate-600 dark:text-slate-400 flex items-center gap-1">
                    <span className="w-2 h-2 rounded-full bg-green-500"></span>
                    Active
                  </span>
                  <span className="text-xs text-green-600 dark:text-green-400">{health.solvers.active}</span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-xs text-slate-600 dark:text-slate-400 flex items-center gap-1">
                    <span className="w-2 h-2 rounded-full bg-gray-400 dark:bg-gray-500"></span>
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
              <p className="text-xs font-semibold text-slate-700 dark:text-slate-300 mb-2">Storage</p>
              <div className="space-y-1">
                <div className="flex items-center justify-between">
                  <span className="text-xs text-slate-600 dark:text-slate-400">Backend</span>
                  <span className="text-xs text-slate-900 dark:text-white font-mono">{health.storage.backend}</span>
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
                className="w-full text-xs text-primary-600 dark:text-primary-400 hover:text-primary-700 dark:hover:text-primary-300 transition-colors"
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

