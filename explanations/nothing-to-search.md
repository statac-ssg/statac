# Nothing to search for

`statac search` was given no words. It shows the pages a reader of the
site would find for some words, so it needs the words.

## What you'll see {#what-youll-see}

```text
error  Nothing to search for: `statac search` needs the words to look for
  Give them after the command, such as `statac search "raspberry pi"`.
  More: https://statac.dev/e/nothing-to-search/
```

## Why Statac stops

Without words, there is nothing to look for.

## How to fix it

Give the words after `statac search`. Put them in quotes when there are
several, so they arrive as one search.

To search a site in another folder, give the words first and the folder
after them: `statac search "raspberry pi" ~/sites/blog`. Every other
command takes the folder straight after its name, so `statac search
~/sites/blog` reads naturally, but there the folder is the words, and
Statac says so.

## Example

```text
statac search
```

To search the site for pages about a Raspberry Pi:

```text
statac search "raspberry pi"
```
