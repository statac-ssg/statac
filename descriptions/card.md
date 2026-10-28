# A social card in a template

Summary: Every value a template can read on a page's social card or social image.

What a layout reads on `page.card`: the card Statac draws for the page,
or the social image its `og_image` names.

## `url`

The URL of the card's image, starting with `/`. If the site's `url` has
a path, such as `https://example.org/blog/`, the URL starts with that
path, `/blog/`. A
card Statac draws is published beside the page, and its file name
changes whenever the card does.

More: https://statac.dev/social-cards/#in-a-layout

## `width`

The card's width in pixels: 1200 for a card Statac draws. Empty for a
social image whose size Statac doesn't know.

More: https://statac.dev/social-cards/#in-a-layout

## `height`

The card's height in pixels: 630 for a card Statac draws. Empty for a
social image whose size Statac doesn't know.

More: https://statac.dev/social-cards/#in-a-layout

## `alt`

The card's alt text. For a card Statac draws, it is the words on the
card. For a social image, it is the image's own alt text, or if it has
none, the page's title.

More: https://statac.dev/social-cards/#in-a-layout
