// Inject interceptor.js into the main world
const script = document.createElement('script');
script.src = chrome.runtime.getURL('interceptor.js');
script.onload = function() {
  this.remove();
};
(document.head || document.documentElement).appendChild(script);

// Listen for messages from the interceptor
window.addEventListener('message', function(event) {
  if (event.source !== window || !event.data || event.data.type !== 'SAXO_DATA_INTERCEPTED') {
    return;
  }

  const { target, data } = event.data;
  const key = 'saxo_data_' + target.replace(/\//g, '_');
  
  chrome.storage.local.set({ [key]: {
    data: data,
    timestamp: Date.now(),
    url: target
  } }, function() {
    console.log('Saxo data saved for:', target);
  });
});
