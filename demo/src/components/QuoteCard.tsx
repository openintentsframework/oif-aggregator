import type { QuoteResponse } from '../types/api';
import { formatInteropAddress } from '../utils/interopAddress';

interface QuoteCardProps {
  quote: QuoteResponse;
  onSelect: (quote: QuoteResponse) => void;
}

export default function QuoteCard({ quote, onSelect }: QuoteCardProps) {
  const formatTimestamp = (timestamp?: number) => {
    if (!timestamp) return 'N/A';
    return new Date(timestamp * 1000).toLocaleString();
  };

  const formatAmount = (amount?: string) => {
    if (!amount) return 'N/A';
    // For display, convert to a more readable format
    try {
      const num = BigInt(amount);
      return num.toString();
    } catch {
      return amount;
    }
  };

  return (
    <div className="card hover:border-primary-500 transition-all cursor-pointer" onClick={() => onSelect(quote)}>
      {/* Header */}
      <div className="flex justify-between items-start mb-4">
        <div>
          <h3 className="text-lg font-semibold text-slate-900 dark:text-white">{quote.provider || 'Unknown Provider'}</h3>
          <p className="text-sm text-slate-600 dark:text-slate-400">Solver: {quote.solverId}</p>
        </div>
        <div className="text-right">
          {quote.eta && (
            <p className="text-sm text-slate-600 dark:text-slate-400">
              ETA: <span className="text-primary-600 dark:text-primary-400 font-medium">{quote.eta}s</span>
            </p>
          )}
        </div>
      </div>

      {/* Preview - Inputs */}
      <div className="mb-3">
        <h4 className="text-sm font-medium text-slate-700 dark:text-slate-300 mb-2">Inputs</h4>
        <div className="space-y-1">
          {quote.preview.inputs.map((input, idx) => (
            <div key={idx} className="bg-slate-100 dark:bg-slate-900 rounded p-2 text-sm border border-slate-300 dark:border-slate-700">
              <p className="text-slate-600 dark:text-slate-400">Amount: <span className="text-slate-900 dark:text-white">{formatAmount(input.amount)}</span></p>
              <p className="text-slate-500 dark:text-slate-500 text-xs truncate">
                Asset: {formatInteropAddress(input.asset)}
              </p>
            </div>
          ))}
        </div>
      </div>

      {/* Preview - Outputs */}
      <div className="mb-4">
        <h4 className="text-sm font-medium text-slate-700 dark:text-slate-300 mb-2">Outputs</h4>
        <div className="space-y-1">
          {quote.preview.outputs.map((output, idx) => (
            <div key={idx} className="bg-slate-100 dark:bg-slate-900 rounded p-2 text-sm border border-slate-300 dark:border-slate-700">
              <p className="text-slate-600 dark:text-slate-400">Amount: <span className="text-slate-900 dark:text-white">{formatAmount(output.amount)}</span></p>
              <p className="text-slate-500 dark:text-slate-500 text-xs truncate">
                Asset: {formatInteropAddress(output.asset)}
              </p>
            </div>
          ))}
        </div>
      </div>

      {/* Footer Info */}
      <div className="border-t border-slate-300 dark:border-slate-700 pt-3 space-y-1">
        {quote.validUntil && (
          <p className="text-xs text-slate-600 dark:text-slate-400">
            Valid Until: {formatTimestamp(quote.validUntil)}
          </p>
        )}
        {quote.failureHandling && (
          <p className="text-xs text-slate-600 dark:text-slate-400">
            Failure Handling: <span className="text-slate-700 dark:text-slate-300">{quote.failureHandling}</span>
          </p>
        )}
        <p className="text-xs text-slate-600 dark:text-slate-400">
          Partial Fill: <span className="text-slate-700 dark:text-slate-300">{quote.partialFill ? 'Yes' : 'No'}</span>
        </p>
        <p className="text-xs text-slate-500 dark:text-slate-500 truncate">
          Quote ID: {quote.quoteId}
        </p>
      </div>

      {/* Select Button */}
      <button
        onClick={(e) => {
          e.stopPropagation();
          onSelect(quote);
        }}
        className="btn-primary w-full mt-4"
      >
        Select This Quote
      </button>
    </div>
  );
}

