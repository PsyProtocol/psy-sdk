const pendingResponses = {}; // requestId -> sendResponse

chrome.runtime.onMessage.addListener((msg, sender, sendResponse) => {
  console.log("background script recived message:", msg);
  if (!msg.isPsy) return;
  if (['psy_requestAccounts', 'psy_accounts', 'psy_sign'].includes(msg.action)) {
    const { id, walletAddress, action, callArgs, dappUrl } = msg;
    if (!id) {
      sendResponse({ error: 'Missing request id' });
      return;
    }
    // Save sendResponse for later invocation
    pendingResponses[id] = sendResponse;

    let msgParams = {
      id: id,
      action: action,
      walletAddress: walletAddress,
      callArgs: callArgs,
      dappUrl: dappUrl,
      timeStamp: Date.now()
    }

    let jsonParams = JSON.stringify(msgParams);

    console.log("json params: ", jsonParams);

    const base64Params = btoa(unescape(encodeURIComponent(jsonParams)));
    console.log("base64 params: ", base64Params);

    chrome.windows.create({
      url: chrome.runtime.getURL(`src/components/DappService/index.html#params=${base64Params}`),
      type: "popup",
      width: 360,
      height: 600,
      focused: true
    });

    return true;
  }

  if (msg.action === "approval-result") {
    const { id, result, error } = msg;
    const callback = pendingResponses[id];

    if (callback) {
      if (!error) {
        console.log("background script approval-result:", msg);
        callback({ id: id, result: result, error: null });
      } else {
        callback({ id: id, result: null, error: error || 'User rejected' });
      }
      delete pendingResponses[id];
    }

    sendResponse({ status: 'result received' });
    return;
  }
});
