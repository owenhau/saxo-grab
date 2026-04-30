document.addEventListener('DOMContentLoaded', function() {
  const items = document.querySelectorAll('.item');
  const clearBtn = document.getElementById('clear-data');
  const downloadAllBtn = document.getElementById('download-all');

  function getFilename(target) {
    if (target.includes('balances')) return 'balances.json';
    if (target.includes('netpositions')) return 'netpositions.json';
    if (target.includes('orders')) return 'orders.json';
    if (target.includes('news')) return 'news.json';
    return 'data.json';
  }

  function updateStatus() {
    chrome.storage.local.get(null, function(data) {
      let anyCaptured = false;
      items.forEach(item => {
        const target = item.getAttribute('data-target');
        const key = 'saxo_data_' + target.replace(/\//g, '_');
        const stored = data[key];
        const statusSpan = item.querySelector('.status');
        const downloadBtn = item.querySelector('.download-btn');

        if (stored) {
          anyCaptured = true;
          statusSpan.textContent = 'Captured: ' + new Date(stored.timestamp).toLocaleTimeString();
          statusSpan.classList.add('ready');
          downloadBtn.disabled = false;
          
          downloadBtn.onclick = function() {
            chrome.runtime.sendMessage({
              type: 'DOWNLOAD_JSON',
              data: stored.data,
              filename: getFilename(target)
            });
          };
        } else {
          statusSpan.textContent = 'Waiting...';
          statusSpan.classList.remove('ready');
          downloadBtn.disabled = true;
        }
      });
      downloadAllBtn.disabled = !anyCaptured;
    });
  }

  downloadAllBtn.addEventListener('click', function() {
    chrome.storage.local.get(null, function(data) {
      items.forEach(item => {
        const target = item.getAttribute('data-target');
        const key = 'saxo_data_' + target.replace(/\//g, '_');
        const stored = data[key];
        if (stored) {
          chrome.runtime.sendMessage({
            type: 'DOWNLOAD_JSON',
            data: stored.data,
            filename: getFilename(target)
          });
        }
      });
    });
  });

  clearBtn.addEventListener('click', function() {
    chrome.storage.local.clear(function() {
      updateStatus();
    });
  });

  updateStatus();
  // Poll for updates every second while popup is open
  setInterval(updateStatus, 1000);
});
