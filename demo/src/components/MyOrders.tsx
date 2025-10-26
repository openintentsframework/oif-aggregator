import { useState, useEffect } from 'react';
import type { OrderResponse, OrderStatus } from '../types/api';
import { localStorageService, type StoredOrder } from '../services/localStorageService';
import { formatInteropAddress } from '../utils/interopAddress';

interface MyOrdersProps {
  onViewOrder: (order: OrderResponse) => void;
  onBack: () => void;
}

export default function MyOrders({ onViewOrder, onBack }: MyOrdersProps) {
  const [orders, setOrders] = useState<StoredOrder[]>([]);

  useEffect(() => {
    loadOrders();
  }, []);

  const loadOrders = () => {
    setOrders(localStorageService.getOrders());
  };

  const handleDelete = (orderId: string) => {
    if (confirm('Are you sure you want to delete this order from history?')) {
      localStorageService.deleteOrder(orderId);
      loadOrders();
    }
  };

  const handleClearAll = () => {
    if (confirm('Are you sure you want to clear all order history?')) {
      localStorageService.clearOrders();
      loadOrders();
    }
  };

  const handleCopyOrderId = (orderId: string) => {
    navigator.clipboard.writeText(orderId);
    alert('Order ID copied to clipboard!');
  };

  const getErrorMessage = (status: OrderStatus): string | null => {
    if (typeof status === 'object' && 'failed' in status) {
      const [txType, error] = status.failed;
      return error;
    }
    return null;
  };

  const getStatusText = (status: OrderStatus): string => {
    // Handle complex failed status
    if (typeof status === 'object' && 'failed' in status) {
      const [txType, error] = status.failed;
      return `failed (${txType})`;
    }
    
    return status;
  };

  const getStatusColor = (status: OrderStatus): string => {
    // Handle complex failed status
    if (typeof status === 'object' && 'failed' in status) {
      return 'bg-red-600 dark:bg-red-700';
    }
    
    switch (status.toLowerCase()) {
      case 'finalized':
      case 'settled':
      case 'executed':
        return 'bg-green-600 dark:bg-green-700';
      case 'failed':
      case 'rejected':
        return 'bg-red-600 dark:bg-red-700';
      case 'executing':
      case 'settling':
        return 'bg-blue-600 dark:bg-blue-700';
      case 'pending':
      case 'created':
        return 'bg-yellow-600 dark:bg-yellow-700';
      default:
        return 'bg-slate-600 dark:bg-slate-700';
    }
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

  return (
    <div className="space-y-4 max-w-6xl mx-auto">
      <div className="card py-4">
        <div className="flex justify-between items-center mb-6">
          <div>
            <h2 className="text-2xl font-bold text-slate-900 dark:text-white">My Orders</h2>
            <p className="text-sm text-slate-600 dark:text-slate-400 mt-1">
              {orders.length} order{orders.length !== 1 ? 's' : ''} in history
            </p>
          </div>
          <div className="flex gap-2">
            {orders.length > 0 && (
              <button
                onClick={handleClearAll}
                className="btn-secondary text-sm"
              >
                Clear All
              </button>
            )}
            <button onClick={onBack} className="btn-secondary">
              ← Back
            </button>
          </div>
        </div>

        {orders.length === 0 ? (
          <div className="text-center py-12">
            <p className="text-slate-600 dark:text-slate-400 text-lg">No orders yet</p>
            <p className="text-slate-500 dark:text-slate-500 text-sm mt-2">
              Your submitted orders will appear here
            </p>
          </div>
        ) : (
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-slate-300 dark:border-slate-700">
                  <th className="text-left py-3 px-4 text-sm font-semibold text-slate-700 dark:text-slate-300">
                    Order ID
                  </th>
                  <th className="text-left py-3 px-4 text-sm font-semibold text-slate-700 dark:text-slate-300">
                    Status
                  </th>
                  <th className="text-left py-3 px-4 text-sm font-semibold text-slate-700 dark:text-slate-300">
                    Created
                  </th>
                  <th className="text-left py-3 px-4 text-sm font-semibold text-slate-700 dark:text-slate-300">
                    Input
                  </th>
                  <th className="text-left py-3 px-4 text-sm font-semibold text-slate-700 dark:text-slate-300">
                    Output
                  </th>
                  <th className="text-left py-3 px-4 text-sm font-semibold text-slate-700 dark:text-slate-300">
                    Actions
                  </th>
                </tr>
              </thead>
              <tbody>
                {orders.map((storedOrder, index) => {
                  const { order } = storedOrder;
                  const inputAsset = order.inputAmounts[0]
                    ? formatInteropAddress(order.inputAmounts[0].asset)
                    : 'N/A';
                  const outputAsset = order.outputAmounts[0]
                    ? formatInteropAddress(order.outputAmounts[0].asset)
                    : 'N/A';
                  const inputAmount = order.inputAmounts[0]
                    ? formatAmount(order.inputAmounts[0].amount)
                    : 'N/A';
                  const outputAmount = order.outputAmounts[0]
                    ? formatAmount(order.outputAmounts[0].amount)
                    : 'N/A';

                  return (
                    <tr
                      key={index}
                      className="border-b border-slate-200 dark:border-slate-700/50 hover:bg-slate-100 dark:hover:bg-slate-800/30 transition-colors"
                    >
                      <td className="py-3 px-4">
                        <div className="flex items-center gap-2">
                          <span className="text-slate-900 dark:text-white font-mono text-xs">
                            {order.orderId.slice(0, 8)}...{order.orderId.slice(-6)}
                          </span>
                          <button
                            onClick={() => handleCopyOrderId(order.orderId)}
                            className="text-slate-500 dark:text-slate-400 hover:text-slate-700 dark:hover:text-slate-200 text-xs"
                            title="Copy full order ID"
                          >
                            📋
                          </button>
                        </div>
                      </td>
                      <td className="py-3 px-4">
                        <span
                          className={`${getStatusColor(order.status)} text-white px-2 py-1 rounded text-xs font-medium uppercase`}
                          title={getErrorMessage(order.status) || undefined}
                        >
                          {getStatusText(order.status)}
                        </span>
                      </td>
                      <td className="py-3 px-4">
                        <span className="text-slate-600 dark:text-slate-400 text-xs">
                          {new Date(storedOrder.timestamp).toLocaleString()}
                        </span>
                      </td>
                      <td className="py-3 px-4">
                        <div className="text-xs">
                          <p className="text-slate-900 dark:text-white font-medium">{inputAmount}</p>
                          <p className="text-slate-500 dark:text-slate-500 truncate max-w-[150px]">
                            {inputAsset}
                          </p>
                        </div>
                      </td>
                      <td className="py-3 px-4">
                        <div className="text-xs">
                          <p className="text-slate-900 dark:text-white font-medium">{outputAmount}</p>
                          <p className="text-slate-500 dark:text-slate-500 truncate max-w-[150px]">
                            {outputAsset}
                          </p>
                        </div>
                      </td>
                      <td className="py-3 px-4">
                        <div className="flex gap-2">
                          <button
                            onClick={() => onViewOrder(order)}
                            className="text-primary-600 dark:text-primary-400 hover:text-primary-700 dark:hover:text-primary-300 text-xs font-medium"
                          >
                            View Details
                          </button>
                          <button
                            onClick={() => handleDelete(order.orderId)}
                            className="text-red-600 dark:text-red-400 hover:text-red-700 dark:hover:text-red-300 text-xs"
                            title="Delete from history"
                          >
                            🗑
                          </button>
                        </div>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}

