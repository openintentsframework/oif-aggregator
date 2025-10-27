import type { Input, Output, QuoteRequest } from '../types/api';

import type { SimpleQuoteFormData } from '../types/solverData';
import { toInteropAddress } from './interopAddress';

/**
 * Convert amount string to wei based on decimals
 * Example: "1.5" with 18 decimals = "1500000000000000000"
 */
export function parseAmount(amount: string, decimals: number): string {
  try {
    const [whole, fraction = ''] = amount.split('.');
    const paddedFraction = fraction.padEnd(decimals, '0').slice(0, decimals);
    const weiString = (whole || '0') + paddedFraction;

    // Remove leading zeros
    const cleanedWei = weiString.replace(/^0+/, '') || '0';

    return cleanedWei;
  } catch (error) {
    throw new Error(`Invalid amount format: ${amount}`);
  }
}

/**
 * Format wei amount to readable string
 * Example: "1500000000000000000" with 18 decimals = "1.5"
 */
export function formatAmount(amountWei: string, decimals: number): string {
  try {
    const padded = amountWei.padStart(decimals + 1, '0');
    const whole = padded.slice(0, -decimals) || '0';
    const fraction = padded.slice(-decimals);

    // Remove trailing zeros from fraction
    const trimmedFraction = fraction.replace(/0+$/, '');

    return trimmedFraction ? `${whole}.${trimmedFraction}` : whole;
  } catch (error) {
    return amountWei;
  }
}

/**
 * Convert simplified form data to OIF QuoteRequest format
 */
export function convertFormToQuoteRequest(
  formData: SimpleQuoteFormData,
  swapType: 'exact-input' | 'exact-output',
  supportedTypes: string[] = ['oif-escrow-v0', 'oif-resource-lock-v0']
): QuoteRequest {
  if (!formData.fromAsset || !formData.toAsset) {
    throw new Error('Both from and to assets must be selected');
  }

  if (!formData.amount || parseFloat(formData.amount) <= 0) {
    throw new Error('Amount must be greater than 0');
  }

  if (!formData.userAddress) {
    throw new Error('User address is required');
  }

  // Convert amount based on decimals
  const amountWei = parseAmount(formData.amount, formData.fromAsset.decimals);

  // Create input
  const input: Input = {
    user: toInteropAddress(formData.userAddress, formData.fromChain),
    asset: toInteropAddress(formData.fromAsset.address, formData.fromChain),
    amount: swapType === 'exact-input' ? amountWei : undefined,
  };

  // Create output
  const receiverAddress = formData.receiverAddress || formData.userAddress;
  const output: Output = {
    receiver: toInteropAddress(receiverAddress, formData.toChain),
    asset: toInteropAddress(formData.toAsset.address, formData.toChain),
    amount: swapType === 'exact-output' ? amountWei : undefined,
  };

  return {
    user: input.user,
    intent: {
      intentType: 'oif-swap',
      inputs: [input],
      outputs: [output],
      swapType,
    },
    supportedTypes,
  };
}

/**
 * Validate form data
 */
export function validateFormData(formData: SimpleQuoteFormData): {
  valid: boolean;
  errors: string[];
} {
  const errors: string[] = [];

  if (!formData.amount || parseFloat(formData.amount) <= 0) {
    errors.push('Amount must be greater than 0');
  }

  if (!formData.fromAsset) {
    errors.push('Please select a from asset');
  }

  if (!formData.toAsset) {
    errors.push('Please select a to asset');
  }

  if (!formData.userAddress) {
    errors.push('Please enter a user address');
  } else if (!/^0x[a-fA-F0-9]{40}$/.test(formData.userAddress)) {
    errors.push('Invalid user address format');
  }

  if (
    formData.fromAsset &&
    formData.toAsset &&
    formData.fromChain === formData.toChain &&
    formData.fromAsset.address.toLowerCase() ===
      formData.toAsset.address.toLowerCase()
  ) {
    errors.push('From and to assets cannot be the same');
  }

  return {
    valid: errors.length === 0,
    errors,
  };
}
