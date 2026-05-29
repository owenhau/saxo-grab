// Inject interceptor.js into the main world
const script = document.createElement('script');
script.src = chrome.runtime.getURL('interceptor.js');
script.onload = function() {
  this.remove();
};
(document.head || document.documentElement).appendChild(script);

function updateTargetStatus(targetName, state) {
  const key = 'status_' + targetName;
  chrome.storage.local.set({ [key]: {
    state: state,
    timestamp: Date.now()
  }});
}

// Listen for messages from the interceptor
window.addEventListener('message', function(event) {
  if (event.source !== window || !event.data || event.data.type !== 'SAXO_DATA_INTERCEPTED') {
    return;
  }

  const { target, data } = event.data;
  
  // Extract target name (e.g., balances, netpositions, orders, news, transactions)
  let targetName = 'unknown';
  if (target.includes('balances')) targetName = 'balances';
  else if (target.includes('netpositions')) targetName = 'netpositions';
  else if (target.includes('orders')) targetName = 'orders';
  else if (target.includes('news')) targetName = 'news';
  else if (target.includes('transactions')) targetName = 'transactions';
  else if (target.includes('earningsbreakdown')) targetName = 'earnings';
  else if (target.includes('watchlists')) targetName = 'watchlists';
  else if (target.includes('charts')) targetName = 'stockCharts';

  if (targetName === 'unknown') return;

  const payload = {
    target: targetName,
    data: data,
    timestamp: Date.now(),
    url: target
  };

  console.log('Pushing Saxo data to local server for:', targetName);
  updateTargetStatus(targetName, 'updating');

  fetch('http://127.0.0.1:9876', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json'
    },
    body: JSON.stringify(payload)
  })
  .then(response => {
    if (!response.ok) {
      console.error('Failed to push data to server:', response.statusText);
      updateTargetStatus(targetName, 'error');
    } else {
      console.log('Successfully pushed data for:', targetName);
      updateTargetStatus(targetName, 'success');
    }
  })
  .catch(error => {
    console.error('Error pushing data to server (is it running?):', error);
    updateTargetStatus(targetName, 'error');
  });
});
