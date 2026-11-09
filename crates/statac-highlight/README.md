# statac-highlight

Code coloured by the kind of each piece, a comment, a keyword, a string
and so on, as [Statac](https://statac.dev), a static site generator,
colours it. It carries the TextMate grammars of npm's `tm-grammars`
whose licences let them ship inside it, so nothing is read from disk or
fetched.

Code comes two ways from one reading:

- as HTML, the inside of a `<code>`, each piece in a `<span>` whose one
  class names its kind, such as `hl-keyword`, for a stylesheet to colour;
- as each piece's byte range in the code with its kind, for an editor's
  own colouring.

```rust
use statac_highlight::{Kind, Language};

let Language::Known(rust) = statac_highlight::language("rust") else {
    unreachable!()
};
let html = statac_highlight::html(rust, "let x = 1;").unwrap();
assert!(html.starts_with("<span class=\"hl-keyword\">let</span>"));

let pieces = statac_highlight::pieces(rust, "let x = 1;").unwrap();
assert_eq!(pieces[0].range, 0..3);
assert_eq!(pieces[0].kind, Kind::Keyword);
```

The nineteen kinds, and so the classes, keep their names from one release
to the next.

Statac's source and issues are at
[git.statac.dev/statac/statac](https://git.statac.dev/statac/statac).

## Licence

The crate's own code is offered under the
[MIT licence](https://git.statac.dev/statac/statac/src/branch/main/LICENSE-MIT)
or the
[Apache License 2.0](https://git.statac.dev/statac/statac/src/branch/main/LICENSE-APACHE),
at your choice. The grammars it carries keep their own licences:
`THIRD-PARTY-GRAMMARS.md`, beside this file, lists each with its source
and the notice it asks for.
