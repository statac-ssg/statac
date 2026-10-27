# A date in a template

What a template reads on a date, such as `page.date.year`.
`{{ page.date }}` on its own shows the date in the site's date format.
A custom field declared `when` can be a year alone, a year and a month,
or a whole date.

## `year`

The date's year, as a number.

More: https://statac.dev/dates/#how-dates-look

## `month`

The date's month, as a number from 1 to 12. A date written as a year
alone has no month.

More: https://statac.dev/dates/#how-dates-look

## `day`

The date's day of the month, as a number. A date written without a day
has no day.

More: https://statac.dev/dates/#how-dates-look

## `weekday`

The name of the date's day of the week, in the page's language. A date
written without a day has no weekday.

More: https://statac.dev/dates/#how-dates-look
