class MessageChannel {
  constructor(name) {
    this.name = name;
  }

  send(type, data) {
    window.postMessage({
      isPsy: true,
      source: this.name,
      type: type,
      data: data
    }, '*');
    console.log(`[ContentScript] forward the message (id: ${data.id || 'none'}):`, data);
  }
}

function sendMsg(message, sendResponse, errorCallback) {
  const { action } = message;
  chrome.runtime.sendMessage(
    { ...message },
    (params) => {
      if (chrome.runtime.lastError) {
        console.error("Failed to send the message:", action, chrome.runtime.lastError);
        if (errorCallback) errorCallback(chrome.runtime.lastError);
        return;
      }
      sendResponse && sendResponse(params);
    }
  );
}


const CONTENT_SCRIPT = "psy-contentscript";
const contentScript = {
  init() {
    if (typeof MessageChannel !== 'function') {
      console.error('MessageChannel Undefined');
      return;
    }

    this.channel = new MessageChannel(CONTENT_SCRIPT);

    if (typeof this.channel.send !== 'function') {
      console.error('channel.send action does not exist');
      return;
    }

    console.log('The message channel has been initialized successfully, and the send action is available.');

    this.registerListeners();
    this.inject();
    this.reportUrl();
  },

  reportUrl() {
    try {
      const connection = chrome.runtime.connect({ name: CONTENT_SCRIPT });
      connection.postMessage({ type: 'reportUrl', url: window.location.href });
      connection.onDisconnect.addListener(() => {
        console.log('[ContentScript] Background connection disconnected');
      });
    } catch (err) {
      console.error('[ContentScript] Report URL failed:', err);
    }
  },

  registerListeners() {
    const self = this;

    window.addEventListener("message", (event) => {
      const { data: eventData, isTrusted } = event;

      const { id, isPsy, action, callArgs, source, walletAddress } = eventData;

      if (!isTrusted) {
        console.warn('Ignore untrusted messages');
        return;
      }
      if (!isPsy || (id === undefined && !action)) {
        console.warn('Ignore messages that do not conform to the format');
        return;
      }
      if (source && source === CONTENT_SCRIPT) {
        console.log('Ignore the messages sent by oneself');
        return;
      }

      console.log('Content script received message from wallet:', eventData);
      if (!action) {
        console.warn('The message lacks the necessary action field');
        self.channel.send('messageFromWallet', {
          id: id,
          error: 'Missing action in message',
          data: null
        });
        return;
      }

      if (action === 'psy_sign' && !walletAddress) {
        console.error('The message lacks the necessary walletAddress field');
        self.channel.send('messageFromWallet', {
          id: id,
          error: 'Missing walletAddress in sign action',
          data: null
        });
        return;
      }

      console.log("content script process messages: ", { action, walletAddress, callArgs });

      sendMsg(
        {
          isPsy,
          id,
          action,
          walletAddress,
          messageSource: "messageFromDapp",
          callArgs,
        },
        (params) => {
          console.log('Preparing to send a response message:', params);
          self.channel.send('messageFromWallet', {
            id: id,
            result: params.result || null,
            error: params.error || null
          });
        },
        (err) => {
          self.channel.send('messageFromWallet', {
            id: id,
            result: null,
            error: `Send to background failed: ${err.message}`
          });
        }
      );
    });

    chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
      console.log('[ContentScript] Receive background message:', message);
      try {
        if (message.id) {
          self.channel.send('messageFromWallet', {
            id: message.id,
            data: message.result || null,
            error: message.error || null
          });
        } else if (message.action === 'accountsChanged') {
          self.channel.send('accountsChanged', {
            data: { accounts: message.data.accounts }
          });
        }
        sendResponse({ status: 'success' });
      } catch (err) {
        sendResponse({ status: 'error', message: err.message });
      }
      return true;
    });
  },

  inject() {
    console.log("Initializing Psy contentScript.js: ", chrome.runtime.id);
    const hostPage = document.head || document.documentElement;
    const script = document.createElement("script");

    const possiblePaths = [
      "assets/webHook.js",
      "webHook.js",
      "/assets/webHook.js"
    ];

    script.src = chrome.runtime.getURL(possiblePaths[0]);
    console.log(`Attempt to load webHook.js path: ${script.src}`);

    script.onload = function () {
      this.parentNode.removeChild(this);
      console.log("webHook.js loaded successfully");
    };

    script.onerror = function (err) {
      console.error(`webHook.js path ${possiblePaths[0]} loading failed`, err);
      if (possiblePaths.length > 1) {
        possiblePaths.shift();
        script.src = chrome.runtime.getURL(possiblePaths[0]);
        console.log(`Try the next path: ${script.src}`);
        hostPage.appendChild(script);
      }
    };

    hostPage.appendChild(script);
  },
};

// Initialize content script
contentScript.init();
