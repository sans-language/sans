function copyCmd(cmd, btn) {
  navigator.clipboard.writeText(cmd).then(function() {
    btn.textContent = 'Copied!';
    setTimeout(function() { btn.textContent = 'Copy'; }, 2000);
  }).catch(function() {
    btn.textContent = 'Copy failed';
    setTimeout(function() { btn.textContent = 'Copy'; }, 2000);
  });
}

document.querySelectorAll('.copy-btn').forEach(function(btn) {
  btn.addEventListener('click', function() {
    var pre = btn.previousElementSibling;
    if (!pre) return;
    var cmd = pre.textContent.replace(/&amp;/g, '&');
    copyCmd(cmd, btn);
  });
});
