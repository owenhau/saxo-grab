document.addEventListener('DOMContentLoaded', function() {
  const items = document.querySelectorAll('.item');
  const clearBtn = document.getElementById('clear-data');

  function updateStatus() {
    chrome.storage.local.get(null, function(data) {
      items.forEach(item => {
        const target = item.getAttribute('data-target');
        const key = 'saxo_data_' + target.replace(/\//g, '_');
        const stored = data[key];
        const statusSpan = item.querySelector('.status');
        const downloadBtn = item.querySelector('.download-btn');

        if (stored) {
          statusSpan.textContent = 'Captured: ' + new Date(stored.timestamp).toLocaleTimeString();
          statusSpan.classList.add('ready');
          downloadBtn.disabled = false;
          
          downloadBtn.onclick = function() {
            const filename = target.split('/').pop() + '_' + stored.timestamp + '.json';
            chrome.runtime.sendMessage({
              type: 'DOWNLOAD_JSON',
              data: stored.data,
              filename: filename
            });
          };
        } else {
          statusSpan.textContent = 'Waiting...';
          statusSpan.classList.remove('ready');
          downloadBtn.disabled = true;
        }
      });
    });
  }

  clearBtn.addEventListener('click', function() {
    chrome.storage.local.clear(function() {
      updateStatus();
    });
  });

  updateStatus();
  // Poll for updates every second while popup is open
  setInterval(updateStatus, 1000);
});
