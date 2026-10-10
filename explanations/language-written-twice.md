# Language written twice

`language` in `statac.yaml` lists the same language twice. Capitals don't
make a different language, so `en` and `EN` are one. The message points at
the second, and shows where the first was written.

## What you'll see {#what-youll-see}

```text
error  Language written twice: `en`
  More: https://statac.dev/e/language-written-twice/
```

## Why Statac stops

The first language in the list is the site's main language, and each
language's pages live in a folder named after it. With one language
written twice, Statac can't know which spelling names its folder, or
whether the second was meant to be another language altogether.

## How to fix it

Keep the language once, spelled as its folder is, and remove the other.

## Example

In `statac.yaml`:

```yaml
language: [en, ar, EN]
```

Each language once:

```yaml
language: [en, ar]
```
