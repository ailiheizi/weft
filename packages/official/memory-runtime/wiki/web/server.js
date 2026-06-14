import http from 'node:http'

const host = process.env.HOSTNAME || '127.0.0.1'
const port = Number(process.env.PORT || '3000')
const apiBase = (process.env.NEXT_PUBLIC_API_URL || 'http://127.0.0.1:18000').replace(/\/$/, '')

function htmlEscape(value) {
  return String(value)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
}

function renderShell() {
  return `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Workspace Wiki</title>
    <style>
      :root {
        color-scheme: dark;
        --bg: #141821;
        --panel: #1b2230;
        --panel-alt: #20293a;
        --border: rgba(255,255,255,0.08);
        --text: #eef3ff;
        --muted: #97a6c2;
        --accent: #78a6ff;
      }
      * { box-sizing: border-box; }
      body {
        margin: 0;
        font-family: Inter, Segoe UI, Arial, sans-serif;
        background: var(--bg);
        color: var(--text);
      }
      .app {
        display: grid;
        grid-template-columns: 280px 1fr;
        height: 100vh;
      }
      .rail, .content { min-height: 0; }
      .rail {
        border-right: 1px solid var(--border);
        background: var(--panel);
        display: flex;
        flex-direction: column;
      }
      .content {
        background: var(--panel-alt);
        display: flex;
        flex-direction: column;
      }
      .header {
        padding: 14px 16px;
        border-bottom: 1px solid var(--border);
      }
      .title { font-size: 15px; font-weight: 700; }
      .subtitle { margin-top: 4px; color: var(--muted); font-size: 12px; }
      .search {
        width: 100%;
        margin-top: 12px;
        padding: 10px 12px;
        border-radius: 10px;
        border: 1px solid var(--border);
        background: #121723;
        color: var(--text);
      }
      .page-list {
        overflow: auto;
        padding: 10px;
        display: flex;
        flex-direction: column;
        gap: 8px;
      }
      .page-item {
        padding: 12px;
        border: 1px solid var(--border);
        border-radius: 12px;
        background: rgba(255,255,255,0.02);
        cursor: pointer;
      }
      .page-item.active { border-color: rgba(120,166,255,0.65); }
      .page-item-title { font-size: 13px; font-weight: 600; }
      .page-item-summary { margin-top: 4px; color: var(--muted); font-size: 12px; line-height: 1.45; }
      .content-body {
        overflow: auto;
        padding: 22px 24px;
      }
      .content-title { font-size: 22px; font-weight: 700; }
      .content-summary { margin-top: 8px; color: var(--muted); }
      .content-markdown {
        margin-top: 18px;
        padding: 18px;
        border-radius: 16px;
        border: 1px solid var(--border);
        background: rgba(0,0,0,0.16);
        white-space: pre-wrap;
        line-height: 1.6;
      }
      .empty, .error {
        padding: 24px;
        color: var(--muted);
      }
      .error { color: #ff8f8f; }
    </style>
  </head>
  <body>
    <div id="app" class="app">
      <aside class="rail">
        <div class="header">
          <div class="title">Workspace Wiki</div>
          <div class="subtitle">Minimal local wiki runtime</div>
          <input id="search" class="search" type="search" placeholder="Search wiki pages" />
        </div>
        <div id="page-list" class="page-list"></div>
      </aside>
      <main class="content">
        <div class="header">
          <div class="title">Page</div>
          <div class="subtitle" id="workspace-label"></div>
        </div>
        <div id="content-body" class="content-body"></div>
      </main>
    </div>
    <script>
      const apiBase = ${JSON.stringify(apiBase)};
      const params = new URLSearchParams(location.search);
      const workspaceId = params.get('workspace_id') || 'workspace-global';
      const pageListEl = document.getElementById('page-list');
      const contentBodyEl = document.getElementById('content-body');
      const searchEl = document.getElementById('search');
      const workspaceLabelEl = document.getElementById('workspace-label');
      workspaceLabelEl.textContent = workspaceId;

      let currentPages = [];
      let selectedPageId = null;

      function escapeHtml(value) {
        return ${htmlEscape.toString()}(value);
      }

      function renderPages() {
        if (!currentPages.length) {
          pageListEl.innerHTML = '<div class="empty">No wiki pages matched this workspace/query.</div>';
          return;
        }
        pageListEl.innerHTML = currentPages.map((page) => {
          const active = page.id === selectedPageId ? ' active' : '';
          return '<button class="page-item' + active + '" data-page-id="' + escapeHtml(page.id) + '">' +
            '<div class="page-item-title">' + escapeHtml(page.title) + '</div>' +
            '<div class="page-item-summary">' + escapeHtml(page.summary || '') + '</div>' +
          '</button>';
        }).join('');
        for (const button of pageListEl.querySelectorAll('[data-page-id]')) {
          button.addEventListener('click', () => {
            const nextId = button.getAttribute('data-page-id');
            if (!nextId) return;
            loadPage(nextId);
          });
        }
      }

      function renderError(message) {
        contentBodyEl.innerHTML = '<div class="error">' + escapeHtml(message) + '</div>';
      }

      async function loadPage(pageId) {
        selectedPageId = pageId;
        renderPages();
        contentBodyEl.innerHTML = '<div class="empty">Loading page…</div>';
        try {
          const response = await fetch(apiBase + '/v1/palace/wiki-page/' + encodeURIComponent(pageId) + '?workspace_id=' + encodeURIComponent(workspaceId));
          const payload = await response.json();
          if (!response.ok || !payload.ok) {
            throw new Error(payload.error || 'Failed to load wiki page.');
          }
          const page = payload.data;
          contentBodyEl.innerHTML =
            '<div class="content-title">' + escapeHtml(page.title || '') + '</div>' +
            '<div class="content-summary">' + escapeHtml(page.summary || '') + '</div>' +
            '<div class="content-markdown">' + escapeHtml(page.content || '') + '</div>';
        } catch (error) {
          renderError(error instanceof Error ? error.message : String(error));
        }
      }

      async function loadView(query = '') {
        pageListEl.innerHTML = '<div class="empty">Loading wiki…</div>';
        contentBodyEl.innerHTML = '<div class="empty">Loading wiki…</div>';
        try {
          const url = new URL(apiBase + '/v1/palace/wiki-view');
          url.searchParams.set('workspace_id', workspaceId);
          if (query.trim()) url.searchParams.set('query', query.trim());
          const response = await fetch(url);
          const payload = await response.json();
          if (!response.ok || !payload.ok) {
            throw new Error(payload.error || 'Failed to load wiki view.');
          }
          currentPages = Array.isArray(payload.data?.pages) ? payload.data.pages : [];
          selectedPageId = currentPages[0]?.id || null;
          renderPages();
          if (selectedPageId) {
            await loadPage(selectedPageId);
          } else {
            contentBodyEl.innerHTML = '<div class="empty">No pages available for this workspace.</div>';
          }
        } catch (error) {
          pageListEl.innerHTML = '';
          renderError(error instanceof Error ? error.message : String(error));
        }
      }

      let searchTimer = null;
      searchEl.addEventListener('input', () => {
        clearTimeout(searchTimer);
        searchTimer = setTimeout(() => {
          loadView(searchEl.value || '');
        }, 180);
      });

      loadView();
    </script>
  </body>
</html>`;
}

const server = http.createServer((req, res) => {
  const url = new URL(req.url || '/', `http://${host}:${port}`)
  if (url.pathname === '/' || url.pathname === '/wikis') {
    const body = Buffer.from(renderShell(), 'utf8')
    res.writeHead(200, {
      'Content-Type': 'text/html; charset=utf-8',
      'Content-Length': String(body.length),
    })
    res.end(body)
    return
  }
  if (url.pathname === '/health') {
    const body = Buffer.from(JSON.stringify({ ok: true, service: 'workspace-wiki-web' }), 'utf8')
    res.writeHead(200, {
      'Content-Type': 'application/json; charset=utf-8',
      'Content-Length': String(body.length),
    })
    res.end(body)
    return
  }
  res.writeHead(404, { 'Content-Type': 'text/plain; charset=utf-8' })
  res.end('not found')
})

server.listen(port, host, () => {
  console.log(`[workspace-wiki-web] listening on http://${host}:${port}`)
})
