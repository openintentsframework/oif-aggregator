import type { ChainInfo } from '../types/solverData';

interface NetworkSelectProps {
  chains: ChainInfo[];
  value: number | null;
  onChange: (chainId: number) => void;
  placeholder?: string;
  disabled?: boolean;
}

export default function NetworkSelect({
  chains,
  value,
  onChange,
  placeholder = 'Select network',
  disabled = false,
}: NetworkSelectProps) {
  return (
    <select
      value={value || ''}
      onChange={(e) => {
        const chainId = parseInt(e.target.value);
        if (!isNaN(chainId)) {
          onChange(chainId);
        }
      }}
      disabled={disabled}
      className="input-field"
    >
      <option value="">{placeholder}</option>
      {chains.map((chain) => (
        <option key={chain.chainId} value={chain.chainId}>
          {chain.name} (Chain {chain.chainId})
          {chain.assetCount > 0 && ` - ${chain.assetCount} assets`}
        </option>
      ))}
    </select>
  );
}

