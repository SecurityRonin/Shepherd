import React from "react";

interface CreditBalanceProps {
  balance: number;
  topupUrl: string;
}

export const CreditBalance: React.FC<CreditBalanceProps> = React.memo(({ balance, topupUrl }) => {
  return (
    <div className="text-center py-4">
      <div className="text-4xl font-bold text-yellow-400 font-mono" data-testid="credit-balance">
        {balance}
      </div>
      <div className="text-sm text-gray-400 mt-1">credits available</div>
      <a
        href={topupUrl}
        target="_blank"
        rel="noreferrer"
        className="mt-3 inline-block text-xs text-blue-400 hover:underline"
        data-testid="topup-link"
      >
        Top up credits →
      </a>
    </div>
  );
});
