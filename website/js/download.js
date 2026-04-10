function copyCmd(id, btn) {
  var cmd = document.getElementById(id).textContent.replace(/&amp;/g, '&');
  navigator.clipboard.writeText(cmd).then(function() {
    btn.textContent = 'Copied!';
    setTimeout(function() { btn.textContent = 'Copy'; }, 2000);
  });
}
function copyInstall() { copyCmd('install-cmd', document.querySelectorAll('.copy-btn')[0]); }
function copyInstallLinux() { copyCmd('install-cmd-linux', document.querySelectorAll('.copy-btn')[1]); }
function copyInstallWindows() { copyCmd('install-cmd-windows', document.querySelectorAll('.copy-btn')[2]); }
