chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.type === 'DOWNLOAD_JSON') {
    const blob = new Blob([JSON.stringify(message.data, null, 2)], { type: 'application/json' });
    const reader = new FileReader();
    
    reader.onload = function() {
      const url = reader.result;
      chrome.downloads.download({
        url: url,
        filename: message.filename,
        saveAs: true
      });
    };
    
    reader.readAsDataURL(blob);
  }
});
