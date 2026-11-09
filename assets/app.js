// hourglazs — progressive enhancement for the static pages.
// Everything here is optional: pages are fully readable without JavaScript.
(() => {
  const root = document.documentElement;
  const THEME_KEY = 'hourglazs.theme';
  const store = {
    get: k => { try { return localStorage.getItem(k); } catch (e) { return null; } },
    set: (k, v) => { try { localStorage.setItem(k, v); } catch (e) { /* private mode */ } },
  };

  /* ---- theme ---- */
  const themeBtn = document.querySelector('[data-theme-toggle]');
  // The public page follows the system setting until the visitor picks one.
  const systemLight = () => window.matchMedia && matchMedia('(prefers-color-scheme: light)').matches;
  const current = () => root.dataset.theme || (systemLight() ? 'light' : 'dark');
  const syncTheme = () => {
    if (!themeBtn) return;
    const dark = current() === 'dark';
    const icon = themeBtn.querySelector('[data-theme-icon]');
    const label = themeBtn.querySelector('[data-theme-label]');
    if (icon) icon.textContent = dark ? '◐' : '◑';
    if (label) label.textContent = dark ? 'Dark theme' : 'Light theme';
    if (!label) themeBtn.setAttribute('aria-label', dark ? 'Switch to light theme' : 'Switch to dark theme');
  };
  const toggleTheme = () => {
    root.dataset.theme = current() === 'light' ? 'dark' : 'light';
    store.set(THEME_KEY, root.dataset.theme);
    syncTheme();
  };
  if (themeBtn) themeBtn.addEventListener('click', toggleTheme);
  syncTheme();

  /* ---- Board/Tasks/Gantt follow the last project you opened ---- */
  const PROJECT_KEY = 'hourglazs.project';
  const here = document.body.dataset.project;
  if (here) store.set(PROJECT_KEY, here);
  const last = store.get(PROJECT_KEY);
  if (!here && last && document.querySelector('[data-project-slug="' + CSS.escape(last) + '"]')) {
    document.querySelectorAll('[data-project-view]').forEach(a => {
      a.setAttribute('href', a.getAttribute('href').replace(/p\/[^/]+\//, 'p/' + last + '/'));
    });
  }

  /* ---- greeting by the reader's local time ---- */
  const greet = document.querySelector('[data-greeting]');
  if (greet) {
    const h = new Date().getHours();
    const part = h < 5 ? 'Good night' : h < 12 ? 'Good morning' : h < 18 ? 'Good afternoon' : 'Good evening';
    greet.textContent = part + ', ' + greet.dataset.greeting + '.';
  }

  /* ---- relative timestamps ("3h ago") ---- */
  const rel = ms => {
    const s = (Date.now() - ms) / 1000;
    if (s < 0 || s > 7 * 86400) return null;
    if (s < 60) return 'just now';
    if (s < 3600) return Math.floor(s / 60) + 'm ago';
    if (s < 86400) return Math.floor(s / 3600) + 'h ago';
    return Math.floor(s / 86400) + 'd ago';
  };
  document.querySelectorAll('time[data-rel]').forEach(t => {
    const r = rel(Date.parse(t.getAttribute('datetime')));
    if (r) { t.title = t.textContent; t.textContent = r; }
  });

  /* ---- filtering + tree collapse ---- */
  const items = Array.from(document.querySelectorAll('[data-item]'));
  const search = document.querySelector('[data-filter]');
  const toggles = Array.from(document.querySelectorAll('[data-toggle]'));
  // Collapsed branches survive reloads (the local app reloads after each edit).
  const FOLD_KEY = 'hourglazs.collapsed:' + location.pathname;
  let saved = [];
  try { saved = JSON.parse(sessionStorage.getItem(FOLD_KEY) || '[]'); } catch (e) { /* ignore */ }
  const state = { q: '', pri: 'all', collapsed: new Set(saved) };

  const apply = () => {
    try { sessionStorage.setItem(FOLD_KEY, JSON.stringify([...state.collapsed])); } catch (e) { /* ignore */ }
    const q = state.q.trim().toLowerCase();
    const filtering = Boolean(q) || state.pri !== 'all';
    document.body.classList.toggle('is-filtering', filtering);
    items.forEach(el => {
      const okQ = !q || (el.dataset.text || '').includes(q);
      const okP = state.pri === 'all' || el.dataset.pri === state.pri;
      const ancestors = (el.dataset.ancestors || '').split(' ').filter(Boolean);
      const folded = !filtering && ancestors.some(a => state.collapsed.has(a));
      el.hidden = !(okQ && okP) || folded;
    });
    toggles.forEach(b => {
      const open = !state.collapsed.has(b.dataset.toggle);
      b.setAttribute('aria-expanded', String(open));
      b.setAttribute('aria-label', (open ? 'Collapse' : 'Expand') + b.getAttribute('aria-label').replace(/^(Collapse|Expand)/, ''));
    });
    document.querySelectorAll('[data-count-scope]').forEach(scope => {
      const c = scope.querySelector('[data-count]');
      if (c) c.textContent = scope.querySelectorAll('[data-item]:not([hidden])').length;
    });
  };

  if (search) search.addEventListener('input', () => { state.q = search.value; apply(); });
  document.querySelectorAll('[data-pri-btn]').forEach(btn => btn.addEventListener('click', () => {
    state.pri = btn.dataset.priBtn;
    document.querySelectorAll('[data-pri-btn]').forEach(b => b.setAttribute('aria-pressed', String(b === btn)));
    apply();
  }));
  toggles.forEach(b => b.addEventListener('click', () => {
    const id = b.dataset.toggle;
    if (state.collapsed.has(id)) state.collapsed.delete(id); else state.collapsed.add(id);
    apply();
  }));
  document.querySelectorAll('[data-expand-all]').forEach(b => b.addEventListener('click', () => { state.collapsed.clear(); apply(); }));
  document.querySelectorAll('[data-collapse-all]').forEach(b => b.addEventListener('click', () => {
    toggles.forEach(t => state.collapsed.add(t.dataset.toggle));
    apply();
  }));
  if (state.collapsed.size) apply();

  /* ---- keyboard ---- */
  document.addEventListener('keydown', e => {
    const tag = (e.target.tagName || '').toLowerCase();
    if (e.key === 'Escape') {
      if (tag === 'input') { e.target.blur(); return; }
      const esc = document.querySelector('[data-esc]');
      if (esc) location.href = esc.href;
      return;
    }
    if (tag === 'input' || tag === 'textarea' || tag === 'select' || e.metaKey || e.ctrlKey || e.altKey) return;
    if (e.key === '/' && search) { e.preventDefault(); search.focus(); return; }
    if (e.key === 't' || e.key === 'T') { toggleTheme(); return; }
    const link = document.querySelector('[data-key="' + CSS.escape(e.key) + '"]');
    if (link) { e.preventDefault(); if (link.href) location.href = link.href; else link.click(); }
  });
})();
