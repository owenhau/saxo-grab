document.addEventListener('DOMContentLoaded', function() {
  const items = document.querySelectorAll('.item');

  function updateUI() {
    chrome.storage.local.get(null, function(data) {
      items.forEach(item => {
        const target = item.getAttribute('data-target');
        const statusKey = 'status_' + target;
        const statusData = data[statusKey] || { state: 'waiting', timestamp: null };
        
        const statusSpan = item.querySelector('.status');
        
        // Update text
        let statusText = statusData.state;
        if (statusData.timestamp && statusData.state !== 'error') {
          const time = new Date(statusData.timestamp).toLocaleTimeString();
          statusText = `${statusData.state} (${time})`;
        }
        statusSpan.textContent = statusText;
        
        // Update class for styling
        statusSpan.className = 'status ' + statusData.state;
      });
    });
  }

  // Initial update
  updateUI();
  
  // Listen for changes in storage
  chrome.storage.onChanged.addListener(() => {
    updateUI();
  });
});
