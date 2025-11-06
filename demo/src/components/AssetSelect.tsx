import { useMemo, useState } from 'react';

import type { AssetInfo } from '../types/solverData';

interface AssetSelectProps {
  assets: AssetInfo[];
  value: AssetInfo | null;
  onChange: (asset: AssetInfo | null) => void;
  placeholder?: string;
  disabled?: boolean;
  showChain?: boolean;
}

// SVG Copy Icon Component
function CopyIcon({ className = "w-4 h-4" }: { className?: string }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      fill="none"
      viewBox="0 0 24 24"
      strokeWidth={1.5}
      stroke="currentColor"
      className={className}
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M15.666 3.888A2.25 2.25 0 0 0 13.5 2.25h-3c-1.03 0-1.9.693-2.166 1.638m7.332 0c.055.194.084.4.084.612v0a.75.75 0 0 1-.75.75H9a.75.75 0 0 1-.75-.75v0c0-.212.03-.418.084-.612m7.332 0c.646.049 1.288.11 1.927.184 1.1.128 1.907 1.077 1.907 2.185V19.5a2.25 2.25 0 0 1-2.25 2.25H6.75A2.25 2.25 0 0 1 4.5 19.5V6.257c0-1.108.806-2.057 1.907-2.185a48.208 48.208 0 0 1 1.927-.184"
      />
    </svg>
  );
}

// SVG Check Icon Component
function CheckIcon({ className = "w-4 h-4" }: { className?: string }) {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      fill="none"
      viewBox="0 0 24 24"
      strokeWidth={1.5}
      stroke="currentColor"
      className={className}
    >
      <path
        strokeLinecap="round"
        strokeLinejoin="round"
        d="M4.5 12.75l6 6 9-13.5"
      />
    </svg>
  );
}

export default function AssetSelect({
  assets,
  value,
  onChange,
  placeholder = 'Select asset',
  disabled = false,
  showChain = false,
}: AssetSelectProps) {
  const [searchTerm, setSearchTerm] = useState('');
  const [isOpen, setIsOpen] = useState(false);
  const [copiedAddress, setCopiedAddress] = useState<string | null>(null);

  const filteredAssets = useMemo(() => {
    if (!searchTerm) return assets;
    const term = searchTerm.toLowerCase();
    return assets.filter(
      (asset) =>
        asset.symbol.toLowerCase().includes(term) ||
        asset.name.toLowerCase().includes(term) ||
        asset.address.toLowerCase().includes(term)
    );
  }, [assets, searchTerm]);

  const handleSelect = (asset: AssetInfo) => {
    onChange(asset);
    setIsOpen(false);
    setSearchTerm('');
  };

  const handleCopyAddress = async (address: string, e: React.MouseEvent) => {
    e.stopPropagation(); // Prevent triggering the select when clicking copy
    try {
      await navigator.clipboard.writeText(address);
      setCopiedAddress(address);
      setTimeout(() => setCopiedAddress(null), 2000); // Reset after 2 seconds
    } catch (err) {
      console.error('Failed to copy address:', err);
    }
  };

  // Simple dropdown for now, can be enhanced to a more sophisticated component later
  return (
    <div className="relative">
      {isOpen && !disabled ? (
        <div className="absolute inset-0 z-50">
          <input
            type="text"
            value={searchTerm}
            onChange={(e) => setSearchTerm(e.target.value)}
            placeholder="Search assets..."
            className="input-field mb-1"
            autoFocus
            onBlur={() => {
              // Delay to allow clicking on options
              setTimeout(() => setIsOpen(false), 200);
            }}
          />
          <div className="bg-white dark:bg-slate-800 border-2 border-slate-300 dark:border-slate-700 rounded-lg max-h-60 overflow-y-auto shadow-lg">
            {filteredAssets.length === 0 ? (
              <div className="p-3 text-slate-600 dark:text-slate-400 text-sm">No assets found</div>
            ) : (
              filteredAssets.map((asset) => (
                <div
                  key={`${asset.chainId}-${asset.address}`}
                  className="flex items-center gap-2 px-3 py-2.5 hover:bg-slate-100 dark:hover:bg-slate-700 transition-colors border-b border-slate-200 dark:border-slate-700 last:border-0"
                >
                  <button
                    type="button"
                    onClick={() => handleSelect(asset)}
                    className="flex-1 flex items-center gap-2 min-w-0 text-left"
                  >
                    <div className="flex items-center gap-2 flex-shrink-0">
                      <span className="text-slate-900 dark:text-white font-medium">{asset.symbol}</span>
                      {showChain && (
                        <span className="text-slate-500 dark:text-slate-500 text-xs">Chain {asset.chainId}</span>
                      )}
                    </div>
                    <span className="text-slate-500 dark:text-slate-500 text-xs font-mono truncate">
                      {asset.address.slice(0, 4)}...{asset.address.slice(-4)}
                    </span>
                  </button>
                  <button
                    type="button"
                    onClick={(e) => handleCopyAddress(asset.address, e)}
                    className="flex-shrink-0 p-1 rounded hover:bg-slate-200 dark:hover:bg-slate-600 transition-colors group"
                    title="Copy address"
                  >
                    {copiedAddress === asset.address ? (
                      <CheckIcon className="w-3 h-3 text-green-600 dark:text-green-400" />
                    ) : (
                      <CopyIcon className="w-3 h-3 text-slate-400 group-hover:text-slate-600 dark:group-hover:text-slate-200" />
                    )}
                  </button>
                </div>
              ))
            )}
          </div>
        </div>
      ) : (
        <div className="input-field text-left flex items-center justify-between gap-2">
          <button
            type="button"
            onClick={() => !disabled && setIsOpen(true)}
            disabled={disabled}
            className="flex-1 flex items-center gap-2 min-w-0"
          >
            {value ? (
              <>
                <div className="flex items-center gap-2 flex-shrink-0">
                  <span className="text-slate-900 dark:text-white font-medium">{value.symbol}</span>
                  {showChain && (
                    <span className="text-slate-500 dark:text-slate-500 text-xs">Chain {value.chainId}</span>
                  )}
                </div>
                <span className="text-slate-500 dark:text-slate-500 text-xs font-mono truncate">
                  {value.address.slice(0, 4)}...{value.address.slice(-4)}
                </span>
              </>
            ) : (
              <span className="text-slate-500 dark:text-slate-400">{placeholder}</span>
            )}
          </button>
          {value && (
            <button
              type="button"
              onClick={(e) => handleCopyAddress(value.address, e)}
              className="p-1.5 rounded hover:bg-slate-200 dark:hover:bg-slate-600 transition-colors group flex-shrink-0"
              title="Copy address"
            >
              {copiedAddress === value.address ? (
                <CheckIcon className="w-4 h-4 text-green-600 dark:text-green-400" />
              ) : (
                <CopyIcon className="w-4 h-4 text-slate-400 group-hover:text-slate-600 dark:group-hover:text-slate-200" />
              )}
            </button>
          )}
          <button
            type="button"
            onClick={() => !disabled && setIsOpen(true)}
            disabled={disabled}
            className="flex-shrink-0 text-slate-500 dark:text-slate-400"
          >
            ▼
          </button>
        </div>
      )}
    </div>
  );
}

