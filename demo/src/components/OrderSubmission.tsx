import type { OrderRequest, QuoteResponse } from '../types/api';
import { formatInteropAddress, fromInteropAddress } from '../utils/interopAddress';
import { getRpcUrlForChain } from '../utils/chainUtils';
import { getSignerAddress, signQuote } from '../utils/quoteSigner';
import { checkPermit2Approval, requestPermit2Approval, waitForPermit2Transaction } from '../utils/permit2Approval';
import {
  checkCompactAllowance,
  requestCompactApproval,
  checkCompactDeposit,
  requestCompactDeposit,
  waitForCompactTransaction,
} from '../utils/compactSetup';
import { useWallet } from '../contexts/WalletContext';
import { useEffect, useMemo, useState } from 'react';
import { useWalletClient } from 'wagmi';

import type { Hex, Address } from 'viem';

interface OrderSubmissionProps {
  selectedQuote: QuoteResponse;
  onSubmit: (request: OrderRequest) => void;
  onBack: () => void;
  isLoading: boolean;
}

interface CompactDetails {
  tokenAddress: Address;
  lockTag: Hex;
  requiredAmount: bigint;
  requiredAmountRaw: string;
  sponsor: Address;
  contractAddress: Address;
  chainId: number;
  rpcUrl?: string;
}

