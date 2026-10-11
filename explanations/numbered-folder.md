# Numbered folder

Every page in a folder has a name that starts with a number and a
hyphen, such as `01-day-one.md`, `02-day-two.md` and `03-day-three.md`,
so Statac treats the folder as numbered.

## What you'll see {#what-youll-see}

```text
note  Numbered folder: `content/days/` is ordered by the number each name opens with
  If the numbers are part of the names, write `numbered: false` in `content/days/index.md`.
  More: https://statac.dev/e/numbered-folder/
```

## Why Statac says so

In a numbered folder, the numbers set the order. The folder's list
includes every page in it, dated or not, in order of their numbers. The
numbers are left out of the addresses, so `03-day-three.md` is published
at `/day-three/`, and renumbering a page never changes its address. Only
the first number sets the order, so `07-50-notes.md` is published at
`/50-notes/`.

A page in the folder that has a date is still a post everywhere else.
The site's list, its tags and its series still sort it by date.

Statac tells you once for each folder, because otherwise a folder whose
names happen to start with numbers would be reordered and given new
addresses without a word. Once you have understood it, you can
quieten it.

## How to fix it

If you meant the numbers to order the folder, nothing is wrong.

If the numbers are part of the names, as in a folder of years, write
this in the folder's `index.md`. The names and addresses then keep the
numbers:

```yaml
numbered: false
```

A folder where only some of the pages start with a number is not
numbered.

## Example

`content/days/` contains `01-day-one.md`, `02-day-two.md` and
`03-day-three.md`. They are listed in that order, at `/days/day-one/`,
`/days/day-two/` and `/days/day-three/`.

To order a folder without renaming its files, write the order in its
`index.md` instead, using the names in the pages' addresses:

```yaml
order: [day-one, day-two, day-three]
```
