# Security

## Reporting a problem

Don't open an issue for a security problem. Report it privately by
email to [security@statac.dev](mailto:security@statac.dev). Only the
people who look after Statac see the report.

Say what you ran, what Statac did and what you expected, with the version
`statac --version` gives. The smallest site that shows the problem helps
most.

## What is a security problem

Statac reads files its user may not have written: a site being imported,
an image, a template, what it fetches from another site. It is a
security problem where such a file makes Statac read or write outside
the folders it was given, run a command the site's settings don't name,
or send anything anywhere the site didn't ask for.

`statac serve` answers this computer alone. It is a security problem
where another machine, or a page from another site open in the same
browser, gets an answer from it.

## Which versions

A fix goes into the next release. Earlier releases are not changed.
