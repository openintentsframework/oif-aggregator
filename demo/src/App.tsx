import type { OrderRequest, OrderResponse, QuoteRequest, QuoteResponse, QuotesResponse } from './types/api';
import { orderApi, quoteApi } from './services/api';
import { useEffect, useState } from 'react';

import HealthWidget from './components/HealthWidget';
import MyOrders from './components/MyOrders';
import OrderStatus from './components/OrderStatus';
import OrderSubmission from './components/OrderSubmission';
import QuoteRequestForm from './components/QuoteRequestForm';
import QuoteResults from './components/QuoteResults';
import Settings from './components/Settings';
import SimpleQuoteForm from './components/SimpleQuoteForm';
import SolverDetail from './components/SolverDetail';
import SolversList from './components/SolversList';
import CustomWalletButton from './components/CustomWalletButton';
import { localStorageService } from './services/localStorageService';
import { solverDataService } from './services/solverDataService';
import { useTheme } from './contexts/ThemeContext';

type Step = 'request' | 'quotes' | 'order' | 'status' | 'solvers' | 'solver-detail' | 'my-orders' | 'settings';

function App() {
  const { theme, toggleTheme } = useTheme();
  const [step, setStep] = useState<Step>('request');
  const [quotes, setQuotes] = useState<QuotesResponse | null>(null);
  const [selectedQuote, setSelectedQuote] = useState<QuoteResponse | null>(null);
  const [order, setOrder] = useState<OrderResponse | null>(null);
  const [selectedSolverId, setSelectedSolverId] = useState<string | null>(null);
  const [lastQuoteRequest, setLastQuoteRequest] = useState<QuoteRequest | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  
  // Data loading state
  const [isLoadingData, setIsLoadingData] = useState(true);
  const [dataError, setDataError] = useState<string | null>(null);
  const [useAdvancedMode, setUseAdvancedMode] = useState(false);

  // Load solver data on mount
  useEffect(() => {
    const initializeSolverData = async () => {
      setIsLoadingData(true);
      setDataError(null);
      try {
        await solverDataService.fetchSolverData();
      } catch (error) {
        console.error('Failed to load solver data:', error);
        setDataError((error as Error).message);
        // Enable advanced mode as fallback
        setUseAdvancedMode(true);
      } finally {
        setIsLoadingData(false);
      }
    };

    initializeSolverData();
  }, []);

  const handleQuoteRequest = async (request: QuoteRequest) => {
    setIsLoading(true);
    setError(null);
    try {
      const response = await quoteApi.getQuotes(request);
      setQuotes(response);
      setLastQuoteRequest(request); // Store the request for refresh
      setStep('quotes');
    } catch (err) {
      setError((err as Error).message);
      alert(`Error: ${(err as Error).message}`);
    } finally {
      setIsLoading(false);
    }
  };

  const handleRefreshQuotes = async () => {
    if (!lastQuoteRequest) return;
    
    setIsLoading(true);
    setError(null);
    try {
      const response = await quoteApi.getQuotes(lastQuoteRequest);
      setQuotes(response);
    } catch (err) {
      setError((err as Error).message);
      alert(`Error refreshing quotes: ${(err as Error).message}`);
    } finally {
      setIsLoading(false);
    }
  };

  const handleSelectQuote = (quote: QuoteResponse) => {
    setSelectedQuote(quote);
    setStep('order');
  };

  const handleOrderSubmission = async (request: OrderRequest) => {
    setIsLoading(true);
    setError(null);
    try {
      const response = await orderApi.submitOrder(request);
      localStorageService.saveOrder(response);
      setOrder(response);
      setStep('status');
    } catch (err) {
      setError((err as Error).message);
      alert(`Error: ${(err as Error).message}`);
    } finally {
      setIsLoading(false);
    }
  };

  const handleStartOver = () => {
    setStep('request');
    setQuotes(null);
    setSelectedQuote(null);
    setOrder(null);
    setError(null);
  };

  const handleBackToRequest = () => {
    setStep('request');
    setQuotes(null);
    setSelectedQuote(null);
  };

  const handleBackToQuotes = () => {
    setStep('quotes');
    setSelectedQuote(null);
  };

  const handleOrderUpdate = (updatedOrder: OrderResponse) => {
    setOrder(updatedOrder);
    // Update localStorage with the latest order status
    localStorageService.saveOrder(updatedOrder);
  };

  const handleViewSolvers = () => {
    setStep('solvers');
    setError(null);
  };

  const handleSelectSolver = (solverId: string) => {
    setSelectedSolverId(solverId);
    setStep('solver-detail');
  };

  const handleBackToSolvers = () => {
    setSelectedSolverId(null);
    setStep('solvers');
  };

  const handleViewMyOrders = () => {
    setStep('my-orders');
    setError(null);
  };

  const handleViewOrderFromHistory = (orderFromHistory: OrderResponse) => {
    setOrder(orderFromHistory);
    setStep('status');
  };

  const handleViewSettings = () => {
    setStep('settings');
    setError(null);
  };

  const handleSettingsSave = async () => {
    setIsLoadingData(true);
    setDataError(null);
    try {
      // Force refresh to fetch from new URL
      await solverDataService.fetchSolverData(true);
    } catch (error) {
      console.error('Failed to reload solver data:', error);
      setDataError((error as Error).message);
    } finally {
      setIsLoadingData(false);
    }
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-50 via-slate-100 to-slate-50 dark:from-slate-900 dark:via-slate-800 dark:to-slate-900">
      {/* Header */}
      <header className="border-b border-slate-200 dark:border-slate-700 bg-white/50 dark:bg-slate-900/50 backdrop-blur">
        <div className="container mx-auto px-4 py-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-6">
              <div>
                <h1 className="text-2xl font-bold text-slate-900 dark:text-white">OIF Aggregator</h1>
                <p className="text-slate-600 dark:text-slate-400 text-xs">Testing Interface for Quote & Order API</p>
              </div>
              {/* Navigation */}
              <nav className="flex items-center gap-2">
                <button
                  onClick={() => setStep('request')}
                  className={`px-3 py-1.5 text-sm rounded transition-colors ${
                    step === 'request' || step === 'quotes' || step === 'order' || step === 'status'
                      ? 'bg-primary-600 text-white'
                      : 'bg-slate-200 dark:bg-slate-800 text-slate-700 dark:text-slate-400 hover:bg-slate-300 dark:hover:bg-slate-700'
                  }`}
                >
                  🔄 Quotes
                </button>
                <button
                  onClick={handleViewSolvers}
                  className={`px-3 py-1.5 text-sm rounded transition-colors ${
                    step === 'solvers' || step === 'solver-detail'
                      ? 'bg-primary-600 text-white'
                      : 'bg-slate-200 dark:bg-slate-800 text-slate-700 dark:text-slate-400 hover:bg-slate-300 dark:hover:bg-slate-700'
                  }`}
                >
                  🔍 Solvers
                </button>
                <button
                  onClick={handleViewMyOrders}
                  className={`px-3 py-1.5 text-sm rounded transition-colors ${
                    step === 'my-orders'
                      ? 'bg-primary-600 text-white'
                      : 'bg-slate-200 dark:bg-slate-800 text-slate-700 dark:text-slate-400 hover:bg-slate-300 dark:hover:bg-slate-700'
                  }`}
                >
                  📋 My Orders
                </button>
                <button
                  onClick={handleViewSettings}
                  className={`px-3 py-1.5 text-sm rounded transition-colors ${
                    step === 'settings'
                      ? 'bg-primary-600 text-white'
                      : 'bg-slate-200 dark:bg-slate-800 text-slate-700 dark:text-slate-400 hover:bg-slate-300 dark:hover:bg-slate-700'
                  }`}
                >
                  ⚙️ Settings
                </button>
              </nav>
            </div>
            <div className="flex items-center gap-4">
              {/* Step Indicator - only show for quote flow */}
              {(step === 'request' || step === 'quotes' || step === 'order' || step === 'status') && (
                <div className="hidden md:flex items-center gap-2 text-sm">
                  <div className={`px-3 py-1 rounded ${step === 'request' ? 'bg-primary-600 text-white' : 'bg-slate-200 dark:bg-slate-800 text-slate-700 dark:text-slate-400'}`}>
                    1. Request
                  </div>
                  <div className="text-slate-400 dark:text-slate-600">→</div>
                  <div className={`px-3 py-1 rounded ${step === 'quotes' ? 'bg-primary-600 text-white' : 'bg-slate-200 dark:bg-slate-800 text-slate-700 dark:text-slate-400'}`}>
                    2. Quotes
                  </div>
                  <div className="text-slate-400 dark:text-slate-600">→</div>
                  <div className={`px-3 py-1 rounded ${step === 'order' ? 'bg-primary-600 text-white' : 'bg-slate-200 dark:bg-slate-800 text-slate-700 dark:text-slate-400'}`}>
                    3. Order
                  </div>
                  <div className="text-slate-400 dark:text-slate-600">→</div>
                  <div className={`px-3 py-1 rounded ${step === 'status' ? 'bg-primary-600 text-white' : 'bg-slate-200 dark:bg-slate-800 text-slate-700 dark:text-slate-400'}`}>
                    4. Status
                  </div>
                </div>
              )}
              {/* Wallet Connect Button */}
              <CustomWalletButton />
              {/* Theme Switcher */}
              <button
                onClick={toggleTheme}
                className="px-3 py-1 text-sm rounded bg-slate-200 dark:bg-slate-800 text-slate-700 dark:text-slate-300 hover:bg-slate-300 dark:hover:bg-slate-700 transition-colors"
                title={`Switch to ${theme === 'light' ? 'dark' : 'light'} mode`}
              >
                {theme === 'light' ? '🌙' : '☀️'}
              </button>
            </div>
          </div>
        </div>
      </header>

      {/* Main Content */}
      <main className="container mx-auto px-4 py-4">
        {/* Data Loading */}
        {isLoadingData && (
          <div className="card text-center py-12">
            <div className="animate-spin h-12 w-12 border-4 border-primary-600 border-t-transparent rounded-full mx-auto mb-4"></div>
            <p className="text-slate-900 dark:text-white text-lg">Loading solver data...</p>
            <p className="text-slate-600 dark:text-slate-400 text-sm mt-2">Fetching available assets and routes</p>
          </div>
        )}

        {/* Data Error */}
        {dataError && !isLoadingData && (
          <div className="mb-6 bg-yellow-50 dark:bg-yellow-900/20 border-2 border-yellow-300 dark:border-yellow-700 rounded-lg p-4">
            <p className="text-yellow-800 dark:text-yellow-400">
              <strong>Warning:</strong> Failed to load solver data. {dataError}
            </p>
            <p className="text-slate-700 dark:text-slate-400 text-sm mt-2">
              Using advanced mode with manual address input.
            </p>
          </div>
        )}

        {error && (
          <div className="mb-6 bg-red-50 dark:bg-red-900/20 border-2 border-red-300 dark:border-red-700 rounded-lg p-4">
            <p className="text-red-800 dark:text-red-400">
              <strong>Error:</strong> {error}
            </p>
          </div>
        )}

        {!isLoadingData && step === 'request' && (
          <>
            {/* Mode Toggle */}
            {!dataError && (
              <div className="mb-4 flex justify-end">
                <button
                  onClick={() => setUseAdvancedMode(!useAdvancedMode)}
                  className="text-sm text-slate-600 dark:text-slate-400 hover:text-primary-600 dark:hover:text-primary-400 transition-colors"
                >
                  {useAdvancedMode ? '← Switch to Simple Mode' : 'Advanced Mode →'}
                </button>
              </div>
            )}

            {useAdvancedMode ? (
              <QuoteRequestForm onSubmit={handleQuoteRequest} isLoading={isLoading} />
            ) : (
              <SimpleQuoteForm onSubmit={handleQuoteRequest} isLoading={isLoading} />
            )}
          </>
        )}

        {step === 'quotes' && quotes && (
          <QuoteResults
            response={quotes}
            onSelectQuote={handleSelectQuote}
            onBack={handleBackToRequest}
            onRefresh={handleRefreshQuotes}
            isRefreshing={isLoading}
          />
        )}

        {step === 'order' && selectedQuote && (
          <OrderSubmission
            selectedQuote={selectedQuote}
            onSubmit={handleOrderSubmission}
            onBack={handleBackToQuotes}
            isLoading={isLoading}
          />
        )}

        {step === 'status' && order && (
          <OrderStatus
            order={order}
            onStartOver={handleStartOver}
            onOrderUpdate={handleOrderUpdate}
          />
        )}

        {step === 'solvers' && (
          <SolversList onSelectSolver={handleSelectSolver} />
        )}

        {step === 'solver-detail' && selectedSolverId && (
          <SolverDetail
            solverId={selectedSolverId}
            onBack={handleBackToSolvers}
          />
        )}

        {step === 'my-orders' && (
          <MyOrders
            onViewOrder={handleViewOrderFromHistory}
            onBack={() => setStep('request')}
          />
        )}

        {step === 'settings' && (
          <Settings onUrlChange={handleSettingsSave} />
        )}
      </main>

      {/* Footer */}
      <footer className="border-t border-slate-200 dark:border-slate-700 bg-white/50 dark:bg-slate-900/50 backdrop-blur mt-12">
        <div className="container mx-auto px-4 py-3">
          <div className="flex items-center justify-between text-sm text-slate-600 dark:text-slate-400">
            <p>OIF Aggregator Testing UI - Built with React + TypeScript + Tailwind</p>
            <div className="flex items-center gap-4">
              <a
                href="https://github.com/openintentsframework/oif-aggregator"
                target="_blank"
                rel="noopener noreferrer"
                className="hover:text-primary-500 dark:hover:text-primary-400 transition-colors"
              >
                GitHub
              </a>
            </div>
          </div>
        </div>
      </footer>

      {/* Health Widget */}
      <HealthWidget />
    </div>
  );
}

export default App;

