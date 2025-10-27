import type {
  Input,
  Output,
  QuoteRequest,
  SolverOptions as SolverOptionsType,
} from '../types/api';
import {
  fromInteropAddress,
  isValidAddress,
  isValidChainId,
  toInteropAddress,
} from '../utils/interopAddress';

import RecentSearchesModal from './RecentSearchesModal';
import SolverOptions from './SolverOptions';
import { localStorageService } from '../services/localStorageService';
import { useForm } from 'react-hook-form';
import { useState } from 'react';

type SolverOptions = SolverOptionsType;

interface QuoteRequestFormProps {
  onSubmit: (request: QuoteRequest) => void;
  isLoading: boolean;
}

interface FormInput {
  userAddress: string;
  userChainId: string;
  asset: string;
  assetChainId: string;
  amount: string;
}

interface FormOutput {
  receiverAddress: string;
  receiverChainId: string;
  asset: string;
  assetChainId: string;
  amount: string;
}

export default function QuoteRequestForm({
  onSubmit,
  isLoading,
}: QuoteRequestFormProps) {
  const { handleSubmit } = useForm();
  const [showSolverOptions, setShowSolverOptions] = useState(false);
  const [showRecentSearches, setShowRecentSearches] = useState(false);
  const [inputs, setInputs] = useState<FormInput[]>([
    {
      userAddress: '',
      userChainId: '1',
      asset: '',
      assetChainId: '1',
      amount: '',
    },
  ]);
  const [outputs, setOutputs] = useState<FormOutput[]>([
    {
      receiverAddress: '',
      receiverChainId: '1',
      asset: '',
      assetChainId: '1',
      amount: '',
    },
  ]);

  const [swapType, setSwapType] = useState<'exact-input' | 'exact-output'>(
    'exact-input'
  );
  const [supportedTypes, setSupportedTypes] = useState<string[]>([
    'oif-escrow-v0',
  ]);

  // Solver options state
  const [solverOptions, setSolverOptions] = useState<Partial<SolverOptions>>({
    timeout: 10000,
    solverTimeout: 5000,
    minQuotes: 1,
    solverSelection: 'all' as const,
    includeSolvers: [] as string[],
    excludeSolvers: [] as string[],
  });

  const addInput = () => {
    setInputs([
      ...inputs,
      {
        userAddress: '',
        userChainId: '1',
        asset: '',
        assetChainId: '1',
        amount: '',
      },
    ]);
  };

  const removeInput = (index: number) => {
    if (inputs.length > 1) {
      setInputs(inputs.filter((_, i) => i !== index));
    }
  };

  const addOutput = () => {
    setOutputs([
      ...outputs,
      {
        receiverAddress: '',
        receiverChainId: '1',
        asset: '',
        assetChainId: '1',
        amount: '',
      },
    ]);
  };

  const removeOutput = (index: number) => {
    if (outputs.length > 1) {
      setOutputs(outputs.filter((_, i) => i !== index));
    }
  };

  const updateInput = (
    index: number,
    field: keyof FormInput,
    value: string
  ) => {
    const newInputs = [...inputs];
    newInputs[index][field] = value;
    setInputs(newInputs);
  };

  const updateOutput = (
    index: number,
    field: keyof FormOutput,
    value: string
  ) => {
    const newOutputs = [...outputs];
    newOutputs[index][field] = value;
    setOutputs(newOutputs);
  };

  const handleFormSubmit = () => {
    try {
      // Validate and convert inputs
      const apiInputs: Input[] = inputs.map((input) => {
        if (!isValidAddress(input.userAddress)) {
          throw new Error(`Invalid user address: ${input.userAddress}`);
        }
        if (!isValidAddress(input.asset)) {
          throw new Error(`Invalid asset address: ${input.asset}`);
        }
        if (!isValidChainId(parseInt(input.userChainId))) {
          throw new Error(`Invalid chain ID: ${input.userChainId}`);
        }

        return {
          user: toInteropAddress(
            input.userAddress,
            parseInt(input.userChainId)
          ),
          asset: toInteropAddress(input.asset, parseInt(input.assetChainId)),
          amount: input.amount || undefined,
        };
      });

      // Validate and convert outputs
      const apiOutputs: Output[] = outputs.map((output) => {
        if (!isValidAddress(output.receiverAddress)) {
          throw new Error(
            `Invalid receiver address: ${output.receiverAddress}`
          );
        }
        if (!isValidAddress(output.asset)) {
          throw new Error(`Invalid output asset address: ${output.asset}`);
        }
        if (!isValidChainId(parseInt(output.receiverChainId))) {
          throw new Error(`Invalid chain ID: ${output.receiverChainId}`);
        }

        return {
          receiver: toInteropAddress(
            output.receiverAddress,
            parseInt(output.receiverChainId)
          ),
          asset: toInteropAddress(output.asset, parseInt(output.assetChainId)),
          amount: output.amount || undefined,
        };
      });

      // Use first input's user as the request user
      const request: QuoteRequest = {
        user: apiInputs[0].user,
        intent: {
          intentType: 'oif-swap',
          inputs: apiInputs,
          outputs: apiOutputs,
          swapType,
        },
        supportedTypes,
        solverOptions: showSolverOptions ? solverOptions : undefined,
      };

      localStorageService.saveRecentSearch(request);
      onSubmit(request);
    } catch (error) {
      alert((error as Error).message);
    }
  };

  const handleLoadSearch = (request: QuoteRequest) => {
    try {
      // Load swap type
      setSwapType(request.intent.swapType || 'exact-input');

      // Load supported types
      setSupportedTypes(request.supportedTypes || ['oif-escrow-v0']);

      // Load inputs
      const loadedInputs: FormInput[] = request.intent.inputs.map((input) => {
        const userInterop = fromInteropAddress(input.user);
        const assetInterop = fromInteropAddress(input.asset);
        return {
          userAddress: userInterop.address,
          userChainId: userInterop.chainId.toString(),
          asset: assetInterop.address,
          assetChainId: assetInterop.chainId.toString(),
          amount: '', // Reset to empty - saved amount is already converted to smallest unit
        };
      });
      setInputs(loadedInputs);

      // Load outputs
      const loadedOutputs: FormOutput[] = request.intent.outputs.map(
        (output) => {
          const receiverInterop = fromInteropAddress(output.receiver);
          const assetInterop = fromInteropAddress(output.asset);
          return {
            receiverAddress: receiverInterop.address,
            receiverChainId: receiverInterop.chainId.toString(),
            asset: assetInterop.address,
            assetChainId: assetInterop.chainId.toString(),
            amount: '', // Reset to empty - saved amount is already converted to smallest unit
          };
        }
      );
      setOutputs(loadedOutputs);

      // Load solver options if present
      if (request.solverOptions) {
        setSolverOptions(request.solverOptions);
        setShowSolverOptions(true);
      }
    } catch (error) {
      console.error('Failed to load search:', error);
      alert('Failed to load search. The data format may have changed.');
    }
  };

  return (
    <form
      onSubmit={handleSubmit(handleFormSubmit)}
      className="space-y-6 max-w-4xl mx-auto"
    >
      <div className="card">
        <div className="flex items-center justify-between mb-6">
          <h2 className="text-2xl font-bold text-slate-900 dark:text-white">
            Request Quote (Advanced Mode)
          </h2>
          <button
            type="button"
            onClick={() => setShowRecentSearches(true)}
            className="btn-secondary text-sm py-1 px-3"
            title="View recent searches"
          >
            🕒 Recent Searches
          </button>
        </div>

        {/* Swap Type */}
        <div className="mb-6">
          <label className="label-text">Swap Type</label>
          <div className="flex gap-4 mt-2">
            <label className="flex items-center">
              <input
                type="radio"
                value="exact-input"
                checked={swapType === 'exact-input'}
                onChange={(e) => setSwapType(e.target.value as 'exact-input')}
                className="mr-2"
              />
              <span className="text-slate-900 dark:text-white">
                Exact Input
              </span>
            </label>
            <label className="flex items-center">
              <input
                type="radio"
                value="exact-output"
                checked={swapType === 'exact-output'}
                onChange={(e) => setSwapType(e.target.value as 'exact-output')}
                className="mr-2"
              />
              <span className="text-slate-900 dark:text-white">
                Exact Output
              </span>
            </label>
          </div>
        </div>

        {/* Inputs Section */}
        <div className="mb-6">
          <div className="flex justify-between items-center mb-3">
            <h3 className="text-lg font-semibold text-slate-900 dark:text-white">
              Inputs
            </h3>
            <button
              type="button"
              onClick={addInput}
              className="text-primary-600 dark:text-primary-400 hover:text-primary-700 dark:hover:text-primary-300 text-sm"
            >
              + Add Input
            </button>
          </div>
          {inputs.map((input, index) => (
            <div
              key={index}
              className="bg-slate-100 dark:bg-slate-900 rounded-lg p-4 mb-3 border border-slate-300 dark:border-slate-700"
            >
              <div className="flex justify-between items-center mb-3">
                <span className="text-slate-600 dark:text-slate-400 text-sm">
                  Input #{index + 1}
                </span>
                {inputs.length > 1 && (
                  <button
                    type="button"
                    onClick={() => removeInput(index)}
                    className="text-red-600 dark:text-red-400 hover:text-red-700 dark:hover:text-red-300 text-sm"
                  >
                    Remove
                  </button>
                )}
              </div>
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="label-text">User Address</label>
                  <input
                    type="text"
                    value={input.userAddress}
                    onChange={(e) =>
                      updateInput(index, 'userAddress', e.target.value)
                    }
                    placeholder="0x..."
                    className="input-field"
                  />
                </div>
                <div>
                  <label className="label-text">Chain ID</label>
                  <input
                    type="number"
                    value={input.userChainId}
                    onChange={(e) =>
                      updateInput(index, 'userChainId', e.target.value)
                    }
                    placeholder="1"
                    className="input-field"
                  />
                </div>
                <div>
                  <label className="label-text">Asset Address</label>
                  <input
                    type="text"
                    value={input.asset}
                    onChange={(e) =>
                      updateInput(index, 'asset', e.target.value)
                    }
                    placeholder="0x..."
                    className="input-field"
                  />
                </div>
                <div>
                  <label className="label-text">Asset Chain ID</label>
                  <input
                    type="number"
                    value={input.assetChainId}
                    onChange={(e) =>
                      updateInput(index, 'assetChainId', e.target.value)
                    }
                    placeholder="1"
                    className="input-field"
                  />
                </div>
                <div className="col-span-2">
                  <label className="label-text">
                    Amount (optional for exact-output)
                  </label>
                  <input
                    type="text"
                    value={input.amount}
                    onChange={(e) =>
                      updateInput(index, 'amount', e.target.value)
                    }
                    placeholder="1000000000000000000"
                    className="input-field"
                  />
                </div>
              </div>
            </div>
          ))}
        </div>

        {/* Outputs Section */}
        <div className="mb-6">
          <div className="flex justify-between items-center mb-3">
            <h3 className="text-lg font-semibold text-slate-900 dark:text-white">
              Outputs
            </h3>
            <button
              type="button"
              onClick={addOutput}
              className="text-primary-600 dark:text-primary-400 hover:text-primary-700 dark:hover:text-primary-300 text-sm"
            >
              + Add Output
            </button>
          </div>
          {outputs.map((output, index) => (
            <div
              key={index}
              className="bg-slate-100 dark:bg-slate-900 rounded-lg p-4 mb-3 border border-slate-300 dark:border-slate-700"
            >
              <div className="flex justify-between items-center mb-3">
                <span className="text-slate-600 dark:text-slate-400 text-sm">
                  Output #{index + 1}
                </span>
                {outputs.length > 1 && (
                  <button
                    type="button"
                    onClick={() => removeOutput(index)}
                    className="text-red-600 dark:text-red-400 hover:text-red-700 dark:hover:text-red-300 text-sm"
                  >
                    Remove
                  </button>
                )}
              </div>
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="label-text">Receiver Address</label>
                  <input
                    type="text"
                    value={output.receiverAddress}
                    onChange={(e) =>
                      updateOutput(index, 'receiverAddress', e.target.value)
                    }
                    placeholder="0x..."
                    className="input-field"
                  />
                </div>
                <div>
                  <label className="label-text">Chain ID</label>
                  <input
                    type="number"
                    value={output.receiverChainId}
                    onChange={(e) =>
                      updateOutput(index, 'receiverChainId', e.target.value)
                    }
                    placeholder="1"
                    className="input-field"
                  />
                </div>
                <div>
                  <label className="label-text">Asset Address</label>
                  <input
                    type="text"
                    value={output.asset}
                    onChange={(e) =>
                      updateOutput(index, 'asset', e.target.value)
                    }
                    placeholder="0x..."
                    className="input-field"
                  />
                </div>
                <div>
                  <label className="label-text">Asset Chain ID</label>
                  <input
                    type="number"
                    value={output.assetChainId}
                    onChange={(e) =>
                      updateOutput(index, 'assetChainId', e.target.value)
                    }
                    placeholder="1"
                    className="input-field"
                  />
                </div>
                <div className="col-span-2">
                  <label className="label-text">
                    Amount (optional for exact-input)
                  </label>
                  <input
                    type="text"
                    value={output.amount}
                    onChange={(e) =>
                      updateOutput(index, 'amount', e.target.value)
                    }
                    placeholder="1000000"
                    className="input-field"
                  />
                </div>
              </div>
            </div>
          ))}
        </div>

        {/* Supported Order Types */}
        <div className="mb-6">
          <label className="label-text">Supported Order Types</label>
          <div className="flex flex-wrap gap-2 mt-2">
            {[
              'oif-escrow-v0',
              'oif-resource-lock-v0',
              'oif-3009-v0',
              'oif-generic-v0',
            ].map((type) => (
              <label key={type} className="flex items-center">
                <input
                  type="checkbox"
                  checked={supportedTypes.includes(type)}
                  onChange={(e) => {
                    if (e.target.checked) {
                      setSupportedTypes([...supportedTypes, type]);
                    } else {
                      setSupportedTypes(
                        supportedTypes.filter((t) => t !== type)
                      );
                    }
                  }}
                  className="mr-2"
                />
                <span className="text-slate-900 dark:text-white text-sm">
                  {type}
                </span>
              </label>
            ))}
          </div>
        </div>

        {/* Solver Options (Collapsible) */}
        <div className="mb-6">
          <button
            type="button"
            onClick={() => setShowSolverOptions(!showSolverOptions)}
            className="flex items-center justify-between w-full text-left text-slate-900 dark:text-white hover:text-primary-600 dark:hover:text-primary-400 transition-colors"
          >
            <span className="font-semibold">Advanced: Solver Options</span>
            <span className="text-xl">{showSolverOptions ? '−' : '+'}</span>
          </button>
          {showSolverOptions && (
            <SolverOptions
              options={solverOptions}
              onChange={setSolverOptions}
            />
          )}
        </div>

        {/* Submit Button */}
        <button
          type="submit"
          disabled={isLoading}
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
