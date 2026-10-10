# Worker outside the import map

A script in `assets/` starts a worker, or loads a script into one, by the
name of another file in `assets/`: `new Worker("worker.js")`,
`new SharedWorker(...)` or `importScripts(...)`.

## What you'll see {#what-youll-see}

```text
warning  Worker outside the import map: `./worker.js` isn't published under that name
  Keep the worker and the files it imports in `static/`, which publishes them under their own names.
  More: https://statac.dev/e/worker-outside-import-map/
```

## Why Statac warns

Statac publishes each file in `assets/` under a new name made from its
contents. A page is given an import map where `js()` names a module, or
where one of its scripts imports a module as it runs, so modules can still
import each other by their own names. Browsers read that map only on the
page, never inside a worker. So the name the script gives the worker isn't
published, and neither are the names the worker imports by. Statac publishes
the script as written and tells you, since the worker won't start.

## How to fix it

Move the worker, and every file it imports, to `static/`. Files there are
published under their own names, so the names in the scripts reach them.

## Example

In `assets/app.js`:

```js
const worker = new Worker("./worker.js", { type: "module" });
```

Move `assets/worker.js` to `static/worker.js`, with the files it imports,
and name it from the top of the site:

```js
const worker = new Worker("/worker.js", { type: "module" });
```
