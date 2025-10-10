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
                <button
                  key={`${asset.chainId}-${asset.address}`}
                  type="button"
                  onClick={() => handleSelect(asset)}
                  className="w-full text-left px-3 py-2 hover:bg-slate-100 dark:hover:bg-slate-700 transition-colors border-b border-slate-200 dark:border-slate-700 last:border-0"
                >
                  <div className="flex items-center justify-between">
                    <div>
                      <span className="text-slate-900 dark:text-white font-medium">{asset.symbol}</span>
                      <span className="text-slate-600 dark:text-slate-400 text-sm ml-2">{asset.name}</span>
                    </div>
                    {showChain && (
                      <span className="text-slate-500 dark:text-slate-500 text-xs">Chain {asset.chainId}</span>
                    )}
                  </div>
                  <div className="text-slate-500 dark:text-slate-500 text-xs truncate mt-1">
                    {asset.address}
                  </div>
                </button>
              ))
            )}
          </div>
        </div>
      ) : (
        <button
          type="button"
          onClick={() => !disabled && setIsOpen(true)}
          disabled={disabled}
          className="input-field text-left flex items-center justify-between"
        >
          {value ? (
            <div className="flex items-center justify-between w-full">
              <div>
                <span className="text-slate-900 dark:text-white font-medium">{value.symbol}</span>
                <span className="text-slate-600 dark:text-slate-400 text-sm ml-2">{value.name}</span>
              </div>
              {showChain && (
                <span className="text-slate-500 dark:text-slate-500 text-xs">Chain {value.chainId}</span>
              )}
            </div>
          ) : (
            <span className="text-slate-500 dark:text-slate-400">{placeholder}</span>
          )}
          <span className="text-slate-500 dark:text-slate-400 ml-2">▼</span>
        </button>
      )}
    </div>
  );
}

