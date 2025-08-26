
class PsyProvider {
  constructor() {
    this._events = new Map();
    this._initMessageListener();
  }

  // Initialize message monitoring (receive background responses forwarded by content scripts)
  _initMessageListener() {
    window.addEventListener('message', (event) => {
      if (event.source !== window || !event.data?.psy) return;
      console.log('Received message:', event.data);
      const { type, data } = event.data;
      switch (type) {
        case 'accountsChanged':
          this._emit('accountsChanged', data.accounts);
          break;
        case 'signatureSuccess':
          this._emit('signatureSuccess', { id: data.id, signature: data.signature });
          break;
        case 'signatureFailed':
          this._emit('signatureFailed', { id: data.id, error: data.error });
          break;
      }
    });
  }

  // Send a message to the content script (relay to the background)
  _sendMessage(method, params, id) {
    window.postMessage({
      psy: true,
      type: 'request',
      data: { method, params, id },
    }, '*');
  }

  // EIP-1193 method: handling DApp requests(login, signature, etc.)
  async request({ method, params }) {
    return new Promise((resolve, reject) => {
      const id = Math.random().toString(36).slice(2, 10);

      // Listen to the background response
      const listener = (event) => {
        if (event.source !== window || !event.data?.psy || event.data.id !== id) return;
        window.removeEventListener('message', listener);

        if (event.data.error) reject(new Error(event.data.error));
        else resolve(event.data.result);
      };
      window.addEventListener('message', listener);

      // Send a request to the backend
      this._sendMessage(method, params, id);
    });
  }

  // Event listening (e.g., DApp listening for login status changes: window.psy.on('accountsChanged', (accounts) => {}))
  on(event, listener) {
    if (!this._events.has(event)) this._events.set(event, new Set());
    this._events.get(event).add(listener);
  }

  removeListener(event, listener) {
    this._events.get(event)?.delete(listener);
  }

  _emit(event, data) {
    this._events.get(event)?.forEach((listener) => listener(data));
  }
}

// Injected into window.psy
if (!window.psy) {
  window.psy = new PsyProvider();
  window.dispatchEvent(new Event('psy#initialized'));
}

console.log("Psy Wallet initialized.");



