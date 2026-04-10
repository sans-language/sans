const MINI_API = 'https://api.sans.dev';
const miniEl = document.getElementById('mini-editor');

// Fix HTML entities in textarea default
miniEl.value = miniEl.value.replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&amp;/g, '&');

function highlightSans(code) {
  const esc = code.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  return esc
    .replace(/"([^"\\]|\\.)*"/g, '<span style="color:#34d399">$&</span>')
    .replace(/(\/\/.*)$/gm, '<span style="color:#64748b;font-style:italic">$1</span>')
    .replace(/\b(\d+\.?\d*)\b/g, '<span style="color:#fb923c">$1</span>')
    .replace(/\b(if|else|while|for|in|match|return|break|continue|struct|enum|trait|impl|fn|let|import|pub|spawn|main|true|false)\b/g, '<span style="color:#c084fc">$1</span>')
    .replace(/\b(I|F|B|S|J|R|O|M|Array|Map|String|Int|Float|Bool|Result|Option|dyn)\b/g, '<span style="color:#fbbf24">$1</span>')
    .replace(/\b(p|str|stoi|itof|ftoi|ftos|range|sleep|time|now|random|rand|fr|fw|fa|fe|jp|jfy|jo|ja|js|ji|jb|jn|hg|hp|ok|err|some|none|assert|serve|listen|channel|mutex|fptr|spawn)\b(?=\s*\()/g, '<span style="color:#60a5fa">$1</span>');
}

function miniUpdate() {
  document.getElementById('mini-highlight').innerHTML = highlightSans(miniEl.value) + '\n';
  const lines = miniEl.value.split('\n').length;
  const nums = [];
  for (let i = 1; i <= lines; i++) nums.push('<div>' + i + '</div>');
  document.getElementById('mini-line-numbers').innerHTML = nums.join('');
}

function miniSync() {
  const hl = document.getElementById('mini-highlight');
  const ln = document.getElementById('mini-line-numbers');
  hl.scrollTop = miniEl.scrollTop;
  hl.scrollLeft = miniEl.scrollLeft;
  ln.scrollTop = miniEl.scrollTop;
}

miniEl.addEventListener('input', miniUpdate);
miniEl.addEventListener('scroll', miniSync);
miniUpdate();

function escapeHtml(s) {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;').replace(/'/g, '&#39;');
}

async function miniRun() {
  const code = miniEl.value;
  const output = document.getElementById('mini-output');
  const btn = document.getElementById('mini-run');
  btn.disabled = true; btn.textContent = 'Running...';
  output.innerHTML = '<span style="color:#94a3b8">Compiling...</span>';
  try {
    const res = await fetch(MINI_API + '/api/run', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ code }),
    });
    if (res.status === 429) { output.innerHTML = '<span class="error">Rate limit exceeded.</span>'; return; }
    const data = await res.json();
    let html = '';
    if (data.stderr) html += '<span class="error">' + escapeHtml(data.stderr) + '</span>\n';
    if (data.stdout) html += '<span class="success">' + escapeHtml(data.stdout) + '</span>';
    if (!data.stdout && !data.stderr) html = '<span class="success">(no output)</span>';
    output.innerHTML = html;
  } catch(e) {
    output.innerHTML = '<span class="error">Server unavailable.</span>';
  } finally {
    btn.disabled = false; btn.textContent = 'Run';
  }
}
miniEl.addEventListener('keydown', function(e) {
  if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') { e.preventDefault(); miniRun(); }
  if (e.key === 'Tab') { e.preventDefault(); const s = this.selectionStart; this.value = this.value.substring(0, s) + '    ' + this.value.substring(this.selectionEnd); this.selectionStart = this.selectionEnd = s + 4; miniUpdate(); }
});
