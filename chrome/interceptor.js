(function() {
  const SIMPLE_TARGET_URLS = [
    '/oapi/portfolio/v3/balances/subscriptions',
    '/oapi/portfolio/v3/netpositions/subscriptions',
    '/oapi/portfolio/v3/orders/subscriptions',
    '/oapi/news/v1/subscriptions',
    '/openapi/trade/v1/watchlists/subscriptions',
    '/openapi/chart/v3/charts/subscriptions'
  ];

  const TRANSACTION_URL = '/openapi/hist/v1/transactions';
  const EARNINGS_URL = '/openapi/hist/v1/reports/earningsbreakdown/';

  function isTarget(url) {
    if (url.includes(TRANSACTION_URL) || url.includes(EARNINGS_URL)) {
      return true;
    }
    
    return SIMPLE_TARGET_URLS.some(target => url.includes(target));
  }

  function handleData(url, data) {
    let target = SIMPLE_TARGET_URLS.find(t => url.includes(t));
    if (url.includes(TRANSACTION_URL)) {
      target = TRANSACTION_URL;
    } else if (url.includes(EARNINGS_URL)) {
      target = EARNINGS_URL;
    }
    
    if (target) {
      window.postMessage({
        type: 'SAXO_DATA_INTERCEPTED',
        target: target,
        data: data
      }, '*');
    }
  }

  // Intercept Fetch
  const originalFetch = window.fetch;
  window.fetch = async (...args) => {
    const response = await originalFetch(...args);
    const url = typeof args[0] === 'string' ? args[0] : args[0].url;

    if (isTarget(url)) {
      const clone = response.clone();
      clone.json().then(data => handleData(url, data)).catch(e => console.error('Saxo Grabber: Error parsing fetch JSON:', e));
    }
    return response;
  };

  // Intercept XHR
  const originalOpen = XMLHttpRequest.prototype.open;
  const originalSend = XMLHttpRequest.prototype.send;

  XMLHttpRequest.prototype.open = function(method, url) {
    this._url = url;
    return originalOpen.apply(this, arguments);
  };

  XMLHttpRequest.prototype.send = function() {
    this.addEventListener('load', function() {
      if (isTarget(this._url)) {
        try {
          const data = JSON.parse(this.responseText);
          handleData(this._url, data);
        } catch (e) {
          console.error('Saxo Grabber: Error parsing XHR JSON:', e);
        }
      }
    });
    return originalSend.apply(this, arguments);
  };

  console.log('Saxo Interceptor Loaded');
})();
