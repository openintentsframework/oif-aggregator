import { ConnectButton } from '@rainbow-me/rainbowkit';
import { useDisconnect, useSwitchChain } from 'wagmi';
import { useWallet } from '../contexts/WalletContext';
import { useState, useEffect, useRef } from 'react';
import { mainnet, optimism, arbitrum, base, polygon, bsc, avalanche, sepolia, optimismSepolia, arbitrumSepolia, baseSepolia, polygonMumbai } from 'wagmi/chains';

const SUPPORTED_CHAINS = [
  { id: mainnet.id, name: 'Ethereum', color: 'bg-blue-500' },
  { id: optimism.id, name: 'Optimism', color: 'bg-red-500' },
  { id: arbitrum.id, name: 'Arbitrum', color: 'bg-blue-600' },
  { id: base.id, name: 'Base', color: 'bg-blue-400' },
  { id: polygon.id, name: 'Polygon', color: 'bg-purple-500' },
  { id: bsc.id, name: 'BSC', color: 'bg-yellow-500' },
  { id: avalanche.id, name: 'Avalanche', color: 'bg-red-600' },
  { id: sepolia.id, name: 'Sepolia', color: 'bg-gray-500' },
  { id: optimismSepolia.id, name: 'OP Sepolia', color: 'bg-red-400' },
  { id: arbitrumSepolia.id, name: 'Arb Sepolia', color: 'bg-blue-500' },
  { id: baseSepolia.id, name: 'Base Sepolia', color: 'bg-blue-300' },
  { id: polygonMumbai.id, name: 'Mumbai', color: 'bg-purple-400' },
];

export default function CustomWalletButton() {
  const { isConnected, address: _, chainId } = useWallet();
  const { disconnect } = useDisconnect();
  const { switchChain } = useSwitchChain();
  const [showNetworkDropdown, setShowNetworkDropdown] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  // Close dropdown when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setShowNetworkDropdown(false);
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, []);

  if (!isConnected) {
    return <ConnectButton />;
  }

  const currentChain = chainId ? SUPPORTED_CHAINS.find(chain => chain.id === chainId) : null;

  const handleSwitchChain = async (targetChainId: number) => {
    try {
      await switchChain({ chainId: targetChainId });
      setShowNetworkDropdown(false);
    } catch (error) {
      console.error('Failed to switch chain:', error);
    }
  };

  return (
    <div className="flex items-center gap-2">
      {/* Network Selector */}
      <div className="relative" ref={dropdownRef}>
        <button
          onClick={() => setShowNetworkDropdown(!showNetworkDropdown)}
          className="flex items-center gap-2 bg-slate-200 dark:bg-slate-700 hover:bg-slate-300 dark:hover:bg-slate-600 text-slate-800 dark:text-white py-1.5 px-3 rounded-md text-sm transition-colors"
        >
          <div className={`w-2 h-2 rounded-full ${currentChain?.color || 'bg-gray-500'}`}></div>
          <span>{currentChain?.name || `Chain ${chainId}`}</span>
          <svg className="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
          </svg>
        </button>

        {/* Network Dropdown */}
        {showNetworkDropdown && (
          <div className="absolute top-full right-0 mt-1 bg-white dark:bg-slate-800 border border-slate-300 dark:border-slate-700 rounded-md shadow-lg z-[100] min-w-48 max-h-64 overflow-y-auto">
            <div className="py-1">
              {SUPPORTED_CHAINS.map((chain) => (
                <button
                  key={chain.id}
                  onClick={() => handleSwitchChain(chain.id)}
                  className={`w-full text-left px-3 py-2 text-sm hover:bg-slate-100 dark:hover:bg-slate-700 flex items-center gap-2 ${
                    chain.id === chainId ? 'bg-slate-100 dark:bg-slate-700' : ''
                  }`}
                >
                  <div className={`w-2 h-2 rounded-full ${chain.color}`}></div>
                  <span className="text-slate-800 dark:text-slate-200">{chain.name}</span>
                  {chain.id === chainId && (
                    <span className="ml-auto text-green-600 dark:text-green-400">✓</span>
                  )}
                </button>
              ))}
            </div>
          </div>
        )}
      </div>

      {/* Disconnect Button */}
      <button
        onClick={() => disconnect()}
        className="text-slate-500 dark:text-slate-400 hover:text-slate-700 dark:hover:text-slate-200 text-xs px-2 py-1 rounded hover:bg-slate-100 dark:hover:bg-slate-800 transition-colors"
        title="Disconnect"
      >
        ✕
      </button>
    </div>
  );
}
