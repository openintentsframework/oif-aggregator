import type {
  ErrorResponse,
  HealthResponse,
  OrderRequest,
  OrderResponse,
  QuoteRequest,
  QuotesResponse,
  SolverResponse,
  SolversListResponse
} from '../types/api';
import axios, { AxiosError } from 'axios';

import { settingsService } from './settingsService';

// Initialize with URL from settings (which checks localStorage first)
export const api = axios.create({
  baseURL: settingsService.getAggregatorUrl(),
  headers: {
    'Content-Type': 'application/json'
  },
  timeout: 30000 // 30 second timeout
});

/**
 * Update the API base URL dynamically
 */
export function updateApiBaseUrl(newUrl: string): void {
  api.defaults.baseURL = newUrl;
}

// Add response interceptor for error handling
api.interceptors.response.use(
  (response) => response,
  (error: AxiosError<ErrorResponse>) => {
    if (error.response?.data) {
      // Server returned an error response
      throw new Error(error.response.data.message || 'An error occurred');
    } else if (error.request) {
      // Request was made but no response received
      throw new Error('No response from server. Please check if the server is running.');
    } else {
      // Something else happened
      throw new Error(error.message || 'An unexpected error occurred');
    }
  }
);

/**
 * Quote API methods
 */
export const quoteApi = {
  /**
   * Request quotes from multiple solvers
   */
  getQuotes: async (request: QuoteRequest): Promise<QuotesResponse> => {
    const response = await api.post<QuotesResponse>('/v1/quotes', request);
    return response.data;
  }
};

/**
 * Order API methods
 */
export const orderApi = {
  /**
   * Submit an order for execution
   */
  submitOrder: async (request: OrderRequest): Promise<OrderResponse> => {
    const response = await api.post<OrderResponse>('/v1/orders', request);
    return response.data;
  },

  /**
   * Get order status by ID
   */
  getOrder: async (id: string): Promise<OrderResponse> => {
    const response = await api.get<OrderResponse>(`/v1/orders/${id}`);
    return response.data;
  }
};

/**
 * Health check
 */
export const healthApi = {
  getHealth: async (): Promise<HealthResponse> => {
    const response = await api.get<HealthResponse>('/health');
    return response.data;
  }
};

/**
 * Solver API methods
 */
export const solverApi = {
  /**
   * Get list of all solvers with pagination
   */
  getSolvers: async (page?: number, pageSize?: number): Promise<SolversListResponse> => {
    const params = new URLSearchParams();
    if (page) params.append('page', page.toString());
    if (pageSize) params.append('page_size', pageSize.toString());
    
    const response = await api.get<SolversListResponse>(
      `/v1/solvers${params.toString() ? `?${params.toString()}` : ''}`
    );
    return response.data;
  },

  /**
   * Get details of a specific solver by ID
   */
  getSolverById: async (id: string): Promise<SolverResponse> => {
    const response = await api.get<SolverResponse>(`/v1/solvers/${id}`);
    return response.data;
  }
};

