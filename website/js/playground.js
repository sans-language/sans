const API = 'https://api.sans.dev';

// Fix the HTML entities in the textarea default value
const editorEl = document.getElementById('editor');
editorEl.value = editorEl.value.replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&amp;/g, '&');
const DEFAULT_CODE = editorEl.value;

// Sans syntax highlighter
function highlightSans(code) {
  const esc = code.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  return esc
    // Strings (double-quoted)
    .replace(/"([^"\\]|\\.)*"/g, '<span style="color:#34d399">$&</span>')
    // Comments
    .replace(/(\/\/.*)$/gm, '<span style="color:#64748b;font-style:italic">$1</span>')
    // Numbers
    .replace(/\b(\d+\.?\d*)\b/g, '<span style="color:#fb923c">$1</span>')
    // Keywords
    .replace(/\b(if|else|while|for|in|match|return|break|continue|struct|enum|trait|impl|fn|let|import|pub|spawn|main|true|false)\b/g, '<span style="color:#c084fc">$1</span>')
    // Types
    .replace(/\b(I|F|B|S|J|R|O|M|Array|Map|String|Int|Float|Bool|Result|Option|HttpServer|HttpRequest|HttpResponse|Sender|Receiver|Mutex|JoinHandle|dyn)\b/g, '<span style="color:#fbbf24">$1</span>')
    // Built-in functions
    .replace(/\b(p|str|stoi|itof|ftoi|ftos|range|sleep|time|now|random|rand|fr|fw|fa|fe|jp|jfy|jo|ja|js|ji|jb|jn|hg|hp|ok|err|some|none|assert|assert_eq|assert_ne|serve|listen|channel|mutex|fptr|spawn)\b(?=\s*\()/g, '<span style="color:#60a5fa">$1</span>');
}

function updateHighlight() {
  const code = editorEl.value;
  document.getElementById('highlight').innerHTML = highlightSans(code) + '\n';
  updateLineNumbers();
  syncScroll();
}

function updateLineNumbers() {
  const lines = editorEl.value.split('\n').length;
  const nums = [];
  for (let i = 1; i <= lines; i++) nums.push('<div>' + i + '</div>');
  document.getElementById('line-numbers').innerHTML = nums.join('');
}

function syncScroll() {
  const hl = document.getElementById('highlight');
  const ln = document.getElementById('line-numbers');
  hl.scrollTop = editorEl.scrollTop;
  hl.scrollLeft = editorEl.scrollLeft;
  ln.scrollTop = editorEl.scrollTop;
}

editorEl.addEventListener('input', updateHighlight);
editorEl.addEventListener('scroll', syncScroll);
editorEl.addEventListener('keydown', function(e) {
  if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
    e.preventDefault();
    runCode();
  }
  if (e.key === 'Tab') {
    e.preventDefault();
    const start = this.selectionStart;
    this.value = this.value.substring(0, start) + '    ' + this.value.substring(this.selectionEnd);
    this.selectionStart = this.selectionEnd = start + 4;
    updateHighlight();
  }
});

// Initial render
updateHighlight();

// Load snippet from URL query param (?s=ID) or path (/play/ID)
(function() {
  const params = new URLSearchParams(window.location.search);
  let id = params.get('s');
  if (!id) {
    const path = window.location.pathname.replace(/\/$/, '');
    const parts = path.split('/');
    const last = parts[parts.length - 1];
    if (last && last !== 'play' && last.length === 8) id = last;
  }
  if (id && /^[a-zA-Z0-9]{8}$/.test(id)) {
    fetch(API + '/api/snippet/' + id)
      .then(r => r.ok ? r.json() : Promise.reject())
      .then(data => { editorEl.value = data.code; updateHighlight(); })
      .catch(() => {
        document.getElementById('output').innerHTML = '<span class="error">Snippet not found. Loading default example.</span>';
      });
  }
})();

async function runCode() {
  const code = editorEl.value;
  const output = document.getElementById('output');
  const status = document.getElementById('status');
  const btn = document.getElementById('run-btn');

  btn.disabled = true;
  btn.textContent = 'Running...';
  output.innerHTML = '<span class="playground-spinner">Compiling and running...</span>';
  status.textContent = '';

  try {
    const res = await fetch(API + '/api/run', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ code }),
    });

    if (res.status === 429) {
      output.innerHTML = '<span class="error">Rate limit exceeded. Please wait a moment.</span>';
      return;
    }

    const data = await res.json();
    let html = '';
    if (data.stderr) {
      html += '<span class="error">' + escapeHtml(data.stderr) + '</span>\n';
    }
    if (data.stdout) {
      html += '<span class="success">' + escapeHtml(data.stdout) + '</span>';
    }
    if (!data.stdout && !data.stderr) {
      html = '<span class="success">(no output)</span>';
    }
    output.innerHTML = html;
    status.textContent = data.compile_success ? 'Compiled OK' : 'Compile error';
    status.style.color = data.compile_success ? '#34d399' : '#f87171';
  } catch (e) {
    output.innerHTML = '<span class="error">Failed to connect to playground server.</span>';
  } finally {
    btn.disabled = false;
    btn.textContent = 'Run';
  }
}

async function shareCode() {
  const code = editorEl.value;
  try {
    const res = await fetch(API + '/api/share', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ code }),
    });
    const data = await res.json();
    const url = window.location.origin + '/play/' + data.id;
    window.history.pushState({}, '', '/play/?s=' + data.id);
    navigator.clipboard.writeText(url).then(() => {
      document.getElementById('status').textContent = 'Link copied!';
      document.getElementById('status').style.color = '#34d399';
    });
  } catch (e) {
    document.getElementById('status').textContent = 'Share failed';
    document.getElementById('status').style.color = '#f87171';
  }
}

function resetCode() {
  editorEl.value = DEFAULT_CODE;
  updateHighlight();
  document.getElementById('output').textContent = 'Press Run to execute your code.';
  document.getElementById('status').textContent = '';
  window.history.pushState({}, '', '/play/');
  updateHighlight();
}

function escapeHtml(s) {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;').replace(/'/g, '&#39;');
}
