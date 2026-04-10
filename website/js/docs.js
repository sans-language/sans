function tryInPlayground(code) {
  fetch('https://api.sans.dev/api/share', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ code }),
  })
  .then(r => r.json())
  .then(data => { window.open('/play/?s=' + data.id, '_blank'); })
  .catch(() => { window.open('/play/', '_blank'); });
}

// Add "Try it" buttons to all code blocks with class "tryable"
document.querySelectorAll('pre.tryable').forEach(pre => {
  const code = pre.querySelector('code').textContent;
  const btn = document.createElement('button');
  btn.className = 'try-it-btn';
  btn.textContent = 'Try it';
  btn.onclick = function() { tryInPlayground(code); };
  pre.insertBefore(btn, pre.firstChild);
});
