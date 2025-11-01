import type { UseCompactSetupReturn } from '../hooks/useCompactSetup';

interface CompactSetupSectionProps extends UseCompactSetupReturn {
  privateKey: string;
  walletClient?: any;
}

export default function CompactSetupSection({
  compactDetails,
  compactAllowance,
  compactDepositStatus,
  isCheckingCompactSetup,
  isCompactApproving,
  isDepositing,
  compactApprovalTxHash,
  compactDepositTxHash,
  compactSetupError,
  handleCompactApprove,
  handleCompactDeposit,
  compactDepositSatisfied,
  walletMatchesCompactSponsor,
  compactActionsRequired,
  compactApprovalSatisfied,
  compactNeedsApproval,
  privateKey,
  walletClient,
}: CompactSetupSectionProps) {
  if (!compactDetails) {
    return null;
  }

  return (
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
            {compactApprovalSatisfied && compactApprovalTxHash && compactActionsRequired ? (
              <div className="bg-green-100 dark:bg-green-900/20 border border-green-300 dark:border-green-700 rounded p-2 text-xs text-green-800 dark:text-green-300">
                <p className="font-semibold mb-1">✅ Approval Complete</p>
                <p className="font-mono text-[10px]">TX: {compactApprovalTxHash.slice(0, 10)}...{compactApprovalTxHash.slice(-8)}</p>
              </div>
            ) : (
              <>
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
                {compactApprovalTxHash && !compactApprovalSatisfied && (
                  <p className="text-xs text-blue-600 dark:text-blue-400 mt-2">
                    ⏳ Approval submitted: {compactApprovalTxHash.slice(0, 10)}...{compactApprovalTxHash.slice(-8)}
                  </p>
                )}
              </>
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
            {compactDepositSatisfied && compactDepositTxHash ? (
              <div className="bg-green-100 dark:bg-green-900/20 border border-green-300 dark:border-green-700 rounded p-2 text-xs text-green-800 dark:text-green-300">
                <p className="font-semibold mb-1">✅ Deposit Complete</p>
                <p className="font-mono text-[10px]">TX: {compactDepositTxHash.slice(0, 10)}...{compactDepositTxHash.slice(-8)}</p>
              </div>
            ) : (
              <>
                <button
                  onClick={handleCompactDeposit}
                  disabled={compactDepositSatisfied || isDepositing || !walletClient || compactNeedsApproval}
                  className={`btn-secondary ${compactDepositSatisfied ? 'opacity-60 cursor-not-allowed' : isDepositing ? 'opacity-50 cursor-not-allowed' : ''}`}
                >
                  {compactDepositSatisfied ? 'Deposit Complete' : isDepositing ? 'Depositing...' : '🚀 Deposit Tokens'}
                </button>
                {compactDepositTxHash && !compactDepositSatisfied && (
                  <p className="text-xs text-blue-600 dark:text-blue-400 mt-2">
                    ⏳ Deposit submitted: {compactDepositTxHash.slice(0, 10)}...{compactDepositTxHash.slice(-8)}
                  </p>
                )}
              </>
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
  );
}

