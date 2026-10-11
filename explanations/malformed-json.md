# Malformed JSON

Statac couldn't read a JSON file in `data/`. The message says what it found
and points at the place to fix, such as a quote that is never closed or a
comma with nothing after it.

## What you'll see {#what-youll-see}

```text
error  Malformed JSON: a comma with nothing after it
  Take the comma out.
  More: https://statac.dev/e/malformed-json/
```

## Why Statac stops

After the mistake, Statac can't tell which values you meant. Templates
that use the file would get the wrong values, or none, and nobody would
notice.

## How to fix it

Look at the place the caret points to. JSON is strict. Common causes
are:

- A quote or bracket opened and never closed.
- A comma after the last value in a list or an object. JSON doesn't allow one.
- A name or text in single quotes, or with no quotes. JSON needs double
  quotes around both.
- A backslash sequence JSON doesn't recognise, such as `\q`. Write `\\`
  for a backslash.
- A line break or a tab typed directly inside quotes. Write `\n` or `\t`
  instead.
- A number written in a form JSON doesn't accept, such as `01`, `.5` or `+1`. Write `1`,
  `0.5` and `1`.

## Example

`data/people.json`:

```json
[
  {"name": "Person 1"},
  {"name": "Person 2"},
]
```

The comma after the last item has nothing after it. Take it out:

```json
[
  {"name": "Person 1"},
  {"name": "Person 2"}
]
```
