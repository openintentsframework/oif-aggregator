import { useEffect, useState } from 'react';
import type { QuoteRequest } from '../types/api';
import { localStorageService, type StoredSearch } from '../services/localStorageService';
import { formatInteropAddress } from '../utils/interopAddress';

interface RecentSearchesModalProps {
  isOpen: boolean;
  onClose: () => void;
  onLoadSearch: (request: QuoteRequest) => void;
}

export default function RecentSearchesModal({ isOpen, onClose, onLoadSearch }: RecentSearchesModalProps) {
  const [searches, setSearches] = useState<StoredSearch[]>([]);

  useEffect(() => {
    if (isOpen) {
      setSearches(localStorageService.getRecentSearches());
    }
  }, [isOpen]);

  const handleDelete = (index: number) => {
    localStorageService.deleteRecentSearch(index);
    setSearches(localStorageService.getRecentSearches());
  };

  const handleClearAll = () => {
    if (confirm('Are you sure you want to clear all recent searches?')) {
      localStorageService.clearRecentSearches();
      setSearches([]);
    }
  };

  const handleLoadSearch = (request: QuoteRequest) => {
    onLoadSearch(request);
    onClose();
  };

  const formatSearchSummary = (request: QuoteRequest): string => {
    const firstInput = request.intent.inputs[0];
    const firstOutput = request.intent.outputs[0];
    const inputAsset = firstInput ? formatInteropAddress(firstInput.asset) : 'N/A';
    const outputAsset = firstOutput ? formatInteropAddress(firstOutput.asset) : 'N/A';
    const amount = firstInput?.amount || firstOutput?.amount || 'N/A';
    
    return `${inputAsset} → ${outputAsset} (${amount})`;
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
      <div className="bg-white dark:bg-slate-800 rounded-lg shadow-xl max-w-2xl w-full max-h-[80vh] flex flex-col">
        {/* Header */}
        <div className="flex justify-between items-center p-4 border-b border-slate-200 dark:border-slate-700">
          <h3 className="text-lg font-semibold text-slate-900 dark:text-white">
            Recent Searches
          </h3>
          <div className="flex gap-2">
            {searches.length > 0 && (
              <button
                onClick={handleClearAll}
                className="text-sm text-red-600 dark:text-red-400 hover:text-red-700 dark:hover:text-red-300 transition-colors"
              >
                Clear All
              </button>
            )}
            <button
              onClick={onClose}
              className="text-slate-500 dark:text-slate-400 hover:text-slate-700 dark:hover:text-slate-200 transition-colors"
            >
              ✕
            </button>
          </div>
        </div>

        {/* Content */}
        <div className="overflow-y-auto flex-1 p-4">
          {searches.length === 0 ? (
            <div className="text-center py-12">
              <p className="text-slate-600 dark:text-slate-400 text-lg">No recent searches</p>
              <p className="text-slate-500 dark:text-slate-500 text-sm mt-2">
                Your recent quote requests will appear here
              </p>
            </div>
          ) : (
            <div className="space-y-3">
              {searches.map((search, index) => (
                <div
                  key={index}
                  className="bg-slate-50 dark:bg-slate-900 rounded-lg p-4 border border-slate-200 dark:border-slate-700"
                >
                  <div className="flex items-start justify-between gap-4">
                    <div className="flex-1">
                      <p className="text-sm text-slate-600 dark:text-slate-400 mb-1">
                        {new Date(search.timestamp).toLocaleString()}
                      </p>
                      <p className="text-slate-900 dark:text-white font-medium mb-2">
                        {formatSearchSummary(search.request)}
                      </p>
                      <div className="flex flex-wrap gap-2 text-xs">
                        <span className="bg-slate-200 dark:bg-slate-800 text-slate-700 dark:text-slate-300 px-2 py-1 rounded">
                          Type: {search.request.intent.swapType}
                        </span>
                        {search.request.solverOptions?.includeSolvers && search.request.solverOptions.includeSolvers.length > 0 && (
                          <span className="bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-400 px-2 py-1 rounded">
                            {search.request.solverOptions.includeSolvers.length} solver(s)
                          </span>
                        )}
                      </div>
                    </div>
                    <div className="flex flex-col gap-2">
                      <button
                        onClick={() => handleLoadSearch(search.request)}
                        className="btn-primary text-xs py-1 px-3 whitespace-nowrap"
                      >
                        Load Search
                      </button>
                      <button
                        onClick={() => handleDelete(index)}
                        className="text-red-600 dark:text-red-400 hover:text-red-700 dark:hover:text-red-300 text-xs"
                        title="Delete this search"
                      >
                        🗑 Delete
                      </button>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="p-4 border-t border-slate-200 dark:border-slate-700">
          <button onClick={onClose} className="btn-secondary w-full">
            Close
          </button>
        </div>
      </div>
    </div>
  );
}

