import ReactDOM from "react-dom/client";
import { createMemoryWalletProvider, IPsyWidgetWallet, useWalletState } from "@psy/psy-wallet-widget";
import React, { useEffect, useState } from 'react';
import { ContractCallArgs, initWasmSync, PsyJSON, PsyWasmWebProverProvider, WasmRpcServer } from "@psy/psy-sdk";
import { useWalletConfig } from "../../config";
import { StoredWalletData, WALLET_STORAGE_KEY } from "../../hooks/usePersistentWallet";

interface msgParams {
  id: string,
  action: string;
  walletAddress?: string;
  callArgs: ContractCallArgs[];
}

const parsemsgParams = (): msgParams | null => {
  try {
    // #params=eyJpZCI6IjEyMzQ1In0=
    const hash = window.location.hash;
    if (!hash) {
      console.warn('URL does not contain hash parameters');
      return null;
    }

    // #params=xxx → extract xxx
    const paramsMatch = hash.match(/^#params=(.+)$/);
    if (!paramsMatch || !paramsMatch[1]) {
      console.error('The parameter format is incorrect and needs to comply with #params=Base64 string');
      return null;
    }
    const base64Str = paramsMatch[1];

    // Base64 decode
    const decodedJson = decodeURIComponent(escape(atob(base64Str)));

    const params = PsyJSON.parse(decodedJson) as msgParams;
    return params;
  } catch (error) {
    console.error('Parameter parsing failed:', error);
    return null;
  }
};

const ApprovePopup = () => {

  const [params, setParams] = useState<msgParams | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  // Control the Processing state of the Sign button
  const [isSignProcessing, setIsSignProcessing] = useState(false);

  useEffect(() => {
    const styleSheet = document.createElement('style');
    styleSheet.textContent = `
      ${styles}
      @keyframes spin {
        to { transform: rotate(360deg); }
      }
      .spin-icon {
        display: inline-block;
        width: 16px;
        height: 16px;
        border: 2px solid rgba(255,255,255,0.5);
        border-top-color: white;
        border-radius: 50%;
        margin-right: 8px;
        animation: spin 1s linear infinite;
      }
      .btn-allow.processing {
        background: var(--primary-dark);
        cursor: wait;
        pointer-events: none; 
      }
    `;
    document.head.appendChild(styleSheet);
    return () => {
      document.head.removeChild(styleSheet);
    };
  }, []);

  useEffect(() => {
    const parsedParams = parsemsgParams();
    setParams(parsedParams);
    setIsLoading(false);
  }, []);

  const { config } = useWalletConfig();

  useEffect(() => {
    // Check if we're in extension context
    if (window.location.protocol === 'chrome-extension:') {
      console.log('Running as Chrome extension');
    }

    const initWasmRpcServer = async () => {
      try {
        const rpcConfigJson = {
          global_user_tree_height: config.network.global_user_tree_height,
          realm_user_tree_height: config.network.realm_user_tree_height,
          users_per_realm: config.network.users_per_realm,
          realm_configs: config.network.realm_configs,
          coordinator_configs: config.network.coordinator_configs,
          prover_url: config.network.prover_url as string,
          prove_proxy_url: config.network.prove_proxy_url as string,
        };
        const json = PsyJSON.stringify(rpcConfigJson);
        const now = new Date().getTime();
        initWasmSync();
        PsyWasmWebProverProvider.wasmServer = await new WasmRpcServer(json);
        console.log(`WASM initialized in ${(new Date().getTime() - now) / 1000} seconds`);
      } catch (error) {
        console.error('Failed to get prover URL:', error);
      }
    };

    initWasmRpcServer();

    // Quick loading
    setTimeout(() => {
      setIsLoading(false);
    }, 10);
  }, []);

  const [
    wallets,
    currentWallet,
    setActiveWalletAsync,
    addWalletFromPrivateKey
  ] = useWalletState((state) => [
    state.wallets,
    state.currentWallet,
    state.setActiveWalletAsync,
    state.addWalletFromPrivateKey
  ]);

  // Load wallets from localStorage on mount
  useEffect(() => {
    const loadStoredWallets = async () => {
      try {
        console.log('Checking for stored wallets...');
        const stored = localStorage.getItem(WALLET_STORAGE_KEY);
        console.log('Stored data:', stored);

        if (stored) {
          const data: StoredWalletData = PsyJSON.parse(stored);
          console.log('Parsed stored data:', data);

          // Check if data is not too old (24 hours)
          const isDataFresh = Date.now() - data.lastUpdated < 24 * 60 * 60 * 1000;
          console.log('Data is fresh:', isDataFresh);

          if (isDataFresh && data.wallets.length > 0) {
            console.log('Loading stored wallets:', data.wallets.length);

            // Always try to restore if we have stored wallets but no current wallets
            if (wallets.length === 0) {
              console.log('Restoring wallets from storage...');

              // Restore wallets sequentially to avoid race conditions
              for (const walletData of data.wallets) {
                if (walletData.privateKey) {
                  try {
                    console.log('Restoring wallet:', walletData.userId, 'with private key length:', walletData.privateKey.length);
                    await addWalletFromPrivateKey(walletData.privateKey, true, false);
                  } catch (error) {
                    console.warn('Failed to restore wallet:', walletData.userId, error);
                  }
                }
              }

              console.log('Wallet restoration completed');
            } else {
              console.log('Wallets already exist, skipping restoration');
            }
          } else {
            console.log('No valid stored wallets found');
          }
        } else {
          console.log('No stored wallet data found');
        }
      } catch (error) {
        console.warn('Failed to load stored wallets:', error);
      }
    };

    // Add a small delay to allow wallet state to initialize
    const timer = setTimeout(() => {
      loadStoredWallets();
    }, 100);

    return () => clearTimeout(timer);
  }, [addWalletFromPrivateKey]);


  if (isLoading) return <div style={{ padding: 20 }}>Loading...</div>;
  if (!params) return <div style={{ padding: 20, color: 'red' }}>Parameter parsing failed, please try again</div>;

  const handleAllow = async (params: msgParams, wallets: IPsyWidgetWallet[]) => {
    if (params.action === "psy_sign") {
      setIsSignProcessing(true);
    }

    try {
      if (wallets.length === 0) {
        chrome.runtime.sendMessage({ action: "approval-result", isPsy: true, id: params.id, result: null, error: "wallet is empty" });
        console.log("wallet is empty");
        window.close();
        return;
      }
      if (params.action === "psy_requestAccounts") {
        const accounts = wallets.map(wallet => wallet.publicKeyHex);
        chrome.runtime.sendMessage({ action: "approval-result", isPsy: true, id: params.id, error: null, result: accounts });
      } else if (params.action === "psy_sign") {
        if (!params.callArgs) {
          chrome.runtime.sendMessage({ action: "approval-result", isPsy: true, id: params.id, result: null, error: "callArgs is empty" });
          console.log("callArgs is empty");
          window.close();
          return;
        }
        if (!params.walletAddress) {
          chrome.runtime.sendMessage({ action: "approval-result", isPsy: true, id: params.id, result: null, error: "walletAddress is empty" });
          console.log("walletAddress is empty");
          window.close();
          return;
        }

        let signWallet = wallets.find(wallet => wallet.publicKeyHex === params.walletAddress);
        if (!signWallet) {
          chrome.runtime.sendMessage({ action: "approval-result", isPsy: true, id: params.id, result: null, error: "walletAddress is not exist" });
          console.log("walletAddress is not exist");
          window.close();
          return;
        }

        try {
          await signWallet.wallet.execContractCall(params.walletAddress, params.callArgs);
          chrome.runtime.sendMessage({ action: "approval-result", isPsy: true, id: params.id, error: null, result: true });
        } catch (error) {
          console.error(error);
          chrome.runtime.sendMessage({ action: "approval-result", isPsy: true, id: params.id, result: null, error: error });
        }
      }

      window.close();
    } catch (error) {
      setIsSignProcessing(false);
      console.error('Handle allow failed:', error);
      chrome.runtime.sendMessage({ action: "approval-result", isPsy: true, id: params.id, result: null, error: error });
    }
    window.close();
  };

  const handleDeny = (params: msgParams) => {
    chrome.runtime.sendMessage({ action: "approval-result", isPsy: true, id: params.id, result: null, error: "User rejected" });
    window.close();
  };

  if (params.action === "psy_requestAccounts") {
    return (
      <div className="wallet-container">
        <div className="wallet-modal">
          <div className="wallet-header">
            <h3>
              <span className="wallet-icon">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                  <path d="M12 2L2 7l10 5 10-5-10-5z"></path>
                  <path d="M2 17l10 5 10-5"></path>
                  <path d="M2 12l10 5 10-5"></path>
                </svg>
              </span>
              Connect Wallet
            </h3>
          </div>
          <div className="wallet-content">
            <p className="wallet-message">
              Allow this application to connect to your wallet? This will let the app view your wallet address.
            </p>
            <div className="wallet-actions">
              <button
                className="wallet-btn btn-allow"
                onClick={() => handleAllow(params, wallets)}
              >
                Connect
              </button>
              <button
                className="wallet-btn btn-deny"
                onClick={() => handleDeny(params)}
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      </div>
    );
  } else if (params.action === "psy_sign") {
    return (
      <div className="wallet-container">
        <div className="wallet-modal">
          <div className="wallet-header">
            <h3>
              <span className="wallet-icon">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                  <path d="M16 8A6 6 0 0 1 4 8c0 7-3 9-3 9h18s-3-2-3-9"></path>
                  <path d="M10.7 14L16 18 22 12"></path>
                </svg>
              </span>
              Sign Transaction
            </h3>
          </div>
          <div className="wallet-content">
            <p className="wallet-message">
              Please confirm to psy sign this transaction with your wallet.
            </p>
            <div className="transaction-details">
              <div className="detail-item">
                <span className="detail-label">Contract ID</span>
                <span className="detail-value">{params.callArgs[0].contract_id.toString() || "0"}</span>
              </div>
              <div className="detail-item">
                <span className="detail-label">Method</span>
                <span className="detail-value">{params.callArgs[0].method_name || "mint"}</span>
              </div>
              <div className="detail-item">
                <span className="detail-label">Inputs</span>
                <span className="detail-value">{params.callArgs[0].inputs.join(', ') || '0'}</span>
              </div>
            </div>
            <div className="wallet-actions">
              <button
                className={`wallet-btn btn-allow ${isSignProcessing ? 'processing' : ''}`}
                onClick={() => handleAllow(params, wallets)}
                disabled={isSignProcessing}
              >
                {isSignProcessing ? (
                  <>
                    <span className="spin-icon"></span>
                    Processing...
                  </>
                ) : (
                  "Sign"
                )}
              </button>
              <button
                className="wallet-btn btn-deny"
                onClick={() => handleDeny(params)}
                disabled={isSignProcessing}
              >
                Reject
              </button>
            </div>
          </div>
        </div>
      </div>
    );
  }
};

ReactDOM.createRoot(document.getElementById("root")!).render(<ApprovePopup />);

const styles = `
  .wallet-container {
    width: 100%;
    height: 100%;
    --primary: #6366f1;
    --primary-light: #818cf8;
    --primary-dark: #4f46e5;
    --neutral: #f3f4f6;
    --neutral-dark: #e5e7eb;
    --text-primary: #111827;
    --text-secondary: #6b7280;
    --danger: #ef4444;
  }

  .wallet-modal {
    background: white;
    box-shadow: 0 10px 50px rgba(0, 0, 0, 0.1);
    max-width: 420px;
    width: 100vw;
    overflow: hidden;
    font-family: 'Inter', system-ui, sans-serif;
    transition: transform 0.3s ease;
  }

  .wallet-modal:hover {
    box-shadow: 0 15px 60px rgba(0, 0, 0, 0.15);
  }

  .wallet-header {
    padding: 24px;
    background: linear-gradient(135deg, var(--primary-light), var(--primary));
    color: white;
  }

  .wallet-header h3 {
    margin: 0;
    font-size: 1.25rem;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .wallet-icon {
    width: 28px;
    height: 28px;
    background: rgba(255, 255, 255, 0.2);
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .wallet-content {
    padding: 24px;
  }

  .wallet-message {
    color: var(--text-secondary);
    line-height: 1.5;
    margin: 0 0 24px 0;
    font-size: 0.95rem;
  }

  .transaction-details {
    background: var(--neutral);
    border-radius: 12px;
    padding: 16px;
    margin-bottom: 24px;
  }

  .detail-item {
    display: flex;
    justify-content: space-between;
    padding: 10px 0;
    border-bottom: 1px solid var(--neutral-dark);
    font-size: 0.9rem;
  }

  .detail-item:last-child {
    border-bottom: none;
  }

  .detail-label {
    color: var(--text-secondary);
    font-weight: 500;
  }

  .detail-value {
    color: var(--text-primary);
    word-break: break-all;
    text-align: right;
    max-width: 60%;
  }

  .wallet-actions {
    display: flex;
    gap: 12px;
    margin-top: 24px;
  }

  .wallet-btn {
    flex: 1;
    padding: 12px 16px;
    border-radius: 10px;
    font-weight: 600;
    font-size: 0.95rem;
    cursor: pointer;
    transition: all 0.2s ease;
    border: none;
    outline: none;
  }

  .btn-allow {
    background: var(--primary);
    color: white;
    position: relative;
    overflow: hidden;
  }

  .btn-allow::after {
    content: '';
    position: absolute;
    top: 0;
    left: -100%;
    width: 100%;
    height: 100%;
    background: linear-gradient(90deg, transparent, rgba(255,255,255,0.2), transparent);
    transition: 0.5s;
  }

  .btn-allow:hover {
    background: var(--primary-dark);
    transform: translateY(-2px);
  }

  .btn-allow:hover::after {
    left: 100%;
  }

  .btn-deny {
    background: var(--neutral);
    color: var(--text-secondary);
  }

  .btn-deny:hover {
    background: var(--neutral-dark);
    transform: translateY(-2px);
    color: var(--danger);
  }

  .wallet-btn:active {
    transform: translateY(0);
  }
`;