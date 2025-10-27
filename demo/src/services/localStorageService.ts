import type { OrderResponse, QuoteRequest } from '../types/api';

const RECENT_SEARCHES_KEY = 'oif_recent_searches';
const ORDERS_KEY = 'oif_orders';
const MAX_RECENT_SEARCHES = 5;
const MAX_ORDERS = 20;

interface StoredSearch {
  timestamp: number;
  request: QuoteRequest;
}

interface StoredOrder {
  timestamp: number;
  order: OrderResponse;
}

class LocalStorageService {
  // Recent Searches
  saveRecentSearch(request: QuoteRequest): void {
    try {
      const searches = this.getRecentSearches();
      const newSearch: StoredSearch = {
        timestamp: Date.now(),
        request,
      };

      // Add to beginning and limit to MAX_RECENT_SEARCHES
      const updatedSearches = [newSearch, ...searches].slice(
        0,
        MAX_RECENT_SEARCHES
      );

      localStorage.setItem(
        RECENT_SEARCHES_KEY,
        JSON.stringify(updatedSearches)
      );
    } catch (error) {
      console.error('Failed to save recent search:', error);
    }
  }

  getRecentSearches(): StoredSearch[] {
    try {
      const data = localStorage.getItem(RECENT_SEARCHES_KEY);
      if (!data) return [];

      const searches = JSON.parse(data);
      return Array.isArray(searches) ? searches : [];
    } catch (error) {
      console.error('Failed to get recent searches:', error);
      return [];
    }
  }

  deleteRecentSearch(index: number): void {
    try {
      const searches = this.getRecentSearches();
      searches.splice(index, 1);
      localStorage.setItem(RECENT_SEARCHES_KEY, JSON.stringify(searches));
    } catch (error) {
      console.error('Failed to delete recent search:', error);
    }
  }

  clearRecentSearches(): void {
    try {
      localStorage.removeItem(RECENT_SEARCHES_KEY);
    } catch (error) {
      console.error('Failed to clear recent searches:', error);
    }
  }

  // Orders
  saveOrder(order: OrderResponse): void {
    try {
      const orders = this.getOrders();

      // Check if order already exists (by orderId)
      const existingIndex = orders.findIndex(
        (item) => item.order.orderId === order.orderId
      );

      if (existingIndex !== -1) {
        // Update existing order in place, preserving original timestamp
        orders[existingIndex] = {
          timestamp: orders[existingIndex].timestamp,
          order,
        };
        localStorage.setItem(ORDERS_KEY, JSON.stringify(orders));
      } else {
        // Add new order to beginning and limit to MAX_ORDERS
        const newOrder: StoredOrder = {
          timestamp: Date.now(),
          order,
        };
        const updatedOrders = [newOrder, ...orders].slice(0, MAX_ORDERS);
        localStorage.setItem(ORDERS_KEY, JSON.stringify(updatedOrders));
      }
    } catch (error) {
      console.error('Failed to save order:', error);
    }
  }

  getOrders(): StoredOrder[] {
    try {
      const data = localStorage.getItem(ORDERS_KEY);
      if (!data) return [];

      const orders = JSON.parse(data);
      return Array.isArray(orders) ? orders : [];
    } catch (error) {
      console.error('Failed to get orders:', error);
      return [];
    }
  }

  deleteOrder(orderId: string): void {
    try {
      const orders = this.getOrders();
      const filteredOrders = orders.filter(
        (item) => item.order.orderId !== orderId
      );
      localStorage.setItem(ORDERS_KEY, JSON.stringify(filteredOrders));
    } catch (error) {
      console.error('Failed to delete order:', error);
    }
  }

  clearOrders(): void {
    try {
      localStorage.removeItem(ORDERS_KEY);
    } catch (error) {
      console.error('Failed to clear orders:', error);
    }
  }
}

export const localStorageService = new LocalStorageService();
export type { StoredSearch, StoredOrder };
