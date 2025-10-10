import type { QuoteResponse, QuotesResponse } from '../types/api';

import QuoteCard from './QuoteCard';
import { useState } from 'react';

interface QuoteResultsProps {
  response: QuotesResponse;
  onSelectQuote: (quote: QuoteResponse) => void;
  onBack: () => void;
  onRefresh: () => void;
  isRefreshing: boolean;
}

type SortBy = 'default' | 'eta' | 'solver';

export default function QuoteResults({ response, onSelectQuote, onBack, onRefresh, isRefreshing }: QuoteResultsProps) {
  const [sortBy, setSortBy] = useState<SortBy>('default');

  const sortedQuotes = [...response.quotes].sort((a, b) => {
    switch (sortBy) {
      case 'eta':
        return (a.eta || Infinity) - (b.eta || Infinity);
      case 'solver':
        return a.solverId.localeCompare(b.solverId);
      default:
        return 0;
    }
  });

  return (
    <div className="space-y-4 max-w-4xl mx-auto">
      {/* Header */}
      <div className="card">
        <div className="flex justify-between items-center mb-4">
          <h2 className="text-2xl font-bold text-slate-900 dark:text-white">
            Quote Results ({response.totalQuotes})
          </h2>
          <div className="flex gap-2">
            <button 
              onClick={onRefresh} 
              disabled={isRefreshing}
              className="btn-secondary"
              title="Refresh quotes with the same parameters"
            >
              {isRefreshing ? '⟳ Refreshing...' : '🔄 Refresh Quotes'}
            </button>
            <button onClick={onBack} className="btn-secondary">
              ← Back to Request
            </button>
          </div>
        </div>

        {/* Aggregation Metadata */}
        {response.metadata && (
          <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-4 grid grid-cols-2 md:grid-cols-4 gap-3 border border-slate-300 dark:border-slate-700">
            <div>
              <p className="text-xs text-slate-600 dark:text-slate-400">Duration</p>
              <p className="text-slate-900 dark:text-white font-medium">{response.metadata.totalDurationMs}ms</p>
            </div>
            <div>
              <p className="text-xs text-slate-600 dark:text-slate-400">Solvers Queried</p>
              <p className="text-slate-900 dark:text-white font-medium">{response.metadata.solversQueried}</p>
            </div>
            <div>
              <p className="text-xs text-slate-600 dark:text-slate-400">Success</p>
              <p className="text-green-600 dark:text-green-400 font-medium">{response.metadata.solversRespondedSuccess}</p>
            </div>
            <div>
              <p className="text-xs text-slate-600 dark:text-slate-400">Errors/Timeout</p>
              <p className="text-red-600 dark:text-red-400 font-medium">
                {response.metadata.solversRespondedError + response.metadata.solversTimedOut}
              </p>
            </div>
            <div>
              <p className="text-xs text-slate-600 dark:text-slate-400">Selection Mode</p>
              <p className="text-slate-900 dark:text-white font-medium">{response.metadata.solverSelectionMode}</p>
            </div>
            <div>
              <p className="text-xs text-slate-600 dark:text-slate-400">Early Termination</p>
              <p className="text-slate-900 dark:text-white font-medium">{response.metadata.earlyTermination ? 'Yes' : 'No'}</p>
            </div>
            <div>
              <p className="text-xs text-slate-600 dark:text-slate-400">Solver Timeout</p>
              <p className="text-slate-900 dark:text-white font-medium">{response.metadata.solverTimeoutMs}ms</p>
            </div>
            <div>
              <p className="text-xs text-slate-600 dark:text-slate-400">Global Timeout</p>
              <p className="text-slate-900 dark:text-white font-medium">{response.metadata.globalTimeoutMs}ms</p>
            </div>
          </div>
        )}

        {/* Sort Options */}
        <div className="mt-4 flex items-center gap-3">
          <label className="text-sm text-slate-600 dark:text-slate-400">Sort by:</label>
          <select
            value={sortBy}
            onChange={(e) => setSortBy(e.target.value as SortBy)}
            className="input-field py-1 text-sm"
          >
            <option value="default">Default</option>
            <option value="eta">Fastest (ETA)</option>
            <option value="solver">Solver Name</option>
          </select>
        </div>
      </div>

      {/* Quotes Grid */}
      {sortedQuotes.length === 0 ? (
        <div className="card text-center py-12">
          <p className="text-slate-600 dark:text-slate-400 text-lg">No quotes available</p>
          <p className="text-slate-500 dark:text-slate-500 text-sm mt-2">Try adjusting your request parameters</p>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {sortedQuotes.map((quote) => (
            <QuoteCard key={quote.quoteId} quote={quote} onSelect={onSelectQuote} />
          ))}
        </div>
      )}
    </div>
  );
}

