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

export default function OrderStatus({ order, onStartOver, onOrderUpdate }: OrderStatusProps): JSX.Element {
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [isPolling, setIsPolling] = useState(true);

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
    
    return ['created', 'pending', 'executing', 'settling'].includes(status);
  };

  // Check if status is failed (handles both simple and complex failed status)
  const isFailed = (status: OrderStatusType): boolean => {
    return status === 'failed' || (typeof status === 'object' && 'failed' in status);
  };

  // Check if status is successful final state
  const isSuccessful = (status: OrderStatusType): boolean => {
    return status === 'finalized' || status === 'settled' || status === 'executed';
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

  return (
    <div className="space-y-4 max-w-4xl mx-auto">
      <div className="card py-4">
        <div className="flex justify-between items-center mb-6">
          <h2 className="text-2xl font-bold text-slate-900 dark:text-white">Order Status</h2>
          <button onClick={onStartOver} className="btn-secondary">
            🔄 Start Over
          </button>
        </div>

        {/* Status Badge */}
        <div className="flex items-center gap-4 mb-6 flex-wrap">
          <span className={`${getStatusColor(order.status)} text-white px-4 py-2 rounded-lg font-semibold uppercase text-sm`}>
            {getStatusText(order.status)}
          </span>
          <button
            onClick={handleRefresh}
            disabled={isRefreshing}
            className="btn-secondary text-sm py-1 px-3"
          >
            {isRefreshing ? '⟳ Refreshing...' : '🔄 Refresh Status'}
          </button>
          <button
            onClick={() => setIsPolling(!isPolling)}
            className={`text-sm py-1 px-3 rounded transition-colors ${
              isPolling 
                ? 'bg-green-600 hover:bg-green-700 text-white' 
                : 'bg-slate-600 hover:bg-slate-700 text-white'
            }`}
            title={isPolling ? 'Auto-refresh enabled (every 2s)' : 'Auto-refresh disabled'}
          >
            {isPolling ? '⏸ Auto-Refresh ON' : '▶ Auto-Refresh OFF'}
          </button>
          {isPolling && shouldPoll(order.status) && (
            <span className="text-xs text-slate-600 dark:text-slate-400 animate-pulse">
              • Polling every 2s
            </span>
          )}
        </div>

        {/* Status Notification */}
        {!shouldPoll(order.status) && (
          <div className={`rounded-lg p-4 mb-6 ${
            isSuccessful(order.status)
              ? 'bg-green-900/20 border border-green-700'
              : isFailed(order.status)
              ? 'bg-red-900/20 border border-red-700'
              : 'bg-blue-900/20 border border-blue-700'
          }`}>
            <p className={`text-sm ${
              isSuccessful(order.status)
                ? 'text-green-400'
                : isFailed(order.status)
                ? 'text-red-400'
                : 'text-blue-400'
            }`}>
              {isSuccessful(order.status)
                ? '✓ Order completed successfully. Auto-refresh has stopped.'
                : isFailed(order.status)
                ? '✗ Order failed. Auto-refresh has stopped.'
                : `Order is in final state: ${getStatusText(order.status)}. Auto-refresh has stopped.`
              }
            </p>
          </div>
        )}

        {/* Error Details - Show detailed error message for failed orders */}
        {isFailed(order.status) && getErrorMessage(order.status) && (
          <div className="bg-red-900/20 border border-red-700 rounded-lg p-4 mb-6">
            <div className="flex items-start gap-3">
              <span className="text-red-400 text-lg flex-shrink-0">⚠️</span>
              <div className="flex-1">
                <h3 className="text-red-300 font-medium mb-2">Error Details</h3>
                <p className="text-red-200 text-sm leading-relaxed break-words">
                  {getErrorMessage(order.status)}
                </p>
              </div>
            </div>
          </div>
        )}

        {/* Order Details */}
        <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-4 mb-6 space-y-4 border border-slate-300 dark:border-slate-700">
          <div className="grid grid-cols-2 gap-4">
            <div>
              <p className="text-xs text-slate-600 dark:text-slate-400">Order ID</p>
              <div className="flex items-center gap-2">
                <p className="text-slate-900 dark:text-white font-mono text-sm truncate" title={order.orderId}>
                  {truncateString(order.orderId, 20)}
                </p>
                <button
                  onClick={() => copyToClipboard(order.orderId, 'Order ID')}
                  className="text-slate-500 hover:text-slate-700 dark:text-slate-400 dark:hover:text-slate-200 transition-colors p-1 flex-shrink-0"
                  title="Copy Order ID"
                >
                  📋
                </button>
              </div>
            </div>
            {order.quoteId && (
              <div>
                <p className="text-xs text-slate-600 dark:text-slate-400">Quote ID</p>
                <div className="flex items-center gap-2">
                  <p className="text-slate-900 dark:text-white font-mono text-sm truncate" title={order.quoteId}>
                    {truncateString(order.quoteId, 20)}
                  </p>
                  <button
                    onClick={() => copyToClipboard(order.quoteId, 'Quote ID')}
                    className="text-slate-500 hover:text-slate-700 dark:text-slate-400 dark:hover:text-slate-200 transition-colors p-1 flex-shrink-0"
                    title="Copy Quote ID"
                  >
                    📋
                  </button>
                </div>
              </div>
            )}
            <div>
              <p className="text-xs text-slate-600 dark:text-slate-400">Created At</p>
              <p className="text-slate-900 dark:text-white text-sm">{formatTimestamp(order.createdAt)}</p>
            </div>
            <div>
              <p className="text-xs text-slate-600 dark:text-slate-400">Updated At</p>
              <p className="text-slate-900 dark:text-white text-sm">{formatTimestamp(order.updatedAt)}</p>
            </div>
          </div>
        </div>

        {/* Input Amounts */}
        <div className="mb-6">
          <h3 className="text-lg font-semibold text-slate-900 dark:text-white mb-3">Input Amounts</h3>
          <div className="space-y-2">
            {order.inputAmounts.map((input, idx) => (
              <div key={idx} className="bg-slate-100 dark:bg-slate-900 rounded-lg p-3 border border-slate-300 dark:border-slate-700">
                <p className="text-slate-600 dark:text-slate-400 text-sm">
                  Amount: <span className="text-slate-900 dark:text-white font-medium">{formatAmount(input.amount)}</span>
                </p>
                <p className="text-slate-500 dark:text-slate-500 text-xs truncate mt-1">
                  Asset: {String(formatInteropAddress(input.asset))}
                </p>
              </div>
            ))}
          </div>
        </div>

        {/* Output Amounts */}
        <div className="mb-6">
          <h3 className="text-lg font-semibold text-slate-900 dark:text-white mb-3">Output Amounts</h3>
          <div className="space-y-2">
            {order.outputAmounts.map((output, idx) => (
              <div key={idx} className="bg-slate-100 dark:bg-slate-900 rounded-lg p-3 border border-slate-300 dark:border-slate-700">
                <p className="text-slate-600 dark:text-slate-400 text-sm">
                  Amount: <span className="text-slate-900 dark:text-white font-medium">{formatAmount(output.amount)}</span>
                </p>
                <p className="text-slate-500 dark:text-slate-500 text-xs truncate mt-1">
                  Asset: {String(formatInteropAddress(output.asset))}
                </p>
              </div>
            ))}
          </div>
        </div>

        {/* Settlement Information */}
        <div className="mb-6">
          <h3 className="text-lg font-semibold text-slate-900 dark:text-white mb-3">Settlement</h3>
          <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-4 border border-slate-300 dark:border-slate-700">
            <p className="text-slate-600 dark:text-slate-400 text-sm mb-2">
              Type: <span className="text-slate-900 dark:text-white font-medium">{order.settlement.type}</span>
            </p>
            <details className="text-sm">
              <summary className="text-slate-600 dark:text-slate-400 cursor-pointer hover:text-slate-900 dark:hover:text-white">Settlement Data (JSON)</summary>
              <pre className="mt-2 bg-slate-200 dark:bg-slate-800 p-3 rounded overflow-auto text-xs text-slate-700 dark:text-slate-300 border border-slate-300 dark:border-slate-700">
                {JSON.stringify(order.settlement.data as any, null, 2)}
              </pre>
            </details>
          </div>
        </div>

        {/* Fill Transaction (if available) */}
        {order.fillTransaction && (
          <div className="mb-6">
            <h3 className="text-lg font-semibold text-slate-900 dark:text-white mb-3">Fill Transaction</h3>
            <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-4 border border-slate-300 dark:border-slate-700">
              <pre className="text-xs text-slate-700 dark:text-slate-300 overflow-auto">
                {JSON.stringify(order.fillTransaction as any, null, 2)}
              </pre>
            </div>
          </div>
        )}

        {/* Status Helper */}
        <div className="bg-primary-100 dark:bg-primary-900/20 border border-primary-300 dark:border-primary-700 rounded-lg p-4">
          <p className="text-primary-700 dark:text-primary-400 text-sm">
            💡 <strong>Status Guide:</strong><br />
            • <strong>created/pending</strong>: Order received and queued<br />
            • <strong>executing</strong>: Order is being processed<br />
            • <strong>executed/settling</strong>: Transaction completed, awaiting confirmation<br />
            • <strong>settled/finalized</strong>: Order successfully completed<br />
            • <strong>failed</strong>: Order execution failed<br />
            • <strong>refunded</strong>: Funds returned to user
          </p>
        </div>
      </div>
    </div>
  );
}

