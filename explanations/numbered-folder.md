# Numbered folder

Every page in a folder opens its name with a number and a hyphen, such as
`01-day-one.md`, `02-day-two.md` and `03-day-three.md`, so Statac reads the
folder as numbered.

## What you'll see {#what-youll-see}

```text
note  Numbered folder: `content/days/` is ordered by the number each name opens with
  If the numbers are part of the names, write `numbered: false` in `content/days/index.md`.
  More: https://statac.dev/e/numbered-folder/
```

## Why Statac says so

In a numbered folder the numbers give the order: the folder's list holds
every page in it, dated or not, first number first. The numbers are left
out of the addresses, so `03-day-three.md` is published at `/day-three/`,
and renumbering a page never moves it. The first number is always the
order, so `07-50-notes.md` is published at `/50-notes/`.

A page in the folder that has a date is still a post everywhere else: the
site's list, its tags and its series keep their date order.

Statac says so once for each folder, since a folder of names that happen to
start with numbers would otherwise be reordered and readdressed without a
word. Once you have understood it, it can be quietened.

## How to fix it

If the numbers are meant to order the folder, nothing is wrong.

If the numbers are part of the names, as in a folder of years, write this
in the folder's `index.md`, and the names and addresses keep them:

```yaml
numbered: false
```

A folder where some pages open with a number and some don't is not
numbered.

## Example

`content/days/` holds `01-day-one.md`, `02-day-two.md` and
`03-day-three.md`. They are listed in that order, at `/days/day-one/`,
`/days/day-two/` and `/days/day-three/`.

To order a folder without renaming its files, write the order in its
`index.md` instead, by the names in the pages' addresses:

```yaml
order: [day-one, day-two, day-three]
```
