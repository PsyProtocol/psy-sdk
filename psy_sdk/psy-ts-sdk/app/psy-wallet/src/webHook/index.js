class PsyProvider {
  constructor() {
    this._events = new Map();
    this._requestListeners = new Map();
    this._accounts = [];
    this._initMessageListener();
  }

  // Initialize message monitoring (receive background responses forwarded by content scripts)
  _initMessageListener() {
    window.addEventListener('message', (event) => {
      if (event.source !== window || !event.data?.isPsy) return;
      const { type, data: nestedData, source } = event.data;

      if (type != 'messageFromWallet' || source != 'psy-contentscript') {
        return;
      }
      if (!nestedData) {
        console.log('WebHook received message from wallet: empty data');
        return;
      }

      console.log('WebHook received message from wallet:', event.data);

      const { id, result, error } = nestedData || {};

      if (id && this._requestListeners.has(id)) {
        const { resolve, reject } = this._requestListeners.get(id);
        this._requestListeners.delete(id);

        if (error) {
          reject(new Error(error.message || error));
        } else {
          if (result?.accounts) {
            this._accounts = result.accounts;
            this._emit('accountsChanged', this._accounts);
          }
          resolve(result);
        }
        return;
      }
    });
  }

  // Send a message to the content script (relay to the background)
  _sendMessage(action, walletAddress, params, id) {
    if (!window.psy) {
      console.error('Psy wallet is not initialized');
      return false;
    }

    try {
      window.postMessage({
        isPsy: true,
        action,
        walletAddress,
        callArgs: params,
        id
      }, '*');
      return true;
    } catch (error) {
      console.error('Failed to send message:', error);
      return false;
    }
  }

  _generateId() {
    return `psy_${Date.now()}_${Math.random().toString(36).slice(2, 10)}`;
  }

  // EIP-1193 action: handling DApp requests(login, signature, etc.)
  async request({ action, walletAddress, params }) {
    return new Promise((resolve, reject) => {
      const validMethods = ['psy_requestAccounts', 'psy_accounts', 'psy_sign'];
      if (!validMethods.includes(action)) {
        reject(new Error(`Unsupported action: ${action}`));
        return;
      }

      const id = this._generateId();

      this._requestListeners.set(id, { resolve, reject });

      const timeoutId = setTimeout(() => {
        if (this._requestListeners.has(id)) {
          const error = new Error(`Request timed out: ${action}`);
          this._requestListeners.get(id).reject(error);
          this._requestListeners.delete(id);
        }
      }, 300000);

      const success = this._sendMessage(action, walletAddress, params, id);
      if (!success) {
        clearTimeout(timeoutId);
        this._requestListeners.delete(id);
        reject(new Error('Failed to send request to wallet'));
      }
    });
  }

  async requestAccounts() {
    try {
      if (this._accounts.length > 0) {
        return this._accounts;
      }

      console.log('Requesting walletAddress from wallet...');

      const result = await this.request({
        action: 'psy_requestAccounts',
        walletAddress: null,
        params: []
      });

      console.log('Received walletAddress from wallet:', result);

      return result || [];
    } catch (error) {
      console.error('Error requesting walletAddress:', error);
      throw error;
    }
  }

  async getAccounts() {
    try {
      if (!this._accounts.includes(accountAddress)) {
        console.warn('Account not found in wallet:', accountAddress);
      }

      console.log('Getting walletAddress from wallet...');

      const result = await this.request({
        action: 'psy_accounts',
        walletAddress: null,
        params: []
      });

      console.log('Received walletAddress from wallet:', result);

      return result || [];
    } catch (error) {
      console.error('Error requesting walletAddress:', error);
      throw error;
    }
  }

  async sign(accountAddress, callArgs) {
    if (!accountAddress) {
      throw new Error('Wallet address is required');
    }
    if (!callArgs || typeof callArgs !== 'object') {
      throw new Error('Invalid call arguments. Expected an object.');
    }

    const requiredFields = ['contract_id', 'method_name', 'inputs'];
    const missingFields = requiredFields.filter(field => !(field in callArgs));
    if (missingFields.length > 0) {
      throw new Error(`Missing required fields in callArgs: ${missingFields.join(', ')}`);
    }

    try {
      const result = await this.request({
        action: 'psy_sign',
        walletAddress: accountAddress,
        params: [callArgs]
      });

      console.log('Received signature from wallet:', result);

      return result || null;
    } catch (error) {
      console.error('Error during signing:', error);
      throw error;
    }
  }

  on(event, listener) {
    if (typeof listener !== 'function') {
      console.error('Listener must be a function');
      return;
    }
    if (!this._events.has(event)) {
      this._events.set(event, new Set());
    }
    this._events.get(event).add(listener);
  }

  removeListener(event, listener) {
    if (typeof listener !== 'function') {
      console.error('Listener must be a function');
      return;
    }
    this._events.get(event)?.delete(listener);
  }

  _emit(event, data) {
    if (!this._events.has(event)) return;
    const listeners = new Set(this._events.get(event));
    listeners.forEach(listener => {
      try {
        listener(data);
      } catch (error) {
        console.error(`Error in ${event} listener:`, error);
      }
    });
  }
}

// Injected into window.psy
if (!window.psy) {
  window.psy = new PsyProvider();
  window.dispatchEvent(new Event('psy#initialized'));
}

console.log("Psy Wallet initialized.");



