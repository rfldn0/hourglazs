// hourglazs — editing for the local app (`hourglazs serve` only).
// Every change is one POST to /api/<op>; the server validates, saves the JSON
// file and the page reloads to show the result.
(() => {
  const body = document.body;
  if (!body.hasAttribute('data-editable')) return;

  const STATUS = ['backlog', 'todo', 'doing', 'done'];
  const FOCUS_KEY = 'hourglazs.focus';

  /* ---- plumbing ---- */
  const toastEl = document.querySelector('[data-toast]');
  let toastTimer;
  const toast = msg => {
    if (!toastEl) return alert(msg);
    toastEl.textContent = msg;
    toastEl.classList.add('show');
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => toastEl.classList.remove('show'), 4000);
  };

  const api = async (op, payload) => {
    const res = await fetch('/api/' + op, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload),
    });
    const data = await res.json().catch(() => ({}));
    if (!res.ok) throw new Error(data.error || res.statusText);
    return data;
  };

  /** Run an edit; on success reload (or call `then`), on failure show why. */
  const run = async (op, payload, then) => {
    body.classList.add('saving');
    try {
      const res = await api(op, payload);
      if (then) then(res); else location.reload();
    } catch (e) {
      body.classList.remove('saving');
      toast(e.message);
    }
  };

  /** Which project/task an element belongs to, from itself or an ancestor. */
  const target = el => ({
    project: el.dataset.project || el.closest('[data-project]')?.dataset.project || body.dataset.project,
    id: el.dataset.id || el.closest('[data-id]')?.dataset.id,
  });

  const update = (el, patch) => {
    const { project, id } = target(el);
    run('update', { project, id, patch });
  };

  /* ---- buttons ---- */
  document.addEventListener('click', e => {
    const el = e.target.closest('[data-action]');
    if (!el) return;
    e.preventDefault();
    e.stopPropagation();
    const d = el.dataset;
    switch (d.action) {
      case 'set':
        update(el, { [d.field]: d.value });
        break;
      case 'clear-dates':
        update(el, { start: null, due: null, days: null });
        break;
      case 'new-task': {
        const project = body.dataset.project;
        run('create', { project, parent: d.parent || null, title: 'Untitled task' },
          res => { location.href = d.taskBase + res.id + '.html'; });
        break;
      }
      case 'delete': {
        if (!confirm('Delete "' + d.title + '" and all its subtasks?')) return;
        const { project, id } = target(el);
        run('delete', { project, id }, () => { location.href = d.next; });
        break;
      }
    }
  });

  /* ---- fields on the task page ---- */
  document.addEventListener('change', e => {
    const el = e.target.closest('[data-field]');
    if (!el || el.tagName === 'BUTTON') return;
    const field = el.dataset.field;
    let value = el.type === 'checkbox' ? el.checked : el.value;

    if (field === 'title') {
      value = value.replace(/\s+/g, ' ').trim();
      if (!value) { toast('Title can\'t be empty'); return; }
    }
    if (field === 'hours') value = value === '' ? null : Number(value);
    if (field === 'tags') value = value.split(',').map(s => s.trim()).filter(Boolean);
    if (field === 'start' || field === 'due') {
      const scope = el.closest('.props') || document;
      const start = scope.querySelector('[data-field="start"]');
      const due = scope.querySelector('[data-field="due"]');
      return update(el, { start: start && start.value ? start.value : null, due: due && due.value ? due.value : null, days: null });
    }
    update(el, { [field]: value });
  });

  // Title: Enter saves instead of adding a newline; the box grows with its text.
  document.querySelectorAll('.title-input').forEach(t => {
    const fit = () => { t.style.height = 'auto'; t.style.height = t.scrollHeight + 'px'; };
    fit();
    t.addEventListener('input', fit);
    t.addEventListener('keydown', e => { if (e.key === 'Enter') { e.preventDefault(); t.blur(); } });
  });

  /* ---- create forms: quick add, board columns, subtasks ---- */
  // "Ship it !high #release" → title "Ship it", priority high, tag release.
  const parseQuick = text => {
    let priority = null;
    const tags = [];
    const title = text
      .replace(/(^|\s)!(high|h|med|medium|m|low|l)\b/gi, (_, sp, p) => {
        p = p.toLowerCase();
        priority = p[0] === 'h' ? 'high' : p[0] === 'l' ? 'low' : 'med';
        return sp;
      })
      .replace(/(^|\s)#([\w-]+)/g, (_, sp, t) => { tags.push(t); return sp; })
      .replace(/\s+/g, ' ')
      .trim();
    return { title, priority, tags };
  };

  document.querySelectorAll('form[data-create]').forEach(form => {
    form.addEventListener('submit', e => {
      e.preventDefault();
      const input = form.querySelector('[name="title"]');
      const q = parseQuick(input.value);
      if (!q.title) return;
      const { project } = target(form);
      const payload = { project, title: q.title, tags: q.tags };
      if (q.priority) payload.priority = q.priority;
      if (form.dataset.parent) payload.parent = form.dataset.parent;
      if (form.dataset.status) payload.status = form.dataset.status;
      sessionStorage.setItem(FOCUS_KEY, 'create:' + (form.dataset.status || form.dataset.parent || 'quick'));
      run('create', payload);
    });
  });

  /* ---- notes ---- */
  document.querySelectorAll('form[data-comment]').forEach(form => {
    const text = form.querySelector('[name="text"]');
    const submit = () => {
      if (!text.value.trim()) return;
      const { project, id } = target(form);
      run('comment', { project, task: id || null, text: text.value });
    };
    form.addEventListener('submit', e => { e.preventDefault(); submit(); });
    text.addEventListener('keydown', e => {
      if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) { e.preventDefault(); submit(); }
    });
  });

  /* ---- board: drag and drop, Shift+arrows ---- */
  const moveCard = (card, status) => {
    if (!status || card.dataset.status === status) return;
    sessionStorage.setItem(FOCUS_KEY, 'card:' + card.dataset.id);
    update(card, { status });
  };

  document.querySelectorAll('.card[draggable="true"]').forEach(card => {
    card.addEventListener('dragstart', e => {
      e.dataTransfer.effectAllowed = 'move';
      e.dataTransfer.setData('text/plain', card.dataset.id);
      card.classList.add('dragging');
    });
    card.addEventListener('dragend', () => card.classList.remove('dragging'));
    card.addEventListener('keydown', e => {
      if (!e.shiftKey || (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight')) return;
      e.preventDefault();
      const i = STATUS.indexOf(card.dataset.status) + (e.key === 'ArrowRight' ? 1 : -1);
      moveCard(card, STATUS[i]);
    });
  });

  document.querySelectorAll('[data-drop-status]').forEach(col => {
    col.addEventListener('dragover', e => { e.preventDefault(); col.classList.add('over'); });
    col.addEventListener('dragleave', e => { if (!col.contains(e.relatedTarget)) col.classList.remove('over'); });
    col.addEventListener('drop', e => {
      e.preventDefault();
      col.classList.remove('over');
      const id = e.dataTransfer.getData('text/plain');
      const card = document.querySelector('.card[data-id="' + CSS.escape(id) + '"]');
      if (card) moveCard(card, col.dataset.dropStatus);
    });
  });

  /* ---- put focus back where you were after the reload ---- */
  const focus = sessionStorage.getItem(FOCUS_KEY);
  if (focus) {
    sessionStorage.removeItem(FOCUS_KEY);
    const [kind, key] = [focus.slice(0, focus.indexOf(':')), focus.slice(focus.indexOf(':') + 1)];
    const el = kind === 'card'
      ? document.querySelector('.card[data-id="' + CSS.escape(key) + '"] .card-title')
      : document.querySelector(
          'form[data-create][data-status="' + CSS.escape(key) + '"] input, ' +
          'form[data-create][data-parent="' + CSS.escape(key) + '"]:not([data-status]) input, ' +
          (key === 'quick' ? '.quick-add input' : '_none_'));
    if (el) el.focus();
  }
})();
