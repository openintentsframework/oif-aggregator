import { useEffect, useMemo, useState } from 'react';
import type { Address, Hex, WalletClient } from 'viem';
import type { QuoteResponse } from '../types/api';
import { fromInteropAddress } from '../utils/interopAddress';
import { getRpcUrlForChain } from '../utils/chainUtils';
import {
  checkCompactAllowance,
  requestCompactApproval,
  checkCompactDeposit,
  requestCompactDeposit,
  waitForCompactTransaction,
} from '../utils/compactSetup';

export interface CompactDetails {
  tokenAddress: Address;
  lockTag: Hex;
  requiredAmount: bigint;
  requiredAmountRaw: string;
  sponsor: Address;
  contractAddress: Address;
  chainId: number;
  rpcUrl?: string;
}

export interface UseCompactSetupParams {
  selectedQuote: QuoteResponse;
  isConnected: boolean;
  address?: string;
  privateKey: string;
  walletClient?: WalletClient;
}

export interface UseCompactSetupReturn {
  compactDetails: CompactDetails | null;
  compactAllowance: { hasApproval: boolean; currentAllowance: bigint } | null;
  compactDepositStatus: { hasDeposit: boolean; depositedAmount: bigint; lockId: bigint } | null;
  isCheckingCompactAllowance: boolean;
  isCheckingCompactDeposit: boolean;
  isCompactApproving: boolean;
  isDepositing: boolean;
  compactApprovalTxHash: string;
  compactDepositTxHash: string;
  compactSetupError: string;
  handleCompactApprove: () => Promise<void>;
  handleCompactDeposit: () => Promise<void>;
  // Derived values
  compactInfoAvailable: boolean;
  compactDepositSatisfied: boolean;
  usingWalletForCompact: boolean;
  compactActionsRequired: boolean;
  compactApprovalSatisfied: boolean;
  compactNeedsApproval: boolean;
  isCheckingCompactSetup: boolean;
  walletMatchesCompactSponsor: boolean;
  setCompactSetupError: (error: string) => void;
}

export function useCompactSetup({
  selectedQuote,
  isConnected,
  address,
  privateKey,
  walletClient,
}: UseCompactSetupParams): UseCompactSetupReturn {
  // Parse compact details from quote
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

  // Handle TheCompact approval
  const handleCompactApprove = async () => {
    if (!walletClient || !address) {
      setCompactSetupError('Wallet not connected');
      return;
    }

    if (!compactDetails) {
      setCompactSetupError('TheCompact details unavailable');
      return;
    }

    setIsCompactApproving(true);
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

      // Retry allowance check with backoff to handle blockchain state propagation delays
      let refreshedAllowance = { hasApproval: false, currentAllowance: 0n };
      const maxRetries = 3;
      
      for (let i = 0; i < maxRetries; i++) {
        if (i > 0) {
          await new Promise(resolve => setTimeout(resolve, 1500 * i)); // Increasing backoff
        }
        
        refreshedAllowance = await checkCompactAllowance({
          owner: address as Address,
          tokenAddress: compactDetails.tokenAddress,
          contractAddress: compactDetails.contractAddress,
          requiredAmount: compactDetails.requiredAmount,
          chainId: compactDetails.chainId,
          rpcUrl: compactDetails.rpcUrl,
        });
        
        if (refreshedAllowance.hasApproval) {
          break;
        }
      }

      setCompactAllowance(refreshedAllowance);
      
      if (refreshedAllowance.hasApproval) {
        alert(`Approval transaction confirmed! Hash: ${txHash}\n\nYou can now proceed to deposit.`);
      } else {
        console.warn('Approval transaction confirmed but allowance check still returns insufficient after retries.');
        setCompactSetupError('Approval transaction confirmed, but status check is delayed. Please refresh the page.');
      }
    } catch (error) {
      console.error('TheCompact approval error:', error);
      setCompactSetupError((error as Error).message);
    } finally {
      setIsCompactApproving(false);
    }
  };

  // Handle TheCompact deposit
  const handleCompactDeposit = async () => {
    if (!walletClient || !address) {
      setCompactSetupError('Wallet not connected');
      return;
    }

    if (!compactDetails) {
      setCompactSetupError('TheCompact details unavailable');
      return;
    }

    setIsDepositing(true);
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

      // Retry deposit check with backoff to handle blockchain state propagation delays
      let refreshedDeposit = { hasDeposit: false, depositedAmount: 0n, lockId: 0n };
      const maxRetries = 3;
      
      for (let i = 0; i < maxRetries; i++) {
        if (i > 0) {
          await new Promise(resolve => setTimeout(resolve, 1500 * i)); // Increasing backoff
        }
        
        refreshedDeposit = await checkCompactDeposit({
          sponsor: compactDetails.sponsor,
          contractAddress: compactDetails.contractAddress,
          lockTag: compactDetails.lockTag,
          tokenAddress: compactDetails.tokenAddress,
          requiredAmount: compactDetails.requiredAmount,
          chainId: compactDetails.chainId,
          rpcUrl: compactDetails.rpcUrl,
        });
        
        if (refreshedDeposit.hasDeposit) {
          break;
        }
      }

      setCompactDepositStatus(refreshedDeposit);
      
      if (refreshedDeposit.hasDeposit) {
        alert(`Deposit transaction confirmed! Hash: ${txHash}\n\nYou can now sign the quote.`);
      } else {
        console.warn('Deposit transaction confirmed but deposit check still returns insufficient after retries.');
        setCompactSetupError('Deposit transaction confirmed, but status check is delayed. Please refresh the page.');
      }
    } catch (error) {
      console.error('TheCompact deposit error:', error);
      setCompactSetupError((error as Error).message);
    } finally {
      setIsDepositing(false);
    }
  };

  // Derived values
  const compactInfoAvailable = selectedQuote.order.type === 'oif-resource-lock-v0' && !!compactDetails;
  const compactDepositSatisfied = !compactInfoAvailable || (compactDepositStatus?.hasDeposit ?? false);
  const usingWalletForCompact = compactInfoAvailable && !privateKey.trim();
  const compactActionsRequired = usingWalletForCompact && !compactDepositSatisfied;
  const compactApprovalSatisfied = !compactActionsRequired || (compactAllowance?.hasApproval ?? false);
  const compactNeedsApproval = compactActionsRequired && !compactApprovalSatisfied;
  const isCheckingCompactSetup = isCheckingCompactAllowance || isCheckingCompactDeposit;
  const walletMatchesCompactSponsor = !compactDetails || !address
    ? true
    : address.toLowerCase() === compactDetails.sponsor.toLowerCase();

  return {
    compactDetails,
    compactAllowance,
    compactDepositStatus,
    isCheckingCompactAllowance,
    isCheckingCompactDeposit,
    isCompactApproving,
    isDepositing,
    compactApprovalTxHash,
    compactDepositTxHash,
    compactSetupError,
    handleCompactApprove,
    handleCompactDeposit,
    compactInfoAvailable,
    compactDepositSatisfied,
    usingWalletForCompact,
    compactActionsRequired,
    compactApprovalSatisfied,
    compactNeedsApproval,
    isCheckingCompactSetup,
    walletMatchesCompactSponsor,
    setCompactSetupError,
  };
}

