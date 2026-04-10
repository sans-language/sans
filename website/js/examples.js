document.querySelectorAll('.btn-try').forEach(btn => {
  btn.addEventListener('click', function(e) {
    e.preventDefault();
    const card = this.closest('.example-card');
    const code = card.querySelector('pre code').textContent;
    fetch('https://api.sans.dev/api/share', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ code }),
    })
    .then(r => r.json())
    .then(data => { window.open('/play/?s=' + data.id, '_blank'); })
    .catch(() => { window.open('/play/', '_blank'); });
  });
});
