import type { OrderRequest, QuoteResponse } from '../types/api';
import { formatInteropAddress, fromInteropAddress } from '../utils/interopAddress';
import { getSignerAddress, signQuote } from '../utils/quoteSigner';
import { getRpcUrlForChain } from '../utils/chainUtils';
import { useWallet } from '../contexts/WalletContext';
import { useEffect, useState } from 'react';

import type { Hex } from 'viem';

interface OrderSubmissionProps {
  selectedQuote: QuoteResponse;
  onSubmit: (request: OrderRequest) => void;
  onBack: () => void;
  isLoading: boolean;
}

export default function OrderSubmission({ selectedQuote, onSubmit, onBack, isLoading }: OrderSubmissionProps) {
  const { isConnected, address, signTypedData, isSigning: walletIsSigning } = useWallet();
  const [privateKey, setPrivateKey] = useState('');
  const [signature, setSignature] = useState('');
  const [signerAddress, setSignerAddress] = useState('');
  const [signingError, setSigningError] = useState('');
  const [isSigning, setIsSigning] = useState(false);
  const [metadataJson, setMetadataJson] = useState('');
  const [showMetadata, setShowMetadata] = useState(false);
  const [remainingTime, setRemainingTime] = useState<number>(0);

  // Calculate and update remaining time
  useEffect(() => {
    const calculateRemainingTime = () => {
      if (!selectedQuote.validUntil) return 0;
      const now = Math.floor(Date.now() / 1000); // Current time in seconds
      const validUntil = selectedQuote.validUntil;
      const remaining = validUntil - now;
      return Math.max(0, remaining); // Don't go negative
    };

    // Initial calculation
    setRemainingTime(calculateRemainingTime());

    // Update every second
    const intervalId = setInterval(() => {
      setRemainingTime(calculateRemainingTime());
    }, 1000);

    return () => clearInterval(intervalId);
  }, [selectedQuote.validUntil]);

  const isQuoteExpired = remainingTime === 0;

  // Format remaining time as MM:SS
  const formatRemainingTime = (seconds: number): string => {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins}:${secs.toString().padStart(2, '0')}`;
  };

  // Get color for timer based on remaining time
  const getTimerColor = () => {
    if (remainingTime === 0) return 'text-red-700 dark:text-red-400';
    if (remainingTime < 30) return 'text-yellow-700 dark:text-yellow-400';
    return 'text-green-700 dark:text-green-400';
  };

  // Sign the quote - use wallet by default, private key as override
  const handleSignQuote = async () => {
    setIsSigning(true);
    setSigningError('');

    try {
      let sig: string;
      let signerAddr: string;

      // Get EIP-712 data from the quote
      const orderPayload = selectedQuote.order.payload;
      if (!orderPayload || typeof orderPayload !== 'object' || !('signatureType' in orderPayload)) {
        throw new Error('Invalid order payload: EIP-712 data not found');
      }

      const eip712Data = orderPayload as {
        signatureType: 'eip712';
        domain: any;
        primaryType: string;
        message: any;
        types: Record<string, Array<{ name: string; type: string }>>;
      };

      if (eip712Data.signatureType !== 'eip712') {
        throw new Error(`Unsupported signature type: ${eip712Data.signatureType}`);
      }

      if (privateKey.trim()) {
        // Use private key override
        let formattedKey = privateKey.trim();
        if (!formattedKey.startsWith('0x')) {
          formattedKey = `0x${formattedKey}`;
        }

        // Get signer address for verification
        signerAddr = getSignerAddress(formattedKey as Hex);
        setSignerAddress(signerAddr);

        // Get chain ID from the order payload to configure the correct RPC
        const chainId = typeof eip712Data.domain.chainId === 'string' 
          ? parseInt(eip712Data.domain.chainId) 
          : Number(eip712Data.domain.chainId);
        
        const rpcUrl = getRpcUrlForChain(chainId);

        // Sign the quote with private key using the EIP-712 data from the quote
        sig = await signQuote(
          selectedQuote as any, // Cast to Quote type from quoteSigner
          formattedKey as Hex,
          {
            rpcUrl  // Pass RPC URL to fetch domain separator from contract!
          }
        );
      } else if (isConnected && address) {
        // Use wallet signing (default)
        signerAddr = address;
        setSignerAddress(signerAddr);

        // Get the chain ID from the order payload to configure the correct RPC
        const chainId = typeof eip712Data.domain.chainId === 'string' 
          ? parseInt(eip712Data.domain.chainId) 
          : Number(eip712Data.domain.chainId);
        
        const rpcUrl = getRpcUrlForChain(chainId);
        
        // Use unified signQuote with wallet signing function
        sig = await signQuote(
          selectedQuote as any,
          undefined, // No private key
          {
            rpcUrl,
            walletSignTypedData: signTypedData
          }
        );
      } else {
        throw new Error('Please connect a wallet or enter a private key');
      }

      setSignature(sig);
      setSigningError('');
      } catch (error) {
        const errorMsg = error instanceof Error ? error.message : 'Failed to sign quote';
        setSigningError(errorMsg);
        setSignature('');
        setSignerAddress('');
      } finally {
      setIsSigning(false);
    }
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();

    if (!signature.trim()) {
      alert('Please sign the quote before submitting');
      return;
    }

    let metadata: unknown = undefined;
    if (showMetadata && metadataJson.trim()) {
      try {
        metadata = JSON.parse(metadataJson);
      } catch (error) {
        alert('Invalid JSON in metadata field');
        return;
      }
    }

    const request: OrderRequest = {
      quoteResponse: selectedQuote,
      signature: signature.trim(),
      metadata
    };

    onSubmit(request);
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

  // Extract user address from quote preview
  const userAddress = selectedQuote.preview?.inputs?.[0]?.user 
    ? fromInteropAddress(selectedQuote.preview.inputs[0].user).address
    : '';

  return (
    <form onSubmit={handleSubmit} className="space-y-4 max-w-3xl mx-auto">
      <div className="card py-4">
        <div className="flex justify-between items-center mb-4">
          <h2 className="text-2xl font-bold text-slate-900 dark:text-white">Submit Order</h2>
          <button type="button" onClick={onBack} className="btn-secondary">
            ← Back to Quotes
          </button>
        </div>

        {/* Quote Validity Timer */}
        <div className={`rounded-lg p-3 mb-4 ${
          isQuoteExpired 
            ? 'bg-red-100 dark:bg-red-900/20 border border-red-300 dark:border-red-700' 
            : remainingTime < 30
            ? 'bg-yellow-100 dark:bg-yellow-900/20 border border-yellow-300 dark:border-yellow-700'
            : 'bg-green-100 dark:bg-green-900/20 border border-green-300 dark:border-green-700'
        }`}>
          <div className="flex items-center justify-between">
            <div>
              <p className={`text-sm font-semibold ${getTimerColor()}`}>
                {isQuoteExpired ? '⏰ Quote Expired' : '⏱ Quote Valid For:'}
              </p>
              {isQuoteExpired && (
                <p className="text-xs text-red-700 dark:text-red-400 mt-1">
                  This quote is no longer valid. Please go back and request a new quote.
                </p>
              )}
            </div>
            <div className={`text-2xl font-mono font-bold ${getTimerColor()}`}>
              {isQuoteExpired ? 'EXPIRED' : formatRemainingTime(remainingTime)}
            </div>
          </div>
          {!isQuoteExpired && remainingTime < 30 && (
            <p className="text-xs text-yellow-700 dark:text-yellow-400 mt-2">
              ⚠️ Quote expires soon! Complete signing and submission quickly.
            </p>
          )}
        </div>

        {/* Selected Quote Summary */}
        <div className="bg-slate-100 dark:bg-slate-900 rounded-lg p-4 mb-6 border border-slate-300 dark:border-slate-700">
          <h3 className="text-lg font-semibold text-slate-900 dark:text-white mb-3">Selected Quote</h3>
          
          <div className="space-y-3">
            <div className="grid grid-cols-2 gap-4">
              <div>
                <p className="text-xs text-slate-600 dark:text-slate-400">Provider</p>
                <p className="text-slate-900 dark:text-white">{selectedQuote.provider || 'Unknown'}</p>
              </div>
              <div>
                <p className="text-xs text-slate-600 dark:text-slate-400">Solver ID</p>
                <p className="text-slate-900 dark:text-white">{selectedQuote.solverId}</p>
              </div>
              <div>
                <p className="text-xs text-slate-600 dark:text-slate-400">Quote ID</p>
                <p className="text-slate-900 dark:text-white text-sm truncate">{selectedQuote.quoteId}</p>
              </div>
              {selectedQuote.eta && (
                <div>
                  <p className="text-xs text-slate-600 dark:text-slate-400">ETA</p>
                  <p className="text-slate-900 dark:text-white">{selectedQuote.eta}s</p>
                </div>
              )}
            </div>

            {/* Inputs */}
            <div>
              <p className="text-sm font-medium text-slate-700 dark:text-slate-300 mb-2">Inputs</p>
              {selectedQuote.preview.inputs.map((input, idx) => (
                <div key={idx} className="bg-slate-200 dark:bg-slate-800 rounded p-2 mb-1 text-sm border border-slate-300 dark:border-slate-700">
                  <p className="text-slate-600 dark:text-slate-400">
                    Amount: <span className="text-slate-900 dark:text-white">{formatAmount(input.amount)}</span>
                  </p>
                  <p className="text-slate-500 dark:text-slate-500 text-xs truncate">
                    Asset: {formatInteropAddress(input.asset)}
                  </p>
                </div>
              ))}
            </div>

            {/* Outputs */}
            <div>
              <p className="text-sm font-medium text-slate-700 dark:text-slate-300 mb-2">Outputs</p>
              {selectedQuote.preview.outputs.map((output, idx) => (
                <div key={idx} className="bg-slate-200 dark:bg-slate-800 rounded p-2 mb-1 text-sm border border-slate-300 dark:border-slate-700">
                  <p className="text-slate-600 dark:text-slate-400">
                    Amount: <span className="text-slate-900 dark:text-white">{formatAmount(output.amount)}</span>
                  </p>
                  <p className="text-slate-500 dark:text-slate-500 text-xs truncate">
                    Asset: {formatInteropAddress(output.asset)}
                  </p>
                </div>
              ))}
            </div>

            {/* Additional Info */}
            <div className="border-t border-slate-300 dark:border-slate-700 pt-3 text-sm">
              <p className="text-slate-600 dark:text-slate-400">
                Partial Fill: <span className="text-slate-900 dark:text-white">{selectedQuote.partialFill ? 'Yes' : 'No'}</span>
              </p>
              {selectedQuote.failureHandling && (
                <p className="text-slate-600 dark:text-slate-400">
                  Failure Handling: <span className="text-slate-900 dark:text-white">{selectedQuote.failureHandling}</span>
                </p>
              )}
              <p className="text-slate-600 dark:text-slate-400">
                Order Type: <span className="text-slate-900 dark:text-white">{selectedQuote.order.type}</span>
              </p>
            </div>
          </div>
        </div>

        {/* Signing Section */}
        <div className="mb-6">
          <div className="flex items-center justify-between mb-3">
            <h3 className="text-lg font-semibold text-slate-900 dark:text-white">Sign Quote</h3>
            {isConnected ? (
              <span className="text-xs text-green-600 dark:text-green-400">✓ Wallet Connected</span>
            ) : (
              <span className="text-xs text-slate-500">Manual signing required</span>
            )}
          </div>

          {/* Private Key Override */}
          <div className="mb-4">
            <label className="label-text">
              Private Key Override <span className="text-slate-500">(optional)</span>
            </label>
            <input
              type="password"
              value={privateKey}
              onChange={(e) => setPrivateKey(e.target.value)}
              placeholder={isConnected ? "Leave empty to use wallet signing" : "0x... (Required if wallet not connected)"}
              className={`input-field font-mono text-sm mb-2 ${isConnected ? 'bg-blue-50 dark:bg-blue-900/20 border-blue-300 dark:border-blue-700' : ''}`}
              disabled={isSigning || walletIsSigning}
            />
            <div className="text-xs text-slate-500 mb-3">
              {isConnected ? (
                <>💡 <strong>Wallet connected:</strong> Leave empty to sign with your wallet, or enter private key to override</>
              ) : (
                <>⚠️ <strong>No wallet connected:</strong> Enter private key for manual signing</>
              )}
            </div>
            
            {privateKey.trim() && (
              <div className="bg-red-100 dark:bg-red-900/20 border border-red-300 dark:border-red-700 rounded p-3 mb-3">
                <p className="text-red-700 dark:text-red-400 text-xs">
                  ⚠️ <strong>WARNING:</strong> Never share your private key! This is for testing only.
                  In production, use wallet integration (MetaMask, WalletConnect, etc.).
                </p>
              </div>
            )}
          </div>
          
          <button
            type="button"
            onClick={handleSignQuote}
            disabled={isSigning || walletIsSigning || isLoading || isQuoteExpired || (!privateKey.trim() && !isConnected)}
            className="btn-secondary w-full mb-3"
          >
            {isSigning || walletIsSigning ? (
              'Signing...'
            ) : isQuoteExpired ? (
              'Quote Expired'
            ) : privateKey.trim() ? (
              'Sign Quote with Private Key'
            ) : isConnected ? (
              'Sign Quote with Wallet'
            ) : (
              'Connect Wallet or Enter Private Key'
            )}
          </button>
          
          {isQuoteExpired && (
            <p className="text-red-700 dark:text-red-400 text-xs mb-3">
              Cannot sign expired quote. Please request a new quote.
            </p>
          )}

          {signerAddress && (
            <>
              <div className="bg-green-100 dark:bg-green-900/20 border border-green-300 dark:border-green-700 rounded p-3 mb-2">
                <p className="text-green-700 dark:text-green-400 text-xs">
                  ✓ Signer Address: <span className="font-mono">{signerAddress}</span>
                </p>
              </div>
              
              {/* Address Validation Warning */}
              {userAddress && signerAddress.toLowerCase() !== userAddress.toLowerCase() && (
                <div className="bg-yellow-100 dark:bg-yellow-900/20 border border-yellow-300 dark:border-yellow-700 rounded p-3 mb-2">
                  <p className="text-yellow-800 dark:text-yellow-400 text-xs font-semibold mb-1">
                    ⚠️ Address Mismatch Warning
                  </p>
                  <p className="text-yellow-800 dark:text-yellow-400 text-xs">
                    The signer address doesn't match the user address from the quote request.
                  </p>
                  <p className="text-yellow-800 dark:text-yellow-400 text-xs mt-1">
                    Expected: <span className="font-mono">{userAddress}</span>
                  </p>
                  <p className="text-yellow-800 dark:text-yellow-400 text-xs">
                    Signing with: <span className="font-mono">{signerAddress}</span>
                  </p>
                </div>
              )}
              
              {userAddress && signerAddress.toLowerCase() === userAddress.toLowerCase() && (
                <div className="bg-blue-100 dark:bg-blue-900/20 border border-blue-300 dark:border-blue-700 rounded p-3 mb-2">
                  <p className="text-blue-700 dark:text-blue-400 text-xs">
                    ✓ Signer address matches quote request user address
                  </p>
                </div>
              )}
            </>
          )}

          {signingError && (
            <div className="bg-red-100 dark:bg-red-900/20 border border-red-300 dark:border-red-700 rounded p-3 mb-2">
              <p className="text-red-700 dark:text-red-400 text-xs">
                ✗ Error: {signingError}
              </p>
            </div>
          )}
        </div>

        {/* Generated Signature */}
        {signature && (
          <div className="mb-6">
            <label className="label-text">
              Generated Signature
            </label>
            <textarea
              value={signature}
              readOnly
              rows={4}
              className="input-field font-mono text-sm bg-slate-100 dark:bg-slate-900"
            />
            <p className="text-xs text-green-600 dark:text-green-400 mt-1">
              ✓ Signature generated successfully! You can now submit the order.
            </p>
          </div>
        )}

        {/* Optional Metadata */}
        <div className="mb-6">
          <button
            type="button"
            onClick={() => setShowMetadata(!showMetadata)}
            className="flex items-center justify-between w-full text-left text-slate-900 dark:text-white hover:text-primary-600 dark:hover:text-primary-400 transition-colors mb-2"
          >
            <span className="text-sm font-medium">Optional: Custom Metadata (JSON)</span>
            <span className="text-xl">{showMetadata ? '−' : '+'}</span>
          </button>
          {showMetadata && (
            <textarea
              value={metadataJson}
              onChange={(e) => setMetadataJson(e.target.value)}
              placeholder='{"sponsor": "0x...", "custom": "value"}'
              rows={6}
              className="input-field font-mono text-sm"
            />
          )}
        </div>

        {/* Submit Button */}
        <button
          type="submit"
          disabled={isLoading || !signature.trim() || isQuoteExpired}
          className="btn-primary w-full py-3 text-lg"
        >
          {isLoading ? 'Submitting Order...' : isQuoteExpired ? 'Quote Expired - Cannot Submit' : 'Submit Signed Order'}
        </button>

        {!signature && !isQuoteExpired && (
          <p className="text-yellow-400 text-sm text-center mt-3">
            ⚠️ Please sign the quote before submitting
          </p>
        )}
        
        {isQuoteExpired && (
          <p className="text-red-400 text-sm text-center mt-3">
            ⏰ This quote has expired. Please go back and request a new quote.
          </p>
        )}
      </div>
    </form>
  );
}

