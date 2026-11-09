# hourglazs

A personal project tracker in Rust, in two parts:

- **Locally**, `hourglazs serve` runs the full app with editing: board, task tree, Gantt,
  timeline and task pages. Every change is saved straight into the JSON files in `data/`.
- **Publicly**, `hourglazs build` renders a single static progress page (momentum, overall
  progress, what's in progress, milestones, recently completed work) that GitHub Pages hosts.
  It uses the same look as the app.

```
you ──edit──► hourglazs serve ──saves──► data/*.json ──git push──► GitHub Actions ──build──► public page
                                                     (automatic with --publish)
```

## Run it

```sh
cargo run -- serve                 # full app at http://127.0.0.1:4000/
cargo run -- serve --publish       # same, and commit + push data/ after you stop editing for 60s
cargo run -- build                 # public page into dist/  (open dist/index.html)
cargo run -- check                 # validate data/ only
```

While `serve` is running, the public page preview is at http://127.0.0.1:4000/public/.

### Editing in the app

| Where | What |
|---|---|
| Anywhere | Click a ○ checkbox to mark done / not done |
| Board | Drag cards between columns, or focus a card and press Shift + ←/→. "＋ Add" at the bottom of each column |
| Tasks | Quick add: `Write docs !high #release` sets the priority and tags |
| Task page | Edit the title, status, priority, dates, estimate, tags, milestone and description; add subtasks and notes (Ctrl+Enter); delete |
| Header | "＋ New task" (or `N`) creates a task and opens it |

Shortcuts: `1` Home, `2` Timeline, `3` Board, `4` Tasks, `5` Gantt, `/` filter, `T` theme,
`N` new task, `Esc` close a task.

The server re-reads the files on every request, so edits you make in your editor show up
when you refresh. If a file has a problem the app shows the errors instead of guessing.

### What's public

The public page shows task titles, progress, milestones and completion dates. It never
shows descriptions, notes or the activity log. To keep a whole project off it, add
`"public": false` to that project's file.

### Momentum

Any day with an action counts as active: a completed task, a status change, a new task or
a note. From those days the public page shows your current and best streak, actions this
week against last week, a year-long activity grid, and a "Quiet Nd" tag on any project
untouched for 7+ days. The local Home page shows the same streak with a nudge when today
has nothing logged yet. The page is rebuilt on every publish and once a day, so a quiet
streak shows up publicly even if you don't push.

### Background publishing

`serve --publish` waits until you've stopped editing for `--quiet` seconds (default 60),
then commits everything changed under `data/` as one commit (listing each edit) and runs
`git push`. It only ever commits `data/`, so other work you have staged is left alone.
Without a git remote it commits but doesn't push. If you stop the server before a batch
goes out, the files are already saved; they're included with the next publish.

## Deploying the public page

1. Push this repository to GitHub.
2. In *Settings → Pages*, set *Source* to **GitHub Actions**.

`.github/workflows/pages.yml` then rebuilds the page on every push to `main`, and once a
day so "today" and overdue flags stay current.

## Data

```
data/
  hourglazs.json        site config: title, owner name, project order
  projects/<slug>.json  one file per project
schema/                 JSON Schemas (editor autocomplete + validation)
```

```jsonc
{
  "$schema": "../../schema/project.schema.json",
  "name": "hourglazs",
  "key": "HG",                      // new task IDs become HG-<n>
  "color": "red",                   // red | amber | green | violet | cyan
  "public": true,                   // show on the public page (default)
  "tasks": [
    {
      "id": "HG-4", "title": "Static generator", "status": "doing", "priority": "high",
      "subtasks": [
        { "id": "HG-5", "title": "Data model", "status": "done", "start": "2026-11-07", "days": 1, "hours": 3, "done_on": "2026-11-07" }
      ]
    },
    { "id": "HG-16", "title": "v0.1 live", "milestone": true, "due": "2026-11-10" }
  ],
  "log": [
    { "at": "2026-11-07T13:10", "task": "HG-4", "text": "A note" },
    { "at": "2026-11-07T15:40", "task": "HG-5", "kind": "event", "text": "completed Data model" }
  ]
}
```

- `status`: backlog | todo | doing | done. `priority`: high | med | low.
- Dates: give any two of `start`, `due`, `days` (or a single date). A task without dates
  spans its subtasks.
- `done_on` is filled in by the app when you mark a task done; it feeds "Recently completed".
- To add a project, create `data/projects/<slug>.json` and list `<slug>` under `"projects"`
  in `data/hourglazs.json`.

## Code layout

```
src/
  main.rs, cli.rs     commands and flags
  model/              serde types mirroring the JSON (read and write)
  data/               load, validate, save; format.rs keeps written JSON hand-editable
  domain/             task tree index, progress, date spans, the Site aggregate
  edit/               edits as pure functions over a project (update/create/delete/note)
  render/
    routes.rs         every URL + relative-link context
    layout.rs         app shell and public-page shell
    components.rs     shared fragments
    views/            one module per page (public.rs is the GitHub page)
    assets.rs         CSS/JS/icon embedded in the binary
  serve/
    mod.rs            localhost HTTP server, page routing
    api.rs            POST /api/<op>: edit → validate → save
    publish.rs        batched git commit + push
assets/               style.css, app.js (all pages), edit.js (local app only), hourglass.svg
```
