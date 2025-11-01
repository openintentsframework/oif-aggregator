import type { UsePermit2ApprovalReturn } from '../hooks/usePermit2Approval';

interface Permit2ApprovalSectionProps extends UsePermit2ApprovalReturn {
  privateKey: string;
  walletClient?: any;
}

export default function Permit2ApprovalSection({
  needsPermitApproval,
  isCheckingPermitApproval,
  isPermitApproving,
  permitApprovalTxHash,
  handleApprove,
  approvalError,
  privateKey,
  walletClient,
}: Permit2ApprovalSectionProps) {
  // Only show for Permit2 orders when using wallet (not private key override)
  if (!needsPermitApproval && !isCheckingPermitApproval && !permitApprovalTxHash) {
    return null;
  }

  return (
    <>
      {/* Permit2 Approval Success */}
      {!needsPermitApproval && permitApprovalTxHash && !privateKey.trim() && (
        <div className="mb-4 p-4 bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded">
          <div className="flex items-start">
            <svg className="w-5 h-5 text-green-600 dark:text-green-400 mt-0.5 mr-2" fill="currentColor" viewBox="0 0 20 20">
              <path fillRule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clipRule="evenodd" />
            </svg>
            <div className="flex-1">
              <h4 className="font-semibold text-green-800 dark:text-green-200 mb-1">
                ✓ Permit2 Approval Complete
              </h4>
              <p className="text-sm text-green-700 dark:text-green-300">
                Approval transaction confirmed: {permitApprovalTxHash.slice(0, 10)}...{permitApprovalTxHash.slice(-8)}
              </p>
            </div>
          </div>
        </div>
      )}

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

      {approvalError && (
        <div className="mb-4 p-3 bg-red-100 dark:bg-red-900/20 border border-red-300 dark:border-red-700 rounded">
          <p className="text-sm text-red-700 dark:text-red-400">
            ✗ Approval Error: {approvalError}
          </p>
        </div>
      )}
    </>
  );
}

