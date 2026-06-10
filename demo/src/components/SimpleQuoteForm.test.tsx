import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';

import SimpleQuoteForm from './SimpleQuoteForm';
import { solverDataService } from '../services/solverDataService';

vi.mock('../contexts/WalletContext', () => ({
  useWallet: () => ({
    isConnected: false,
    address: undefined,
    chainId: undefined,
  }),
}));

describe('SimpleQuoteForm', () => {
  beforeEach(() => {
    solverDataService.clearCache();
    (solverDataService as any).cache = {
      assets: new Map([
        [
          '1-0x1111111111111111111111111111111111111111',
          {
            address: '0x1111111111111111111111111111111111111111',
            chainId: 1,
            symbol: 'ETH',
            name: 'Ether',
            decimals: 18,
            solvers: ['solver-a'],
          },
        ],
        [
          '10-0x2222222222222222222222222222222222222222',
          {
            address: '0x2222222222222222222222222222222222222222',
            chainId: 10,
            symbol: 'OP',
            name: 'Optimism',
            decimals: 18,
            solvers: ['solver-a'],
          },
        ],
        [
          '8453-0x3333333333333333333333333333333333333333',
          {
            address: '0x3333333333333333333333333333333333333333',
            chainId: 8453,
            symbol: 'USDC',
            name: 'USD Coin',
            decimals: 6,
            solvers: ['solver-a'],
          },
        ],
      ]),
      assetsByChain: new Map([
        [
          1,
          [
            {
              address: '0x1111111111111111111111111111111111111111',
              chainId: 1,
              symbol: 'ETH',
              name: 'Ether',
              decimals: 18,
              solvers: ['solver-a'],
            },
          ],
        ],
        [
          10,
          [
            {
              address: '0x2222222222222222222222222222222222222222',
              chainId: 10,
              symbol: 'OP',
              name: 'Optimism',
              decimals: 18,
              solvers: ['solver-a'],
            },
          ],
        ],
        [
          8453,
          [
            {
              address: '0x3333333333333333333333333333333333333333',
              chainId: 8453,
              symbol: 'USDC',
              name: 'USD Coin',
              decimals: 6,
              solvers: ['solver-a'],
            },
          ],
        ],
      ]),
      routes: [],
      chains: [
        { chainId: 1, name: 'Ethereum', assetCount: 1 },
        { chainId: 10, name: 'Optimism', assetCount: 1 },
        { chainId: 8453, name: 'Base', assetCount: 1 },
      ],
      lastFetched: Date.now(),
    };
  });

  it('keeps destination network editable before a source asset is selected', () => {
    render(<SimpleQuoteForm onSubmit={vi.fn()} isLoading={false} />);

    const [, toNetworkSelect] = screen.getAllByRole('combobox');
    expect(toNetworkSelect).toBeEnabled();

    fireEvent.change(toNetworkSelect, { target: { value: '8453' } });
    expect(toNetworkSelect).toHaveValue('8453');

    const toAssetButton = screen.getByRole('button', { name: /select to asset/i });
    expect(toAssetButton).toBeEnabled();
  });
});
