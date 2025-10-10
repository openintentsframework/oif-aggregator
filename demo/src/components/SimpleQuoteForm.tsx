import type { AssetInfo, SimpleQuoteFormData } from '../types/solverData';
import type { QuoteRequest, SolverOptions as SolverOptionsType } from '../types/api';
import { convertFormToQuoteRequest, validateFormData } from '../utils/formToQuoteRequest';

import AssetSelect from './AssetSelect';
import NetworkSelect from './NetworkSelect';
import RecentSearchesModal from './RecentSearchesModal';
import SolverOptions from './SolverOptions';
import { fromInteropAddress } from '../utils/interopAddress';
import { localStorageService } from '../services/localStorageService';
import { solverDataService } from '../services/solverDataService';
import { useState } from 'react';

interface SimpleQuoteFormProps {
  onSubmit: (request: QuoteRequest) => void;
  isLoading: boolean;
}

export default function SimpleQuoteForm({ onSubmit, isLoading }: SimpleQuoteFormProps) {
  const [formData, setFormData] = useState<SimpleQuoteFormData>({
    amount: '1',
    fromAsset: null,
    fromChain: 1, // Default to Ethereum
    toChain: 10, // Default to Optimism
    toAsset: null,
    userAddress: '',
  });

  const [swapType, setSwapType] = useState<'exact-input' | 'exact-output'>('exact-input');
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [showSolverOptions, setShowSolverOptions] = useState(false);
  const [showRecentSearches, setShowRecentSearches] = useState(false);
  const [validationErrors, setValidationErrors] = useState<string[]>([]);
  
  const [solverOptions, setSolverOptions] = useState<Partial<SolverOptionsType>>({
    timeout: 10000,
    solverTimeout: 5000,
    minQuotes: 1,
    solverSelection: 'all' as const,
  });

  const cache = solverDataService.getCachedData();
  const chains = cache?.chains || [];
  const fromAssets = solverDataService.getAssetsByChain(formData.fromChain);
  
  // Get compatible destinations when from asset changes
  const compatibleDest = formData.fromAsset
    ? solverDataService.getCompatibleDestinations(formData.fromChain, formData.fromAsset.address)
    : { chains: [], assetsByChain: new Map<number, AssetInfo[]>() };
  
  const toChains = compatibleDest.chains.length > 0
    ? chains.filter((c) => compatibleDest.chains.includes(c.chainId))
    : chains;
  
  const toAssets: AssetInfo[] = compatibleDest.assetsByChain.get(formData.toChain) || [];

  // Update form field
  const updateField = <K extends keyof SimpleQuoteFormData>(
    field: K,
    value: SimpleQuoteFormData[K]
  ) => {
    setFormData((prev) => ({ ...prev, [field]: value }));
    setValidationErrors([]); // Clear errors on change
  };

  // Handle from asset change
  const handleFromAssetChange = (asset: AssetInfo | null) => {
    updateField('fromAsset', asset);
    if (asset) {
      // Clear to asset if it's not compatible with the new from asset
      if (formData.toAsset) {
        const compat = solverDataService.getCompatibleDestinations(formData.fromChain, asset.address);
        const hasCompatibleRoute = compat.assetsByChain
          .get(formData.toChain)
          ?.some((a) => a.address.toLowerCase() === formData.toAsset!.address.toLowerCase());
        if (!hasCompatibleRoute) {
          updateField('toAsset', null);
        }
      }
    }
  };

  // Handle to chain change
  const handleToChainChange = (chainId: number) => {
    updateField('toChain', chainId);
    // Clear to asset if chain changes
    updateField('toAsset', null);
  };

  // Swap from/to
  const handleSwap = () => {
    if (formData.fromAsset && formData.toAsset) {
      setFormData({
        ...formData,
        fromAsset: formData.toAsset,
        fromChain: formData.toChain,
        toAsset: formData.fromAsset,
        toChain: formData.fromChain,
      });
    }
  };

  // Handle submit
  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    
    const validation = validateFormData(formData);
    if (!validation.valid) {
      setValidationErrors(validation.errors);
      return;
    }

    try {
      const request = convertFormToQuoteRequest(formData, swapType);
      if (showSolverOptions) {
        request.solverOptions = solverOptions;
      }
      localStorageService.saveRecentSearch(request);
      onSubmit(request);
    } catch (error) {
      setValidationErrors([(error as Error).message]);
    }
  };

  const handleLoadSearch = (request: QuoteRequest) => {
    try {
      // Extract and populate form data from the saved request
      setSwapType(request.intent.swapType || 'exact-input');
      
      // Get first input and output
      const firstInput = request.intent.inputs[0];
      const firstOutput = request.intent.outputs[0];
      
      if (!firstInput || !firstOutput) {
        throw new Error('Invalid request: missing inputs or outputs');
      }
      
      // Parse input
      const inputInterop = fromInteropAddress(firstInput.asset);
      const inputAsset = cache?.assets.get(`${inputInterop.chainId}-${inputInterop.address}`);
      
      // Parse output
      const outputInterop = fromInteropAddress(firstOutput.asset);
      const outputAsset = cache?.assets.get(`${outputInterop.chainId}-${outputInterop.address}`);
      
      setFormData({
        amount: '1', // Reset to default - saved amount is already converted to smallest unit
        fromAsset: inputAsset || null,
        fromChain: inputInterop.chainId,
        toChain: outputInterop.chainId,
        toAsset: outputAsset || null,
        userAddress: fromInteropAddress(firstInput.user).address,
      });
      
      // Load solver options if present
      if (request.solverOptions) {
        setSolverOptions(request.solverOptions);
        setShowSolverOptions(true);
      }
      
      setValidationErrors([]);
    } catch (error) {
      console.error('Failed to load search:', error);
      alert('Failed to load search. The data format may have changed.');
    }
  };

  // Check if route is supported
  const routeSupported = formData.fromAsset && formData.toAsset
    ? solverDataService.getSolversForRoute(
        formData.fromChain,
        formData.fromAsset.address,
        formData.toChain,
        formData.toAsset.address
      ).length > 0
    : true; // Don't show warning if not fully selected

  return (
    <form onSubmit={handleSubmit} className="space-y-4 max-w-2xl mx-auto">
      <div className="card py-4">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-xl font-bold text-slate-900 dark:text-white">Request Quote</h2>
          <button
            type="button"
            onClick={() => setShowRecentSearches(true)}
            className="btn-secondary text-sm py-1 px-3"
            title="View recent searches"
          >
            🕒 Recent Searches
          </button>
        </div>

        {/* Validation Errors */}
        {validationErrors.length > 0 && (
          <div className="bg-red-900/20 border border-red-700 rounded-lg p-3 mb-4">
            {validationErrors.map((error, idx) => (
              <p key={idx} className="text-red-400 text-sm">{error}</p>
            ))}
          </div>
        )}

        {/* Swap Type & Amount */}
        <div className="grid grid-cols-2 gap-4 mb-4">
          <div>
            <label className="label-text">Swap Type</label>
            <div className="flex flex-col gap-2 mt-2">
            <label className="flex items-center">
              <input
                type="radio"
                value="exact-input"
                checked={swapType === 'exact-input'}
                onChange={(e) => setSwapType(e.target.value as 'exact-input')}
                className="mr-2"
              />
              <span className="text-slate-900 dark:text-white">Exact Input</span>
            </label>
            <label className="flex items-center">
              <input
                type="radio"
                value="exact-output"
                checked={swapType === 'exact-output'}
                onChange={(e) => setSwapType(e.target.value as 'exact-output')}
                className="mr-2"
              />
              <span className="text-slate-900 dark:text-white">Exact Output</span>
            </label>
            </div>
          </div>
          <div>
            <label className="label-text">
              Amount {formData.fromAsset && `(${formData.fromAsset.symbol})`}
            </label>
            <input
              type="text"
              value={formData.amount}
              onChange={(e) => updateField('amount', e.target.value)}
              placeholder="1.0"
              className="input-field mt-2"
            />
            {formData.fromAsset && (
              <p className="text-xs text-slate-500 mt-1">
                {formData.fromAsset.decimals} decimals
              </p>
            )}
          </div>
        </div>

        {/* From Section */}
        <div className="mb-4 bg-slate-100 dark:bg-slate-900 rounded-lg p-3 border border-slate-300 dark:border-slate-700">
          <div className="flex items-center justify-between mb-2">
            <h3 className="text-base font-semibold text-slate-900 dark:text-white">From</h3>
            {formData.fromAsset && formData.toAsset && (
              <button
                type="button"
                onClick={handleSwap}
                className="text-primary-600 dark:text-primary-400 hover:text-primary-700 dark:hover:text-primary-300 text-xs"
              >
                ⇅ Swap
              </button>
            )}
          </div>
          <div className="space-y-2">
            <div>
              <label className="label-text">Network</label>
              <NetworkSelect
                chains={chains}
                value={formData.fromChain}
                onChange={(chainId) => {
                  updateField('fromChain', chainId);
                  updateField('fromAsset', null); // Reset asset
                }}
                placeholder="Select from network"
              />
            </div>
            <div>
              <label className="label-text">Asset</label>
              <AssetSelect
                assets={fromAssets}
                value={formData.fromAsset}
                onChange={handleFromAssetChange}
                placeholder="Select from asset"
                showChain
              />
            </div>
          </div>
        </div>

        {/* To Section */}
        <div className="mb-4 bg-slate-100 dark:bg-slate-900 rounded-lg p-3 border border-slate-300 dark:border-slate-700">
          <h3 className="text-base font-semibold text-slate-900 dark:text-white mb-2">To</h3>
          <div className="space-y-2">
            <div>
              <label className="label-text">Network</label>
              <NetworkSelect
                chains={toChains}
                value={formData.toChain}
                onChange={handleToChainChange}
                placeholder="Select to network"
                disabled={!formData.fromAsset}
              />
              {formData.fromAsset && toChains.length === 0 && (
                <p className="text-yellow-400 text-xs mt-1">
                  No compatible destination networks found for this asset
                </p>
              )}
            </div>
            <div>
              <label className="label-text">Asset</label>
              <AssetSelect
                assets={toAssets}
                value={formData.toAsset}
                onChange={(asset) => updateField('toAsset', asset)}
                placeholder="Select to asset"
                disabled={!formData.fromAsset || toAssets.length === 0}
              />
              {formData.fromAsset && formData.toChain && toAssets.length === 0 && (
                <p className="text-yellow-400 text-xs mt-1">
                  No compatible assets found on this network
                </p>
              )}
            </div>
          </div>
        </div>

        {/* Route Warning */}
        {formData.fromAsset && formData.toAsset && !routeSupported && (
          <div className="bg-yellow-900/20 border border-yellow-700 rounded-lg p-3 mb-4">
            <p className="text-yellow-400 text-sm">
              ⚠️ No solvers currently support this route. You may not receive any quotes.
            </p>
          </div>
        )}

        {/* User Address */}
        <div className="mb-4">
          <div className="flex items-center justify-between mb-1">
            <label className="label-text text-sm">User Address</label>
            <span className="text-xs text-slate-500">Wallet connection coming soon</span>
          </div>
          <input
            type="text"
            value={formData.userAddress}
            onChange={(e) => updateField('userAddress', e.target.value)}
            placeholder="0x... (enter manually for now)"
            className="input-field"
          />
          <p className="text-xs text-slate-500 mt-1">
            💡 Future: Click "Connect Wallet" to auto-fill
          </p>
        </div>

        {/* Advanced Options */}
        <div className="mb-3">
          <button
            type="button"
            onClick={() => setShowAdvanced(!showAdvanced)}
            className="flex items-center justify-between w-full text-left text-slate-900 dark:text-white hover:text-primary-600 dark:hover:text-primary-400 transition-colors"
          >
            <span className="text-sm font-medium">Advanced Options</span>
            <span className="text-lg">{showAdvanced ? '−' : '+'}</span>
          </button>
          {showAdvanced && (
            <div className="mt-2 space-y-3 bg-slate-100 dark:bg-slate-900 rounded-lg p-3 border border-slate-300 dark:border-slate-700">
              <div>
                <label className="label-text">Receiver Address (optional)</label>
                <input
                  type="text"
                  value={formData.receiverAddress || ''}
                  onChange={(e) => updateField('receiverAddress', e.target.value)}
                  placeholder="Same as user address"
                  className="input-field"
                />
              </div>
              
              <button
                type="button"
                onClick={() => setShowSolverOptions(!showSolverOptions)}
                className="text-primary-600 dark:text-primary-400 hover:text-primary-700 dark:hover:text-primary-300 text-sm"
              >
                {showSolverOptions ? 'Hide' : 'Show'} Solver Options
              </button>
              {showSolverOptions && (
                <SolverOptions options={solverOptions} onChange={setSolverOptions} />
              )}
            </div>
          )}
        </div>

        {/* Submit Button */}
        <button
          type="submit"
          disabled={isLoading || !formData.fromAsset || !formData.toAsset}
          className="btn-primary w-full py-3 text-lg"
        >
          {isLoading ? 'Getting Quotes...' : 'Get Quotes'}
        </button>
      </div>

      {/* Recent Searches Modal */}
      <RecentSearchesModal
        isOpen={showRecentSearches}
        onClose={() => setShowRecentSearches(false)}
        onLoadSearch={handleLoadSearch}
      />
    </form>
  );
}

