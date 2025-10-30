import { createContext, useContext, ReactNode, useEffect } from 'react';
import { useAccount, useSignTypedData, useSignMessage, useDisconnect, useChainId } from 'wagmi';
import type { Address, Hex } from 'viem';

interface WalletContextType {
  // Connection state
  isConnected: boolean;
  isConnecting: boolean;
  address: Address | undefined;
  chainId: number | undefined;
  
  // Signing functions
  signTypedData: (args: {
    domain: any;
    types: any;
    primaryType: string;
    message: any;
  }) => Promise<Hex>;
  signMessage: (message: string | Hex) => Promise<Hex>;
  isSigning: boolean;
  
  // Connection management
  disconnect: () => void;
}

const WalletContext = createContext<WalletContextType | undefined>(undefined);

export function WalletProvider({ children }: { children: ReactNode }) {
  let address, isConnected, isConnecting, chainId;
  
  try {
    const accountData = useAccount();
    address = accountData.address;
    isConnected = accountData.isConnected;
    isConnecting = accountData.isConnecting;
    chainId = accountData.chainId;
  } catch (error) {
    console.warn('Error getting account data:', error);
    address = undefined;
    isConnected = false;
    isConnecting = false;
    chainId = undefined;
  }

  // Try to get chainId from useChainId hook as fallback
  let fallbackChainId;
  try {
    fallbackChainId = useChainId();
  } catch (error) {
    console.warn('Error getting chain ID:', error);
    fallbackChainId = undefined;
  }

  const { signTypedDataAsync, isPending: isSigningTypedData } = useSignTypedData();
  const { signMessageAsync, isPending: isSigningMessage } = useSignMessage();
  const { disconnect } = useDisconnect();

  // Use chainId from account or fallback to useChainId
  const safeChainId = chainId || fallbackChainId || undefined;
  
  // Combine signing states
  const isSigning = isSigningTypedData || isSigningMessage;

  // Handle wallet disconnection gracefully
  useEffect(() => {
    if (!isConnected && address) {
      // Wallet was disconnected, cleanup logic can be added here if needed
    }
  }, [isConnected, address]);

  const signTypedData = async (args: {
    domain: any;
    types: any;
    primaryType: string;
    message: any;
  }): Promise<Hex> => {
    if (!isConnected) {
      throw new Error('Wallet not connected');
    }
    
    try {
      return await signTypedDataAsync(args);
    } catch (error: any) {
      // Handle specific connector errors
      if (error.message?.includes('getChainId is not a function')) {
        throw new Error('Wallet connector error: Please try disconnecting and reconnecting your wallet');
      }
      if (error.message?.includes('User rejected')) {
        throw new Error('Transaction was rejected by user');
      }
      if (error.message?.includes('insufficient funds')) {
        throw new Error('Insufficient funds for transaction');
      }
      // Re-throw other errors
      throw error;
    }
  };

  const signMessage = async (message: string | Hex): Promise<Hex> => {
    if (!isConnected) {
      throw new Error('Wallet not connected');
    }
    
    try {
      const messageParam = typeof message === 'string' ? message : { raw: message };
      return await signMessageAsync({ message: messageParam });
    } catch (error: any) {
      // Handle specific errors
      if (error.message?.includes('User rejected')) {
        throw new Error('Signature was rejected by user');
      }
      // Re-throw other errors
      throw error;
    }
  };

  const value: WalletContextType = {
    isConnected,
    isConnecting,
    address,
    chainId: safeChainId,
    signTypedData,
    signMessage,
    isSigning,
    disconnect,
  };

  return (
    <WalletContext.Provider value={value}>
      {children}
    </WalletContext.Provider>
  );
}

export function useWallet() {
  const context = useContext(WalletContext);
  if (context === undefined) {
    throw new Error('useWallet must be used within a WalletProvider');
  }
  return context;
}
