# Implementation Plan: SaxoTrader Network Interceptor Chrome Extension

### Objective
Create a Manifest V3 Chrome Extension that silently intercepts network requests on `https://www.saxotrader.com/d/portfolio/tradingaccounts?*`, captures the JSON responses for specific endpoints, and allows the user to download them by clicking the extension icon.

### Key Files & Architecture
1.  **`manifest.json`**: Configuration file for the extension (Manifest V3), requesting permissions for storage, downloads, and access to the SaxoTrader domain.
2.  **`interceptor.js`**: A script injected directly into the web page's environment (the "Main World"). It safely overrides the browser's `fetch` and `XMLHttpRequest` functions to "listen" for requests matching the target URLs. When it sees one, it copies the JSON response and passes it to our content script.
3.  **`content.js`**: Runs in an isolated environment on the page. It receives the copied JSON data from `interceptor.js` and saves it securely into the extension's local storage (`chrome.storage.local`).
4.  **`popup.html` & `popup.css`**: The user interface that appears when you click the extension icon. It will check local storage for captured data and provide a button to download the files (as `.json`) to your local machine.
5.  **`popup.js`**: Logic to display the status of captured endpoints and trigger the downloads.
6.  **`background.js`**: A background service worker that handles the actual file downloading mechanism (`chrome.downloads`) when instructed by the popup.

### Implementation Steps
1.  Initialize the project directory and create the `manifest.json`.
2.  Develop `interceptor.js` to target the four specific endpoints:
    *   `/oapi/portfolio/v3/balances/subscriptions`
    *   `/oapi/portfolio/v3/netpositions/subscriptions`
    *   `/oapi/portfolio/v3/orders/subscriptions`
    *   `/oapi/news/v1/subscriptions`
3.  Develop `content.js` to dynamically inject `interceptor.js` into the page and listen for incoming messages containing the JSON payloads.
4.  Develop `background.js` to listen for download requests and execute them.
5.  Create the UI (`popup.html` and `popup.css`) and logic (`popup.js`) to display the status of captured endpoints and trigger the downloads.

### Verification & Testing
*   Load the extension locally in Chrome via `chrome://extensions/` (Developer Mode -> Load unpacked).
*   Navigate to the specified SaxoTrader URL.
*   Verify that the extension icon popup correctly indicates when data from the 4 endpoints has been intercepted.
*   Click the download buttons and verify the output JSON matches expectations.
