const pendingResponses = {}; // requestId -> sendResponse

chrome.runtime.onMessage.addListener((msg, sender, sendResponse) => {
  console.log("background script recived message:", msg);
  if (msg.action === "requestAccount" || msg.action === "sign") {
    const id = msg.id;
    console.log("msg.id: ", id);
    // Save sendResponse for later invocation
    pendingResponses[id] = sendResponse;

    let msgParams = {
      id: msg.id,
      action: msg.action,
      callArgs: msg.callArgs,
    }

    let jsonParams = JSON.stringify(msgParams);

    console.log("json params: ", jsonParams);

    const base64Params = btoa(unescape(encodeURIComponent(jsonParams)));
    console.log("base64 params: ", base64Params);

    chrome.windows.create({
      url: chrome.runtime.getURL(`src/components/DappService/index.html#params=${base64Params}`),
      type: "popup",
      width: 360,
      height: 600
    });

    return true;
  }

  if (msg.action === "approval-result") {
    const { id, ok } = msg;
    console.log("recive approval-result:", msg);
    const sendResponse = pendingResponses[id];
    if (sendResponse) {
      sendResponse({
        id,
        result: ok ? msg.walletAddress : null,
        error: ok ? null : "User rejected"
      });
      delete pendingResponses[id];
    }
  }
});
