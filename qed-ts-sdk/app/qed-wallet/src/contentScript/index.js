class MessageChannel {
  constructor(name) {
    this.name = name;
  }

  send(type, data) {
    window.postMessage({
      source: this.name,
      type: type,
      data: data
    }, '*');
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

export const generateRequestId = () => {
  const randomHex = (length) => {
    return Array.from({ length }, () =>
      Math.floor(Math.random() * 16).toString(16)
    ).join('');
  };

  // UUID v498-4-4-4-12)
  return [
    randomHex(8),
    randomHex(4),
    '4' + randomHex(3),
    Math.floor(Math.random() * 4 + 8).toString(16) + randomHex(3),
    randomHex(12)
  ].join('-');
};


const CONTENT_SCRIPT = "psy-contentscript";
const contentScript = {
  init() {
    if (typeof MessageChannel !== 'function') {
      console.error('MessageChannel Undefined');
      return;
    }

    this.channel = new MessageChannel(CONTENT_SCRIPT);

    if (typeof this.channel.send !== 'function') {
      console.error('channel.send method does not exist');
      return;
    }

    console.log('The message channel has been initialized successfully, and the send method is available.');

    this.registerListeners();
    this.inject();
    this.reportUrl();
  },

  reportUrl() {
    const connection = chrome.runtime.connect({ name: CONTENT_SCRIPT });
    connection.onDisconnect.addListener(() => {
      console.log('The connection to the backend has been disconnected.');
    });
  },

  registerListeners() {
    const self = this;

    window.addEventListener("message", (event) => {
      const { data: eventData, isTrusted } = event;

      const { isPsy = false, message, source } = eventData;

      if (!isTrusted) {
        console.warn('Ignore untrusted messages');
        return;
      }
      if (!isPsy || (!message && !source)) {
        console.warn('Ignore messages that do not conform to the format');
        return;
      }
      if (source === CONTENT_SCRIPT) {
        console.log('Ignore the messages sent by oneself');
        return;
      }

      const { data } = message;
      if (!data || !data.action) {
        console.warn('The message lacks the necessary action field');
        return;
      }

      console.log("content script process messages:", message);

      sendMsg(
        {
          id: generateRequestId(),
          action: data.action,
          messageSource: "messageFromDapp",
          callArgs: data.callArgs,
        },
        (params) => {
          console.log('Preparing to send a response message:', params);
          self.channel.send("messageFromWallet", params);
        }
      );
    });

    chrome.runtime.onMessage.addListener(
      (message, sender, sendResponse) => {
        console.log("Received a background message:", message);
        try {
          if (message.id) {
            this.channel.send("messageFromWallet", message);
          } else {
            this.channel.send(message.action, message.result);
          }
          sendResponse("content-back");
        } catch (error) {
          console.error("Failed to send background message response:", error);
          sendResponse("error: " + error.message);
        }
        return true;
      }
    );
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
