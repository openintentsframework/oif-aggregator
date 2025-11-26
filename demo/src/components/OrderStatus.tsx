// @ts-nocheck
import type { OrderResponse, OrderStatus as OrderStatusType } from '../types/api';
import { useEffect, useState } from 'react';

import { formatInteropAddress } from '../utils/interopAddress';
import { orderApi } from '../services/api';

interface OrderStatusProps {
  order: OrderResponse;
  onStartOver: () => void;
  onOrderUpdate: (order: OrderResponse) => void;
}

// Status steps configuration with descriptions for tooltips
const STATUS_STEPS = [
  { key: 'created', label: 'Created', description: 'Order received and queued' },
  { key: 'pending', label: 'Pending', description: 'Order is being validated' },
  { key: 'executing', label: 'Executing', description: 'Order is being processed' },
  { key: 'executed', label: 'Executed', description: 'Transaction completed' },
  { key: 'settling', label: 'Settling', description: 'Awaiting confirmation' },
  { key: 'settled', label: 'Settled', description: 'Confirmation received' },
  { key: 'finalized', label: 'Finalized', description: 'Order successfully completed' },
] as const;

type StepStatus = 'completed' | 'current' | 'failed' | 'pending';

interface StatusStepProps {
  label: string;
  description: string;
  status: StepStatus;
  isLast: boolean;
  isHovered: boolean;
  onMouseEnter: () => void;
  onMouseLeave: () => void;
}

// Individual step component
function StatusStep({ label, description, status, isLast, isHovered, onMouseEnter, onMouseLeave }: StatusStepProps) {
  // Determine colors based on step status
  let circleColor = 'bg-slate-300 dark:bg-slate-600';
  let textColor = 'text-slate-500 dark:text-slate-400';
  let lineColor = 'bg-slate-300 dark:bg-slate-600';
  
  if (status === 'completed') {
    circleColor = 'bg-green-500';
    textColor = 'text-green-600 dark:text-green-400';
    lineColor = 'bg-green-500';
  } else if (status === 'current') {
    circleColor = 'bg-orange-500';
    textColor = 'text-orange-600 dark:text-orange-400';
  } else if (status === 'failed') {
    circleColor = 'bg-red-500';
    textColor = 'text-red-600 dark:text-red-400';
    lineColor = 'bg-red-500';
  }

  return (
    <>
      {/* Step circle and label */}
      <div className="flex flex-col items-center relative flex-shrink-0">
        {/* Circle with info icon */}
        <div
          className="relative"
          onMouseEnter={onMouseEnter}
          onMouseLeave={onMouseLeave}
        >
          <div className={`w-10 h-10 rounded-full ${circleColor} flex items-center justify-center transition-all duration-200 cursor-help`}>
            <span className="text-white text-xs font-bold">ℹ</span>
          </div>
          
          {/* Tooltip */}
          {isHovered && (
            <div className="absolute z-10 px-3 py-2 text-sm text-white bg-slate-900 rounded-lg shadow-lg -top-16 left-1/2 transform -translate-x-1/2 whitespace-nowrap">
              <div className="font-semibold">{label}</div>
              <div className="text-xs text-slate-300">{description}</div>
              {/* Tooltip arrow */}
              <div className="absolute w-2 h-2 bg-slate-900 transform rotate-45 -bottom-1 left-1/2 -translate-x-1/2"></div>
            </div>
          )}
        </div>
        
        {/* Label */}
        <span className={`mt-2 text-xs font-medium ${textColor} text-center whitespace-nowrap`}>
          {label}
        </span>
      </div>

      {/* Connecting line (not for last item) */}
      {!isLast && (
        <div className={`flex-1 h-1 ${lineColor} mx-2 transition-all duration-200 min-w-[40px]`}></div>
      )}
    </>
  );
}

