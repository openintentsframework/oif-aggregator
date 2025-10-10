/**
 * This file will be auto-generated from OpenAPI spec.
 * Run: npm run generate-types
 * 
 * For now, we define the core types manually based on the OpenAPI schema
 */

import { InteropAddress } from '../utils/interopAddress';

// Core request/response types based on OpenAPI spec

export interface QuoteRequest {
  user: InteropAddress;
  intent: IntentRequest;
  supportedTypes: string[];
  metadata?: unknown;
  solverOptions?: SolverOptions;
}

export interface IntentRequest {
  intentType: 'oif-swap';
  inputs: Input[];
  outputs: Output[];
  swapType?: 'exact-input' | 'exact-output';
  preference?: 'price' | 'speed' | 'inputPriority' | 'trustMinimization';
  partialFill?: boolean;
  minValidUntil?: number;
  failureHandling?: ('refund-automatic' | 'refund-claim' | 'needs-new-signature')[];
  originSubmission?: OriginSubmission;
  metadata?: unknown;
}

export interface Input {
  user: InteropAddress;
  asset: InteropAddress;
  amount?: string;
  lock?: AssetLockReference;
}

export interface Output {
  receiver: InteropAddress;
  asset: InteropAddress;
  amount?: string;
  calldata?: string;
}

export interface AssetLockReference {
  kind: 'the-compact' | 'rhinestone';
  params?: unknown;
}

export interface OriginSubmission {
  mode: 'user' | 'protocol';
  schemes?: ('erc4337' | 'permit2' | 'erc20-permit' | 'eip3009')[];
}

export interface SolverOptions {
  timeout?: number;
  solverTimeout?: number;
  minQuotes?: number;
  solverSelection?: 'all' | 'sampled' | 'priority';
  includeSolvers?: string[];
  excludeSolvers?: string[];
  sampleSize?: number;
  priorityThreshold?: number;
}

export interface QuotesResponse {
  quotes: QuoteResponse[];
  totalQuotes: number;
  metadata?: AggregationMetadata;
}

export interface QuoteResponse {
  quoteId: string;
  solverId: string;
  order: Order;
  partialFill: boolean;
  preview: QuotePreview;
  integrityChecksum: string;
  provider?: string;
  eta?: number;
  failureHandling?: 'refund-automatic' | 'refund-claim' | 'needs-new-signature';
  validUntil?: number;
  metadata?: unknown;
}

export interface QuotePreview {
  inputs: Input[];
  outputs: Output[];
}

export interface Order {
  type: string;
  payload: OrderPayload | unknown;
  metadata?: unknown;
}

export interface OrderPayload {
  signatureType: 'eip712' | 'eip3009';
  domain: unknown;
  primaryType: string;
  message: unknown;
  types: Record<string, Array<{ name: string; type: string }>>;
}

export interface AggregationMetadata {
  totalDurationMs: number;
  solverTimeoutMs: number;
  globalTimeoutMs: number;
  earlyTermination: boolean;
  totalSolversAvailable: number;
  solversQueried: number;
  solversRespondedSuccess: number;
  solversRespondedError: number;
  solversTimedOut: number;
  minQuotesRequired: number;
  solverSelectionMode: string;
}

export interface OrderRequest {
  quoteResponse: QuoteResponse;
  signature: string;
  metadata?: unknown;
  originSubmission?: OriginSubmission;
}

export interface OrderResponse {
  orderId: string;
  status: OrderStatus;
  createdAt: string;
  updatedAt: string;
  inputAmounts: AssetAmount[];
  outputAmounts: AssetAmount[];
  settlement: Settlement;
  quoteId?: string;
  fillTransaction?: unknown;
}

export type OrderStatus =
  | 'created'
  | 'pending'
  | 'executing'
  | 'executed'
  | 'settled'
  | 'settling'
  | 'finalized'
  | 'failed'
  | 'refunded';

export interface AssetAmount {
  asset: InteropAddress;
  amount?: string;
}

export interface Settlement {
  type: 'escrow' | 'resourceLock';
  data: unknown;
}

export interface ErrorResponse {
  error: string;
  message: string;
  timestamp: number;
}

// Solver types

export type SolverStatus = 'active' | 'inactive' | 'circuit-breaker-open' | 'unknown';

export interface AssetInfo {
  address: string;
  chainId: number;
  symbol: string;
  name: string;
  decimals: number;
}

export interface RouteInfo {
  originChainId: number;
  originTokenAddress: string;
  originTokenSymbol?: string;
  destinationChainId: number;
  destinationTokenAddress: string;
  destinationTokenSymbol?: string;
}

export interface SupportedAssetsResponse {
  type: 'assets' | 'routes';
  source: 'autoDiscovered' | 'manual';
  assets?: AssetInfo[];
  routes?: RouteInfo[];
}

export interface SolverResponse {
  solverId: string;
  adapterId: string;
  endpoint: string;
  status: SolverStatus;
  name?: string | null;
  description?: string | null;
  supportedAssets: SupportedAssetsResponse;
  createdAt: string;
  lastSeen?: string | null;
}

export interface SolversListResponse {
  solvers: SolverResponse[];
  totalSolvers: number;
  page?: number;
  pageSize?: number;
}

// Health check types

export interface SolverStats {
  total: number;
  active: number;
  inactive: number;
  healthy: number;
  unhealthy: number;
  healthDetails: Record<string, boolean>;
}

export interface StorageHealthInfo {
  healthy: boolean;
  backend: string;
}

export interface HealthResponse {
  status: string;
  version: string;
  solvers: SolverStats;
  storage: StorageHealthInfo;
}