export default function OrderSubmission({ selectedQuote, onSubmit, onBack, isLoading }: OrderSubmissionProps) {
  const { isConnected, address, signTypedData, isSigning: walletIsSigning } = useWallet();
  const { data: walletClient } = useWalletClient();
  const [privateKey, setPrivateKey] = useState('');
  const [signature, setSignature] = useState('');
  const [signerAddress, setSignerAddress] = useState('');
  const [signingError, setSigningError] = useState('');
  const [isSigning, setIsSigning] = useState(false);
  
  // Permit2 approval state
  const [needsPermitApproval, setNeedsPermitApproval] = useState(false);
  const [isCheckingPermitApproval, setIsCheckingPermitApproval] = useState(false);
  const [isPermitApproving, setIsPermitApproving] = useState(false);
  const [permitApprovalTxHash, setPermitApprovalTxHash] = useState<string>('');
  // TheCompact setup state
  const compactDetails = useMemo<CompactDetails | null>(() => {
    if (selectedQuote.order.type !== 'oif-resource-lock-v0') {
      return null;
    }

    try {
      const payload = selectedQuote.order.payload as any;
      const domain = payload?.domain ?? {};
      const message = payload?.message ?? {};
      const commitments = Array.isArray(message?.commitments) ? message.commitments : [];

      if (!domain?.verifyingContract || commitments.length === 0) {
        return null;
      }

      const normalizeAddress = (value: string, label: string): Address => {
        if (typeof value !== 'string') {
          throw new Error(`${label} is missing`);
        }
        const normalized = value.toLowerCase();
        if (!/^0x[0-9a-f]{40}$/.test(normalized)) {
          throw new Error(`Invalid ${label}: ${value}`);
        }
        return normalized as Address;
      };

      const normalizeLockTag = (value: string): Hex => {
        if (typeof value !== 'string') {
          throw new Error('Lock tag is missing');
        }
        const normalized = value.toLowerCase();
        if (!/^0x[0-9a-f]{24}$/.test(normalized)) {
          throw new Error(`Invalid lock tag: ${value}`);
        }
        return normalized as Hex;
      };

      const firstCommitment = commitments[0];
      const tokenAddress = normalizeAddress(firstCommitment.token, 'token address');
      const lockTag = normalizeLockTag(firstCommitment.lockTag);
      const amountRaw = firstCommitment.amount;
      if (typeof amountRaw !== 'string') {
        throw new Error('Commitment amount is missing');
      }

      const requiredAmount = BigInt(amountRaw);

      const domainContract = normalizeAddress(domain.verifyingContract, 'verifying contract');
      const sponsor = normalizeAddress(message?.sponsor, 'sponsor');

      const firstInput = selectedQuote.preview.inputs[0];
      if (!firstInput) {
        throw new Error('Quote missing preview inputs');
      }

      const inputInterop = fromInteropAddress(firstInput.asset);
      const chainId = inputInterop.chainId;
      const rpcUrl = getRpcUrlForChain(chainId);

      return {
        tokenAddress,
        lockTag,
        requiredAmount,
        requiredAmountRaw: amountRaw,
        sponsor,
        contractAddress: domainContract,
        chainId,
        rpcUrl,
      };
    } catch (error) {
      console.error('Failed to parse TheCompact details from quote:', error);
      return null;
    }
  }, [selectedQuote]);

  const [compactAllowance, setCompactAllowance] = useState<{ hasApproval: boolean; currentAllowance: bigint } | null>(null);
  const [compactDepositStatus, setCompactDepositStatus] = useState<{ hasDeposit: boolean; depositedAmount: bigint; lockId: bigint } | null>(null);
  const [isCheckingCompactAllowance, setIsCheckingCompactAllowance] = useState(false);
  const [isCheckingCompactDeposit, setIsCheckingCompactDeposit] = useState(false);
  const [isCompactApproving, setIsCompactApproving] = useState(false);
  const [isDepositing, setIsDepositing] = useState(false);
  const [compactApprovalTxHash, setCompactApprovalTxHash] = useState<string>('');
  const [compactDepositTxHash, setCompactDepositTxHash] = useState<string>('');
  const [compactSetupError, setCompactSetupError] = useState('');

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

  // Check Permit2 approval for wallet signing
  useEffect(() => {
    const checkApproval = async () => {
      // Only check for Permit2 orders when using wallet (not private key override)
      if (selectedQuote.order.type !== 'oif-escrow-v0' || !isConnected || !address || privateKey.trim()) {
        setNeedsPermitApproval(false);
        setPermitApprovalTxHash('');
        return;
      }

      setIsCheckingPermitApproval(true);
      
      try {
        const firstInput = selectedQuote.preview.inputs[0];
        if (!firstInput) return;

        const inputInterop = fromInteropAddress(firstInput.asset);
        const rpcUrl = getRpcUrlForChain(inputInterop.chainId);
        
        const result = await checkPermit2Approval(
          address as Address,
          inputInterop.address as Address,
          inputInterop.chainId,
          rpcUrl
        );

        setNeedsPermitApproval(!result.hasApproval);
      } catch (error) {
        console.error('Failed to check Permit2 approval:', error);
        setNeedsPermitApproval(false);
      } finally {
        setIsCheckingPermitApproval(false);
      }
    };

    checkApproval();
  }, [selectedQuote, isConnected, address, privateKey]);

  // Check TheCompact deposit status (independent of wallet connection)
  useEffect(() => {
    if (selectedQuote.order.type !== 'oif-resource-lock-v0' || !compactDetails) {
      setCompactDepositStatus(null);
      if (selectedQuote.order.type !== 'oif-resource-lock-v0') {
        setCompactSetupError('');
      }
      return;
    }

    let cancelled = false;

    const runDepositCheck = async () => {
      setIsCheckingCompactDeposit(true);
      setCompactSetupError('');
      try {
        const depositResult = await checkCompactDeposit({
          sponsor: compactDetails.sponsor,
          contractAddress: compactDetails.contractAddress,
          lockTag: compactDetails.lockTag,
          tokenAddress: compactDetails.tokenAddress,
          requiredAmount: compactDetails.requiredAmount,
          chainId: compactDetails.chainId,
          rpcUrl: compactDetails.rpcUrl,
        });

        if (!cancelled) {
          setCompactDepositStatus(depositResult);
        }
      } catch (error) {
        if (!cancelled) {
          console.error('Failed to check TheCompact deposit status:', error);
          setCompactSetupError((error as Error).message);
          setCompactDepositStatus(null);
        }
      } finally {
        if (!cancelled) {
          setIsCheckingCompactDeposit(false);
        }
      }
    };

    runDepositCheck();

    return () => {
      cancelled = true;
    };
  }, [selectedQuote, compactDetails]);

  // Check TheCompact token allowance for connected wallet
  useEffect(() => {
    if (
      selectedQuote.order.type !== 'oif-resource-lock-v0' ||
      !compactDetails ||
      !isConnected ||
      !address ||
      privateKey.trim()
    ) {
      setCompactAllowance(null);
      return;
    }

    let cancelled = false;

    const runAllowanceCheck = async () => {
      setIsCheckingCompactAllowance(true);
      setCompactSetupError('');
      try {
        const allowanceResult = await checkCompactAllowance({
          owner: address as Address,
          tokenAddress: compactDetails.tokenAddress,
          contractAddress: compactDetails.contractAddress,
          requiredAmount: compactDetails.requiredAmount,
          chainId: compactDetails.chainId,
          rpcUrl: compactDetails.rpcUrl,
        });

        if (!cancelled) {
          setCompactAllowance(allowanceResult);
        }
      } catch (error) {
        if (!cancelled) {
          console.error('Failed to check TheCompact allowance:', error);
          setCompactSetupError((error as Error).message);
          setCompactAllowance(null);
        }
      } finally {
        if (!cancelled) {
          setIsCheckingCompactAllowance(false);
        }
      }
    };

    runAllowanceCheck();

    return () => {
      cancelled = true;
    };
  }, [selectedQuote, compactDetails, isConnected, address, privateKey]);
  // Handle Permit2 approval
  const handleApprove = async () => {
    if (!walletClient || !address) {
      setSigningError('Wallet not connected');
      return;
    }

    setIsPermitApproving(true);
    setSigningError('');

    try {
      const firstInput = selectedQuote.preview.inputs[0];
      if (!firstInput) {
        throw new Error('No input found in quote');
      }

      const inputInterop = fromInteropAddress(firstInput.asset);
      
      const rpcUrl = getRpcUrlForChain(inputInterop.chainId);

      const txHash = await requestPermit2Approval(
        inputInterop.address as Address,
        inputInterop.chainId,
        walletClient
      );

      setPermitApprovalTxHash(txHash);

      await waitForPermit2Transaction({
        chainId: inputInterop.chainId,
        rpcUrl,
        hash: txHash,
      });

      const approvalStatus = await checkPermit2Approval(
        address as Address,
        inputInterop.address as Address,
        inputInterop.chainId,
        rpcUrl
      );

      setNeedsPermitApproval(!approvalStatus.hasApproval);
      alert(`Approval transaction confirmed! Hash: ${txHash}\n\nYou can now sign the quote.`);
    } catch (error) {
      console.error('Approval error:', error);
      setSigningError((error as Error).message);
    } finally {
      setIsPermitApproving(false);
    }
  };

  const handleCompactApprove = async () => {
    if (!walletClient || !address) {
      setSigningError('Wallet not connected');
      return;
    }

    if (!compactDetails) {
      setSigningError('TheCompact details unavailable');
      return;
    }

    setIsCompactApproving(true);
    setSigningError('');
    setCompactSetupError('');

    try {
      const txHash = await requestCompactApproval({
        tokenAddress: compactDetails.tokenAddress,
        contractAddress: compactDetails.contractAddress,
        amount: compactDetails.requiredAmount,
        walletClient,
      });

      setCompactApprovalTxHash(txHash);

      await waitForCompactTransaction({
        chainId: compactDetails.chainId,
        rpcUrl: compactDetails.rpcUrl,
        hash: txHash,
      });

      const refreshedAllowance = await checkCompactAllowance({
        owner: address as Address,
        tokenAddress: compactDetails.tokenAddress,
        contractAddress: compactDetails.contractAddress,
        requiredAmount: compactDetails.requiredAmount,
        chainId: compactDetails.chainId,
        rpcUrl: compactDetails.rpcUrl,
      });

      setCompactAllowance(refreshedAllowance);
      alert(`Approval transaction submitted! Hash: ${txHash}\n\nProceed to deposit once it confirms.`);
    } catch (error) {
      console.error('TheCompact approval error:', error);
      setCompactSetupError((error as Error).message);
    } finally {
      setIsCompactApproving(false);
    }
  };

  const handleCompactDeposit = async () => {
    if (!walletClient || !address) {
      setSigningError('Wallet not connected');
      return;
    }

    if (!compactDetails) {
      setSigningError('TheCompact details unavailable');
      return;
    }

    setIsDepositing(true);
    setSigningError('');
    setCompactSetupError('');

    try {
      const txHash = await requestCompactDeposit({
        contractAddress: compactDetails.contractAddress,
        tokenAddress: compactDetails.tokenAddress,
        lockTag: compactDetails.lockTag,
        amount: compactDetails.requiredAmount,
        recipient: compactDetails.sponsor,
        walletClient,
      });

      setCompactDepositTxHash(txHash);

      await waitForCompactTransaction({
        chainId: compactDetails.chainId,
        rpcUrl: compactDetails.rpcUrl,
        hash: txHash,
      });

      const refreshedDeposit = await checkCompactDeposit({
        sponsor: compactDetails.sponsor,
        contractAddress: compactDetails.contractAddress,
        lockTag: compactDetails.lockTag,
        tokenAddress: compactDetails.tokenAddress,
        requiredAmount: compactDetails.requiredAmount,
        chainId: compactDetails.chainId,
        rpcUrl: compactDetails.rpcUrl,
      });

      setCompactDepositStatus(refreshedDeposit);
      alert(`Deposit transaction submitted! Hash: ${txHash}\n\nOnce it confirms, you can sign the quote.`);
    } catch (error) {
      console.error('TheCompact deposit error:', error);
      setCompactSetupError((error as Error).message);
    } finally {
      setIsDepositing(false);
    }
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

  const isPermitOrder = selectedQuote.order.type === 'oif-escrow-v0';
  const isCompactOrder = selectedQuote.order.type === 'oif-resource-lock-v0';
  const compactInfoAvailable = isCompactOrder && !!compactDetails;
  const compactDepositSatisfied = !compactInfoAvailable || (compactDepositStatus?.hasDeposit ?? false);
  const usingWalletForCompact = compactInfoAvailable && !privateKey.trim();
  const compactActionsRequired = usingWalletForCompact && !compactDepositSatisfied;
  const compactApprovalSatisfied = !compactActionsRequired || (compactAllowance?.hasApproval ?? false);
  const compactNeedsApproval = compactActionsRequired && !compactApprovalSatisfied;
  const isCheckingCompactSetup = isCheckingCompactAllowance || isCheckingCompactDeposit;
  const walletMatchesCompactSponsor = !compactDetails || !address
    ? true
    : address.toLowerCase() === compactDetails.sponsor.toLowerCase();

  const signButtonDisabled =
    isSigning ||
    walletIsSigning ||
    isLoading ||
    isQuoteExpired ||
    (!privateKey.trim() && !isConnected) ||
    (isPermitOrder && needsPermitApproval && !privateKey.trim()) ||
    !compactDepositSatisfied ||
    (compactActionsRequired && !compactApprovalSatisfied);

  const signButtonLabel = (() => {
    if (isSigning || walletIsSigning) return 'Signing...';
    if (isQuoteExpired) return 'Quote Expired';
    if (!privateKey.trim() && isPermitOrder && needsPermitApproval) return '🔒 Approve Permit2 First';
    if (!compactDepositSatisfied) return '🔒 Deposit into TheCompact';
    if (compactActionsRequired && !compactApprovalSatisfied) return '🔒 Approve TheCompact First';
    if (privateKey.trim()) return 'Sign Quote with Private Key';
    if (isConnected) return 'Sign Quote with Wallet';
    return 'Connect Wallet or Enter Private Key';
  })();

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

          {/* Permit2 Approval Warning */}
          {needsPermitApproval && !privateKey.trim() && (
            <div className="mb-4 p-4 bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded">
              <div className="flex items-start">
                <svg className="w-5 h-5 text-yellow-600 dark:text-yellow-400 mt-0.5 mr-2" fill="currentColor" viewBox="0 0 20 20">
                  <path fillRule="evenodd" d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z" clipRule="evenodd" />
                </svg>
                <div className="flex-1">
                  <h4 className="font-semibold text-yellow-800 dark:text-yellow-200 mb-1">
                    Permit2 Approval Required
                  </h4>
                  <p className="text-sm text-yellow-700 dark:text-yellow-300 mb-3">
                    This order requires approval for the Permit2 contract to spend your tokens. 
                    You'll need to complete the approval transaction before signing the quote.
                  </p>
                  <button
                    onClick={handleApprove}
                    disabled={isPermitApproving || !walletClient}
                    className={`btn-primary ${isPermitApproving ? 'opacity-50 cursor-not-allowed' : ''}`}
                  >
                    {isPermitApproving ? 'Approving...' : '✓ Approve Permit2'}
                  </button>
                  {permitApprovalTxHash && (
                    <p className="text-xs text-green-600 dark:text-green-400 mt-2">
                      ✓ Approval submitted: {permitApprovalTxHash.slice(0, 10)}...{permitApprovalTxHash.slice(-8)}
                    </p>
                  )}
                </div>
              </div>
            </div>
          )}

          {isCheckingPermitApproval && !privateKey.trim() && (
            <div className="mb-4 p-3 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded">
              <p className="text-sm text-blue-700 dark:text-blue-300">
                ⏳ Checking Permit2 approval...
              </p>
            </div>
          )}

          {/* TheCompact Setup */}
          {compactDetails && (
            <div className="mb-4 p-4 bg-indigo-50 dark:bg-indigo-900/20 border border-indigo-200 dark:border-indigo-700 rounded">
              <h4 className="font-semibold text-indigo-800 dark:text-indigo-200 mb-2">
                Resource Lock Setup (TheCompact)
              </h4>
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-2 text-xs text-slate-600 dark:text-slate-300 mb-3">
                <p>Contract: <span className="font-mono break-all text-slate-800 dark:text-slate-100">{compactDetails.contractAddress}</span></p>
                <p>Token: <span className="font-mono break-all text-slate-800 dark:text-slate-100">{compactDetails.tokenAddress}</span></p>
                <p>Required Amount: <span className="font-mono text-slate-800 dark:text-slate-100">{compactDetails.requiredAmountRaw}</span></p>
                <p>Lock Tag: <span className="font-mono text-slate-800 dark:text-slate-100">{compactDetails.lockTag}</span></p>
                <p className="sm:col-span-2">Sponsor (recipient): <span className="font-mono break-all text-slate-800 dark:text-slate-100">{compactDetails.sponsor}</span></p>
              </div>

              {compactDepositSatisfied && (
                <div className="mb-3 bg-green-100 dark:bg-green-900/20 border border-green-300 dark:border-green-700 rounded p-2 text-xs text-green-800 dark:text-green-300">
                  ✅ Required deposit detected on TheCompact. You can proceed directly to signing once the quote is signed.
                </div>
              )}

              {!walletMatchesCompactSponsor && !privateKey.trim() && (
                <div className="mb-2 bg-yellow-100 dark:bg-yellow-900/20 border border-yellow-300 dark:border-yellow-700 rounded p-2 text-xs text-yellow-800 dark:text-yellow-300">
                  ⚠️ Connected wallet address does not match the sponsor in this quote. Deposits will credit the sponsor address shown above.
                </div>
              )}

              {isCheckingCompactSetup && !privateKey.trim() && (
                <div className="mb-2 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-700 rounded p-2 text-xs text-blue-700 dark:text-blue-300">
                  ⏳ Checking TheCompact allowance and deposit status...
                </div>
              )}

              {compactSetupError && !privateKey.trim() && (
                <div className="mb-2 bg-red-100 dark:bg-red-900/20 border border-red-300 dark:border-red-700 rounded p-2 text-xs text-red-700 dark:text-red-300">
                  ✗ {compactSetupError}
                </div>
              )}

              {!privateKey.trim() ? (
                <div className="space-y-4">
                  <div className="bg-white/70 dark:bg-slate-900/30 border border-indigo-200 dark:border-indigo-700 rounded p-3">
                    <div className="flex items-center justify-between mb-2">
                      <span className="text-sm font-semibold text-indigo-900 dark:text-indigo-200">Step 1: Approve TheCompact</span>
                      {compactApprovalSatisfied ? (
                        <span className="text-xs text-green-600 dark:text-green-400 font-semibold">✓ {compactActionsRequired ? 'Ready' : 'Not Required'}</span>
                      ) : (
                        <span className="text-xs text-yellow-600 dark:text-yellow-300 font-semibold">Pending</span>
                      )}
                    </div>
                    <p className="text-xs text-slate-600 dark:text-slate-300 mb-1">
                      Allow TheCompact to transfer your tokens for this lock.
                    </p>
                    <p className="text-xs text-slate-500 dark:text-slate-400 mb-3">
                      Current allowance: <span className="font-mono">{compactAllowance ? compactAllowance.currentAllowance.toString() : '—'}</span>
                    </p>
                    {!compactActionsRequired && (
                      <p className="text-xs text-slate-500 dark:text-slate-400 mb-3">
                        Approval is optional because the required deposit is already present.
                      </p>
                    )}
                    <button
                      onClick={handleCompactApprove}
                      disabled={compactApprovalSatisfied || isCompactApproving || !walletClient || !compactActionsRequired}
                      className={`btn-primary ${compactApprovalSatisfied || !compactActionsRequired ? 'opacity-60 cursor-not-allowed' : isCompactApproving ? 'opacity-50 cursor-not-allowed' : ''}`}
                    >
                      {compactApprovalSatisfied
                        ? compactActionsRequired
                          ? 'Approval Complete'
                          : 'Approval Not Required'
                        : isCompactApproving
                          ? 'Approving...'
                          : '✓ Approve TheCompact'}
                    </button>
                    {compactApprovalTxHash && (
                      <p className="text-xs text-green-600 dark:text-green-400 mt-2">
                        ✓ Approval submitted: {compactApprovalTxHash.slice(0, 10)}...{compactApprovalTxHash.slice(-8)}
                      </p>
                    )}
                  </div>

                  <div className="bg-white/70 dark:bg-slate-900/30 border border-indigo-200 dark:border-indigo-700 rounded p-3">
                    <div className="flex items-center justify-between mb-2">
                      <span className="text-sm font-semibold text-indigo-900 dark:text-indigo-200">Step 2: Deposit into TheCompact</span>
                      {compactDepositSatisfied ? (
                        <span className="text-xs text-green-600 dark:text-green-400 font-semibold">✓ Ready</span>
                      ) : (
                        <span className="text-xs text-yellow-600 dark:text-yellow-300 font-semibold">Pending</span>
                      )}
                    </div>
                    <p className="text-xs text-slate-600 dark:text-slate-300 mb-1">
                      Deposit the required amount into the resource lock. Tokens will credit the sponsor address above.
                    </p>
                    <p className="text-xs text-slate-500 dark:text-slate-400 mb-3">
                      Deposited amount: <span className="font-mono">{compactDepositStatus ? compactDepositStatus.depositedAmount.toString() : '—'}</span>
                    </p>
                    {compactDepositSatisfied && (
                      <p className="text-xs text-slate-500 dark:text-slate-400 mb-3">
                        Requirement met: TheCompact already holds the necessary funds for this lock.
                      </p>
                    )}
                    <button
                      onClick={handleCompactDeposit}
                      disabled={compactDepositSatisfied || isDepositing || !walletClient || compactNeedsApproval}
                      className={`btn-secondary ${compactDepositSatisfied ? 'opacity-60 cursor-not-allowed' : isDepositing ? 'opacity-50 cursor-not-allowed' : ''}`}
                    >
                      {compactDepositSatisfied ? 'Deposit Complete' : isDepositing ? 'Depositing...' : '🚀 Deposit Tokens'}
                    </button>
                    {compactDepositTxHash && (
                      <p className="text-xs text-green-600 dark:text-green-400 mt-2">
                        ✓ Deposit submitted: {compactDepositTxHash.slice(0, 10)}...{compactDepositTxHash.slice(-8)}
                      </p>
                    )}
                  </div>
                </div>
              ) : (
                <div className="bg-slate-100 dark:bg-slate-900/30 border border-slate-300 dark:border-slate-700 rounded p-3 text-xs text-slate-700 dark:text-slate-200 space-y-2">
                  <p>
                    Using private key override? Execute the approval and deposit manually before submitting the order:
                  </p>
                  <pre className="bg-slate-900 text-slate-100 p-3 rounded font-mono text-[11px] overflow-x-auto">
{`cast send ${compactDetails.tokenAddress} "approve(address,uint256)" ${compactDetails.contractAddress} ${compactDetails.requiredAmountRaw} --rpc-url ${compactDetails.rpcUrl ?? '<RPC_URL>'} --private-key <YOUR_PRIVATE_KEY>
cast send ${compactDetails.contractAddress} "depositERC20(address,bytes12,uint256,address)" ${compactDetails.tokenAddress} ${compactDetails.lockTag} ${compactDetails.requiredAmountRaw} ${compactDetails.sponsor} --rpc-url ${compactDetails.rpcUrl ?? '<RPC_URL>'} --private-key <YOUR_PRIVATE_KEY>`}
                  </pre>
                  <p className="text-[11px] text-slate-600 dark:text-slate-300">
                    Replace <code className="font-mono">&lt;YOUR_PRIVATE_KEY&gt;</code> and confirm the RPC URL matches your network.
                  </p>
                </div>
              )}
            </div>
          )}

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
            disabled={signButtonDisabled}
            className="btn-secondary w-full mb-3"
          >
            {signButtonLabel}
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

