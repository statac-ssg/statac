# Language written twice

`language` in `statac.yaml` lists the same language twice. Capital letters
don't make it a different language, so `en` and `EN` are the same. The
message points at the second one, and shows where the first is written.

## What you'll see {#what-youll-see}

```text
error  Language written twice: `en` and `EN` are one language
  statac.yaml, line 1
   1 │ language: [en, ar, EN]
     │                    ^^
  First written: statac.yaml, line 1
   1 │ language: [en, ar, EN]
     │            ^^
  More: https://statac.dev/e/language-written-twice/
```

## Why Statac stops

The first language in the list is the site's main language, and each
language's pages are in a folder named after that language. If a language
is written twice, Statac can't tell which spelling is the folder's name, or
whether you meant the second to be a different language.

## How to fix it

Keep one of them, spelled the same way as the language's folder, and
remove the other.

## Example

In `statac.yaml`:

```yaml
language: [en, ar, EN]
```

With each language listed once:

```yaml
language: [en, ar]
```
