# SaxoTrader JSON Grabber

A Chrome Extension to intercept and retrieve specific JSON data from the SaxoTrader portfolio page.

## Features

- **Automatic Interception**: Automatically detects and captures JSON responses from specific SaxoTrader API endpoints.
- **Targeted Endpoints**:
    - `/oapi/portfolio/v3/balances/subscriptions`
    - `/oapi/portfolio/v3/netpositions/subscriptions`
    - `/oapi/portfolio/v3/orders/subscriptions`
    - `/oapi/news/v1/subscriptions`
- **Easy Download**: Download the captured JSON data directly through the extension popup.
- **Persistent Storage**: Captured data is stored locally in the extension until cleared.

## Installation

1.  Clone or download this repository to your local machine.
2.  Open Google Chrome and navigate to `chrome://extensions/`.
3.  In the top right corner, enable **Developer mode**.
4.  Click the **Load unpacked** button.
5.  Select the folder containing this extension's files (`saxo-grab`).

## How to Use

1.  Navigate to the SaxoTrader portfolio page: `https://www.saxotrader.com/d/portfolio/tradingaccounts`.
2.  The extension will silently listen for background network requests.
3.  Click the extension icon in the Chrome toolbar to open the popup.
4.  The popup will display the status of each target endpoint:
    - **Waiting...**: The endpoint hasn't been requested by the page yet.
    - **Captured [Time]**: Data has been successfully intercepted.
5.  Click the **Download** button next to a captured item to save the JSON file to your computer.
6.  Use the **Clear Stored Data** button to remove all captured data from the extension's storage.

## Architecture

- **Manifest V3**: Uses the latest Chrome Extension API.
- **Network Interception**: Overrides `fetch` and `XMLHttpRequest` in the page's main world to capture data without re-fetching.
- **Chrome Storage**: Uses `chrome.storage.local` for secure data persistence.
- **Downloads API**: Uses `chrome.downloads` for reliable file saving.

## Security Note

This extension is designed for personal use to help retrieve your own trading data. It only listens for specific endpoints on the SaxoTrader domain. Always be cautious when using extensions that intercept network traffic.