export default function OrderStatus({ order, onStartOver, onOrderUpdate }: OrderStatusProps): JSX.Element {
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [isPolling, setIsPolling] = useState(true);
  const [hoveredStep, setHoveredStep] = useState<string | null>(null);

  // Get the current status as a string (handling complex failed status)
  const getCurrentStatusKey = (status: OrderStatusType): string => {
    if (typeof status === 'object' && 'failed' in status) {
      return 'failed';
    }
    return status;
  };

  // Determine the status of a given step: 'completed', 'current', 'failed', or 'pending'
  const getStepStatus = (stepKey: string): 'completed' | 'current' | 'failed' | 'pending' => {
    const currentStatus = getCurrentStatusKey(order.status);
    
    // If current status is failed, mark the executing step as red, previous steps as completed
    if (currentStatus === 'failed') {
      // Typically orders fail during execution
      const failedStepIndex = STATUS_STEPS.findIndex(s => s.key === 'executing');
      const stepIndex = STATUS_STEPS.findIndex(s => s.key === stepKey);
      
      if (stepIndex < failedStepIndex) {
        return 'completed'; // Steps before failure are completed (green)
      } else if (stepIndex === failedStepIndex) {
        return 'failed'; // The step where it failed (red)
      } else {
        return 'pending'; // Steps after failure remain gray
      }
    }
    
    // Find indices
    const currentStepIndex = STATUS_STEPS.findIndex(s => s.key === currentStatus);
    const stepIndex = STATUS_STEPS.findIndex(s => s.key === stepKey);
    
    if (stepIndex < 0) return 'pending';
    
    if (stepIndex < currentStepIndex) {
      return 'completed';
    } else if (stepIndex === currentStepIndex) {
      // If this is the finalized step and we're at it, mark as completed (green) not current (orange)
      if (currentStatus === 'finalized' && stepKey === 'finalized') {
        return 'completed';
      }
      return 'current';
    } else {
      return 'pending';
    }
  };

  const getStatusColor = (status: OrderStatusType): string => {
    // Handle complex failed status
    if (typeof status === 'object' && 'failed' in status) {
      return 'bg-red-600';
    }
    
    switch (status) {
      case 'created':
      case 'pending':
        return 'bg-blue-600';
      case 'executing':
        return 'bg-yellow-600';
      case 'executed':
      case 'settled':
      case 'settling':
        return 'bg-green-600';
      case 'finalized':
        return 'bg-green-700';
      case 'failed':
        return 'bg-red-600';
      case 'refunded':
        return 'bg-orange-600';
      default:
        return 'bg-slate-600';
    }
  };

  const getStatusText = (status: OrderStatusType): string => {
    // Handle complex failed status
    if (typeof status === 'object' && 'failed' in status) {
      const [txType, error] = status.failed;
      return `failed (${txType})`;
    }
    
    return status;
  };

  const handleRefresh = async () => {
    setIsRefreshing(true);
    try {
      const updatedOrder = await orderApi.getOrder(order.orderId);
      onOrderUpdate(updatedOrder);
    } catch (error) {
      alert(`Failed to refresh: ${(error as Error).message}`);
    } finally {
      setIsRefreshing(false);
    }
  };

  // Check if order is in a non-final state (should keep polling)
  const shouldPoll = (status: OrderStatusType): boolean => {
    // Handle complex failed status
    if (typeof status === 'object' && 'failed' in status) {
      return false; // Failed orders should not poll
    }
    
    return ['created', 'pending', 'executing', 'executed', 'settling', 'settled'].includes(status);
  };

  // Check if status is failed (handles both simple and complex failed status)
  const isFailed = (status: OrderStatusType): boolean => {
    return status === 'failed' || (typeof status === 'object' && 'failed' in status);
  };

  // Check if status is successful final state
  const isSuccessful = (status: OrderStatusType): boolean => {
    return status === 'finalized';
  };

  // Get error message from failed status
  const getErrorMessage = (status: OrderStatusType): string | null => {
    if (typeof status === 'object' && 'failed' in status) {
      const [txType, error] = status.failed;
      return error;
    }
    return null;
  };

  // Auto-polling effect
  useEffect(() => {
    // Don't poll if polling is disabled or order is in a final state
    if (!isPolling || !shouldPoll(order.status)) {
      return;
    }

    // Set up polling interval (every 2 seconds)
    const intervalId = setInterval(async () => {
      try {
        const updatedOrder = await orderApi.getOrder(order.orderId);
        onOrderUpdate(updatedOrder);
        
        // Stop polling if order reaches a final state
        if (!shouldPoll(updatedOrder.status)) {
          setIsPolling(false);
        }
      } catch (error) {
        console.error('Polling error:', error);
        // Continue polling even if there's an error
      }
    }, 2000);

    // Cleanup interval on unmount or when dependencies change
    return () => clearInterval(intervalId);
  }, [order.orderId, order.status, isPolling, onOrderUpdate]);

  const formatTimestamp = (timestamp: string) => {
    return new Date(timestamp).toLocaleString();
  };

  const formatAmount = (amount?: string) => {
    if (!amount) return 'N/A';
    try {
      const num = BigInt(amount);
      return num.toString();
    } catch {
      return amount;
    }
  };

  const copyToClipboard = async (text: string, label: string) => {
    try {
      await navigator.clipboard.writeText(text);
      console.log(`${label} copied to clipboard`);
    } catch (err) {
      console.error('Failed to copy to clipboard:', err);
    }
  };

  const truncateString = (str: string, maxLength: number = 20) => {
    if (str.length <= maxLength) return str;
    return `${str.substring(0, maxLength)}...`;
  };

  // Render the status stepper component
  const renderStatusStepper = () => {
    const currentStatus = getCurrentStatusKey(order.status);
    const isFailedStatus = currentStatus === 'failed';
    const isRefundedStatus = currentStatus === 'refunded';

    return (
      <div className="mb-6 w-full">
        {/* Main stepper row */}
        <div className="flex items-center w-full mb-4 px-2">
          {STATUS_STEPS.map((step, index) => (
            <StatusStep
              key={step.key}
              label={step.label}
              description={step.description}
              status={getStepStatus(step.key)}
              isLast={index === STATUS_STEPS.length - 1}
              isHovered={hoveredStep === step.key}
              onMouseEnter={() => setHoveredStep(step.key)}
              onMouseLeave={() => setHoveredStep(null)}
            />
          ))}
        </div>

        {/* Refunded branch (shown if order is refunded) */}
        {isRefundedStatus && (
          <div className="flex items-center mt-4 ml-12">
            <div className="w-px h-8 bg-orange-500 mb-2"></div>
            <div className="flex flex-col items-center ml-4">
              <div className="w-10 h-10 rounded-full bg-orange-500 flex items-center justify-center">
                <span className="text-white text-xs font-bold">ℹ</span>
              </div>
              <span className="mt-2 text-xs font-medium text-orange-600 dark:text-orange-400">
                Refunded
              </span>
            </div>
          </div>
        )}

        {/* Error message (shown if order failed) */}
        {isFailedStatus && getErrorMessage(order.status) && (
          <div className="p-4 mt-4 rounded-lg border border-red-700 bg-red-900/20">
            <div className="flex gap-3 items-start">
              <span className="flex-shrink-0 text-lg text-red-400">⚠️</span>
              <div className="flex-1">
                <h3 className="mb-2 font-medium text-red-300">Error Details</h3>
                <p className="text-sm leading-relaxed text-red-200 break-words">
                  {getErrorMessage(order.status)}
                </p>
              </div>
            </div>
          </div>
        )}
      </div>
    );
  };

  return (
    <div className="mx-auto space-y-4 max-w-4xl">
      <div className="py-4 card">
        <div className="flex justify-between items-center mb-6">
          <h2 className="text-2xl font-bold text-slate-900 dark:text-white">Order Status</h2>
          <button onClick={onStartOver} className="btn-secondary">
            🔄 Start Over
          </button>
        </div>

        {/* Control Buttons */}
        <div className="flex flex-wrap gap-3 items-center mb-6">
          <button
            onClick={handleRefresh}
            disabled={isRefreshing}
            className="px-3 py-2 text-sm btn-secondary"
          >
            {isRefreshing ? '⟳ Refreshing...' : '🔄 Refresh Status'}
          </button>
          <button
            onClick={() => setIsPolling(!isPolling)}
            className={`text-sm py-2 px-3 rounded transition-colors ${
              isPolling 
                ? 'text-white bg-green-600 hover:bg-green-700' 
                : 'text-white bg-slate-600 hover:bg-slate-700'
            }`}
            title={isPolling ? 'Auto-refresh enabled (every 2s)' : 'Auto-refresh disabled'}
          >
            {isPolling ? '⏸ Auto-Refresh ON' : '▶ Auto-Refresh OFF'}
          </button>
          {isPolling && shouldPoll(order.status) && (
            <span className="text-xs animate-pulse text-slate-600 dark:text-slate-400">
              • Polling every 2s
            </span>
          )}
        </div>

        {/* Status Stepper */}
        {renderStatusStepper()}

        {/* Success Notification */}
        {!shouldPoll(order.status) && isSuccessful(order.status) && (
          <div className="rounded-lg p-4 mb-6 border border-green-700 bg-green-900/20">
            <p className="text-sm text-green-400">
              ✓ Order completed successfully. Auto-refresh has stopped.
            </p>
          </div>
        )}

        {/* Order Details */}
        <div className="p-4 mb-6 space-y-4 rounded-lg border bg-slate-100 dark:bg-slate-900 border-slate-300 dark:border-slate-700">
          <div className="grid grid-cols-2 gap-4">
            <div>
              <p className="text-xs text-slate-600 dark:text-slate-400">Order ID</p>
              <div className="flex gap-2 items-center">
                <p className="font-mono text-sm truncate text-slate-900 dark:text-white" title={order.orderId}>
                  {truncateString(order.orderId, 20)}
                </p>
                <button
                  onClick={() => copyToClipboard(order.orderId, 'Order ID')}
                  className="flex-shrink-0 p-1 transition-colors text-slate-500 hover:text-slate-700 dark:text-slate-400 dark:hover:text-slate-200"
                  title="Copy Order ID"
                >
                  📋
                </button>
              </div>
            </div>
            {order.quoteId && (
              <div>
                <p className="text-xs text-slate-600 dark:text-slate-400">Quote ID</p>
                <div className="flex gap-2 items-center">
                  <p className="font-mono text-sm truncate text-slate-900 dark:text-white" title={order.quoteId}>
                    {truncateString(order.quoteId, 20)}
                  </p>
                  <button
                    onClick={() => copyToClipboard(order.quoteId, 'Quote ID')}
                    className="flex-shrink-0 p-1 transition-colors text-slate-500 hover:text-slate-700 dark:text-slate-400 dark:hover:text-slate-200"
                    title="Copy Quote ID"
                  >
                    📋
                  </button>
                </div>
              </div>
            )}
            <div>
              <p className="text-xs text-slate-600 dark:text-slate-400">Created At</p>
              <p className="text-sm text-slate-900 dark:text-white">{formatTimestamp(order.createdAt)}</p>
            </div>
            <div>
              <p className="text-xs text-slate-600 dark:text-slate-400">Updated At</p>
              <p className="text-sm text-slate-900 dark:text-white">{formatTimestamp(order.updatedAt)}</p>
            </div>
          </div>
        </div>

        {/* Input Amounts */}
        <div className="mb-6">
          <h3 className="mb-3 text-lg font-semibold text-slate-900 dark:text-white">Input Amounts</h3>
          <div className="space-y-2">
            {order.inputAmounts.map((input, idx) => (
              <div key={idx} className="p-3 rounded-lg border bg-slate-100 dark:bg-slate-900 border-slate-300 dark:border-slate-700">
                <p className="text-sm text-slate-600 dark:text-slate-400">
                  Amount: <span className="font-medium text-slate-900 dark:text-white">{formatAmount(input.amount)}</span>
                </p>
                <p className="mt-1 text-xs truncate text-slate-500 dark:text-slate-500">
                  Asset: {String(formatInteropAddress(input.asset))}
                </p>
              </div>
            ))}
          </div>
        </div>

        {/* Output Amounts */}
        <div className="mb-6">
          <h3 className="mb-3 text-lg font-semibold text-slate-900 dark:text-white">Output Amounts</h3>
          <div className="space-y-2">
            {order.outputAmounts.map((output, idx) => (
              <div key={idx} className="p-3 rounded-lg border bg-slate-100 dark:bg-slate-900 border-slate-300 dark:border-slate-700">
                <p className="text-sm text-slate-600 dark:text-slate-400">
                  Amount: <span className="font-medium text-slate-900 dark:text-white">{formatAmount(output.amount)}</span>
                </p>
                <p className="mt-1 text-xs truncate text-slate-500 dark:text-slate-500">
                  Asset: {String(formatInteropAddress(output.asset))}
                </p>
              </div>
            ))}
          </div>
        </div>

        {/* Settlement Information */}
        <div className="mb-6">
          <h3 className="mb-3 text-lg font-semibold text-slate-900 dark:text-white">Settlement</h3>
          <div className="p-4 rounded-lg border bg-slate-100 dark:bg-slate-900 border-slate-300 dark:border-slate-700">
            <p className="mb-2 text-sm text-slate-600 dark:text-slate-400">
              Type: <span className="font-medium text-slate-900 dark:text-white">{order.orderType}</span>
            </p>
            <details className="text-sm">
              <summary className="cursor-pointer text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white">Settlement Data (JSON)</summary>
              <pre className="overflow-auto p-3 mt-2 text-xs rounded border bg-slate-200 dark:bg-slate-800 text-slate-700 dark:text-slate-300 border-slate-300 dark:border-slate-700">
                {JSON.stringify(order.settlement.data as any, null, 2)}
              </pre>
            </details>
          </div>
        </div>

        {/* Fill Transaction (if available) */}
        {order.fillTransaction && (
          <div className="mb-6">
            <h3 className="mb-3 text-lg font-semibold text-slate-900 dark:text-white">Fill Transaction</h3>
            <div className="p-4 rounded-lg border bg-slate-100 dark:bg-slate-900 border-slate-300 dark:border-slate-700">
              <pre className="overflow-auto text-xs text-slate-700 dark:text-slate-300">
                {JSON.stringify(order.fillTransaction as any, null, 2)}
              </pre>
            </div>
          </div>
        )}

      </div>
    </div>
  );
}

