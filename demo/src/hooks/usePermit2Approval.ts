import { useEffect, useState } from 'react';
import type { Address, WalletClient } from 'viem';
import type { QuoteResponse } from '../types/api';
import { fromInteropAddress } from '../utils/interopAddress';
import { getRpcUrlForChain } from '../utils/chainUtils';
import {
  checkPermit2Approval,
  requestPermit2Approval,
  waitForPermit2Transaction,
} from '../utils/permit2Approval';

export interface UsePermit2ApprovalParams {
  selectedQuote: QuoteResponse;
  isConnected: boolean;
  address?: string;
  privateKey: string;
  walletClient?: WalletClient;
}

export interface UsePermit2ApprovalReturn {
  needsPermitApproval: boolean;
  isCheckingPermitApproval: boolean;
  isPermitApproving: boolean;
  permitApprovalTxHash: string;
  handleApprove: () => Promise<void>;
  approvalError: string;
}

export function usePermit2Approval({
  selectedQuote,
  isConnected,
  address,
  privateKey,
  walletClient,
}: UsePermit2ApprovalParams): UsePermit2ApprovalReturn {
  const [needsPermitApproval, setNeedsPermitApproval] = useState(false);
  const [isCheckingPermitApproval, setIsCheckingPermitApproval] = useState(false);
  const [isPermitApproving, setIsPermitApproving] = useState(false);
  const [permitApprovalTxHash, setPermitApprovalTxHash] = useState<string>('');
  const [approvalError, setApprovalError] = useState('');

  // Check Permit2 approval for wallet signing
  useEffect(() => {
    const checkApproval = async () => {
      // Only check for Permit2 orders when using wallet (not private key override)
      if (selectedQuote.order.type !== 'oif-escrow-v0' || !isConnected || !address || privateKey.trim()) {
        setNeedsPermitApproval(false);
        // Don't clear tx hash here - let it persist to show successful approval
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

  // Handle Permit2 approval
  const handleApprove = async () => {
    if (!walletClient || !address) {
      setApprovalError('Wallet not connected');
      return;
    }

    setIsPermitApproving(true);
    setApprovalError('');

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

      // Retry approval check with backoff to handle blockchain state propagation delays
      let approvalStatus = { hasApproval: false };
      const maxRetries = 3;
      
      for (let i = 0; i < maxRetries; i++) {
        if (i > 0) {
          await new Promise(resolve => setTimeout(resolve, 1500 * i)); // Increasing backoff
        }
        
        approvalStatus = await checkPermit2Approval(
          address as Address,
          inputInterop.address as Address,
          inputInterop.chainId,
          rpcUrl
        );
        
        if (approvalStatus.hasApproval) {
          break;
        }
      }

      setNeedsPermitApproval(!approvalStatus.hasApproval);
      
      if (approvalStatus.hasApproval) {
        alert(`Approval transaction confirmed! Hash: ${txHash}\n\nYou can now sign the quote.`);
      } else {
        console.warn('Approval transaction confirmed but approval check still returns false after retries.');
        setApprovalError('Approval transaction confirmed, but status check is delayed. Please refresh the page.');
      }
    } catch (error) {
      console.error('Approval error:', error);
      setApprovalError((error as Error).message);
    } finally {
      setIsPermitApproving(false);
    }
  };

  return {
    needsPermitApproval,
    isCheckingPermitApproval,
    isPermitApproving,
    permitApprovalTxHash,
    handleApprove,
    approvalError,
  };
}

