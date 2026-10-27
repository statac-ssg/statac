# Worker outside the import map

A script in `assets/` starts a worker, or loads a script into a worker,
using the name of another file in `assets/`. It does this with
`new Worker("worker.js")`, `new SharedWorker(...)` or `importScripts(...)`.

## What you'll see {#what-youll-see}

```text
warning  Worker outside the import map: `./worker.js` isn't published under that name
  Keep the worker and the files it imports in `static/`: Statac publishes those under their own names.
  More: https://statac.dev/e/worker-outside-import-map/
```

## Why Statac warns

Statac publishes each file in `assets/` under a new name made from its
contents. If `js()` places a module on a page, or one of the page's scripts imports
a module as it runs, Statac adds an import map to the page. The map lets
modules still import each other by their own names. Browsers read that
map only on the page, never inside a worker. So the file name the script
uses to start the worker isn't published, and neither are the names the
worker imports. Statac publishes the script as written and warns you,
because the worker won't start.

## How to fix it

Move the worker, and every file it imports, to `static/`. Statac
publishes the files there under their own names, so the names in the
scripts lead to them.

## Example

In `assets/app.js`:

```js
const worker = new Worker("./worker.js", { type: "module" });
```

Move `assets/worker.js` to `static/worker.js`, along with the files it
imports, and give it a URL starting with `/`:

```js
const worker = new Worker("/worker.js", { type: "module" });
```
