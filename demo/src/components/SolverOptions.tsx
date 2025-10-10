import type { SolverOptions as SolverOptionsType } from '../types/api';
import { useState } from 'react';

interface SolverOptionsProps {
  options: Partial<SolverOptionsType>;
  onChange: (options: Partial<SolverOptionsType>) => void;
}

export default function SolverOptions({ options, onChange }: SolverOptionsProps) {
  const [includeSolverInput, setIncludeSolverInput] = useState('');
  const [excludeSolverInput, setExcludeSolverInput] = useState('');

  const updateOption = <K extends keyof SolverOptionsType>(
    key: K,
    value: SolverOptionsType[K]
  ) => {
    onChange({ ...options, [key]: value });
  };

  const addIncludeSolver = () => {
    if (includeSolverInput.trim()) {
      const current = options.includeSolvers || [];
      updateOption('includeSolvers', [...current, includeSolverInput.trim()]);
      setIncludeSolverInput('');
    }
  };

  const removeIncludeSolver = (solver: string) => {
    const current = options.includeSolvers || [];
    updateOption('includeSolvers', current.filter(s => s !== solver));
  };

  const addExcludeSolver = () => {
    if (excludeSolverInput.trim()) {
      const current = options.excludeSolvers || [];
      updateOption('excludeSolvers', [...current, excludeSolverInput.trim()]);
      setExcludeSolverInput('');
    }
  };

  const removeExcludeSolver = (solver: string) => {
    const current = options.excludeSolvers || [];
    updateOption('excludeSolvers', current.filter(s => s !== solver));
  };

  return (
    <div className="mt-4 space-y-4 bg-slate-100 dark:bg-slate-900 rounded-lg p-4 border border-slate-300 dark:border-slate-700">
      {/* Timeout Settings */}
      <div className="grid grid-cols-2 gap-4">
        <div>
          <label className="label-text">Global Timeout (ms)</label>
          <input
            type="number"
            value={options.timeout || ''}
            onChange={(e) => updateOption('timeout', parseInt(e.target.value) || undefined)}
            placeholder="10000"
            className="input-field"
          />
        </div>
        <div>
          <label className="label-text">Solver Timeout (ms)</label>
          <input
            type="number"
            value={options.solverTimeout || ''}
            onChange={(e) => updateOption('solverTimeout', parseInt(e.target.value) || undefined)}
            placeholder="5000"
            className="input-field"
          />
        </div>
      </div>

      {/* Min Quotes */}
      <div>
        <label className="label-text">Minimum Quotes Required</label>
        <input
          type="number"
          value={options.minQuotes || ''}
          onChange={(e) => updateOption('minQuotes', parseInt(e.target.value) || undefined)}
          placeholder="1"
          className="input-field"
        />
      </div>

      {/* Solver Selection */}
      <div>
        <label className="label-text">Solver Selection Strategy</label>
        <select
          value={options.solverSelection || 'all'}
          onChange={(e) => updateOption('solverSelection', e.target.value as 'all' | 'sampled' | 'priority')}
          className="input-field"
        >
          <option value="all">All</option>
          <option value="sampled">Sampled</option>
          <option value="priority">Priority</option>
        </select>
      </div>

      {/* Sample Size (only for sampled mode) */}
      {options.solverSelection === 'sampled' && (
        <div>
          <label className="label-text">Sample Size</label>
          <input
            type="number"
            value={options.sampleSize || ''}
            onChange={(e) => updateOption('sampleSize', parseInt(e.target.value) || undefined)}
            placeholder="5"
            className="input-field"
          />
        </div>
      )}

      {/* Priority Threshold (only for priority mode) */}
      {options.solverSelection === 'priority' && (
        <div>
          <label className="label-text">Priority Threshold</label>
          <input
            type="number"
            value={options.priorityThreshold || ''}
            onChange={(e) => updateOption('priorityThreshold', parseInt(e.target.value) || undefined)}
            placeholder="70"
            className="input-field"
          />
        </div>
      )}

      {/* Include Solvers */}
      <div>
        <label className="label-text">Include Solvers</label>
        <div className="flex gap-2 mb-2">
          <input
            type="text"
            value={includeSolverInput}
            onChange={(e) => setIncludeSolverInput(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && (e.preventDefault(), addIncludeSolver())}
            placeholder="solver-id"
            className="input-field flex-1"
          />
          <button
            type="button"
            onClick={addIncludeSolver}
            className="btn-secondary"
          >
            Add
          </button>
        </div>
        <div className="flex flex-wrap gap-2">
          {(options.includeSolvers || []).map((solver) => (
            <span
              key={solver}
              className="bg-primary-600 text-white px-3 py-1 rounded-full text-sm flex items-center gap-2"
            >
              {solver}
              <button
                type="button"
                onClick={() => removeIncludeSolver(solver)}
                className="hover:text-red-300"
              >
                ×
              </button>
            </span>
          ))}
        </div>
      </div>

      {/* Exclude Solvers */}
      <div>
        <label className="label-text">Exclude Solvers</label>
        <div className="flex gap-2 mb-2">
          <input
            type="text"
            value={excludeSolverInput}
            onChange={(e) => setExcludeSolverInput(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && (e.preventDefault(), addExcludeSolver())}
            placeholder="solver-id"
            className="input-field flex-1"
          />
          <button
            type="button"
            onClick={addExcludeSolver}
            className="btn-secondary"
          >
            Add
          </button>
        </div>
        <div className="flex flex-wrap gap-2">
          {(options.excludeSolvers || []).map((solver) => (
            <span
              key={solver}
              className="bg-red-600 text-white px-3 py-1 rounded-full text-sm flex items-center gap-2"
            >
              {solver}
              <button
                type="button"
                onClick={() => removeExcludeSolver(solver)}
                className="hover:text-red-300"
              >
                ×
              </button>
            </span>
          ))}
        </div>
      </div>
    </div>
  );
}

