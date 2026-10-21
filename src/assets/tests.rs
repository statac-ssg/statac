use super::*;
use crate::problem::SassFault;

/// A site folder in the system's temporary folder, removed when dropped.
struct Site(PathBuf);

impl Site {
    fn new(name: &str, files: &[(&str, &str)]) -> Site {
        let root =
            std::env::temp_dir().join(format!("statac-assets-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let site = Site(root);
        for (path, text) in files {
            site.write(path, text);
        }
        site
    }

    fn write(&self, path: &str, text: &str) {
        let path = platform::join(&self.0, path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn gather(&self, base: &str) -> (Assets, Vec<Problem>, Sources) {
        let mut sources = Sources::default();
        let (assets, problems) = gather(&self.0, base, None, &mut sources);
        (assets, problems, sources)
    }
}

impl Drop for Site {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn text(asset: &Asset) -> &str {
    std::str::from_utf8(asset.bytes()).unwrap()
}

fn mark_of(published: &str) -> &str {
    published.rsplit('.').nth(1).unwrap()
}

#[test]
fn no_code_stylesheet_is_published_where_code_style_is_none() {
    let site = Site::new(
        "code-css-off",
        &[("assets/site.css", "body { margin: 0; }\n")],
    );
    let (assets, problems) = gather(&site.0, "", Some("none"), &mut Sources::default());
    assert_eq!(problems, []);
    assert!(assets.get("code.css").is_none());
}

#[test]
fn the_code_stylesheet_is_written_from_the_style_and_published_like_any_other() {
    let site = Site::new("code-css", &[("assets/site.css", "body { margin: 0; }\n")]);
    let gathered = || gather(&site.0, "", Some("one-half"), &mut Sources::default());
    let (assets, problems) = gathered();
    assert_eq!(problems, []);
    let code = assets.get("code.css").unwrap();
    assert_eq!(code.source, "statac.yaml");
    assert_eq!(code.kind, AssetKind::Stylesheet);
    assert_eq!(
        text(code),
        crate::highlight::stylesheet("one-half").unwrap()
    );
    assert_eq!(
        code.published,
        format!("code.{}.css", fingerprint::mark(code.bytes()))
    );
    assert_eq!(
        assets.css("code.css", &mut Reads::default()).unwrap(),
        format!("<link rel=\"stylesheet\" href=\"/{}\">", code.published)
    );

    // A `code.css` of the site's own is the one published.
    site.write("assets/code.css", ".hl-comment { color: grey; }\n");
    let (assets, problems) = gathered();
    assert_eq!(problems, []);
    let code = assets.get("code.css").unwrap();
    assert_eq!(code.source, "assets/code.css");
    assert_eq!(text(code), ".hl-comment { color: grey; }\n");
}

#[test]
fn sass_compiles_and_its_partials_are_not_published() {
    let site = Site::new(
        "sass",
        &[
            (
                "assets/site.scss",
                "@use 'colours';\nbody { color: colours.$ink; }\n",
            ),
            ("assets/_colours.scss", "$ink: #333;\n"),
        ],
    );
    let (assets, problems, _) = site.gather("");
    assert_eq!(problems, []);
    let site_css = assets.get("site.css").unwrap();
    assert_eq!(site_css.source, "assets/site.scss");
    assert!(text(site_css).contains("color: #333"), "{}", text(site_css));
    assert_eq!(
        site_css.published,
        format!("site.{}.css", fingerprint::mark(site_css.bytes()))
    );
    assert!(assets.get("_colours.css").is_none());
    assert_eq!(assets.iter().count(), 1);
}

#[test]
fn a_stylesheet_names_the_files_it_uses_as_published() {
    let site = Site::new(
        "names",
        &[
            (
                "assets/css/site.css",
                "@import 'type.css';\n\
                 @font-face { src: url(\"../fonts/a.woff2?v=2\") format('woff2'); }\n\
                 .logo { background: url(/logo.png); }\n\
                 .elsewhere { background: url(https://example.com/a.png); }\n",
            ),
            ("assets/css/type.css", "body { font-family: a; }\n"),
            ("assets/fonts/a.woff2", "font"),
            ("assets/logo.png", "picture"),
        ],
    );
    let (assets, problems, _) = site.gather("");
    assert_eq!(problems, []);
    let font = assets.get("fonts/a.woff2").unwrap();
    assert_eq!(
        font.published,
        format!("fonts/a.{}.woff2", fingerprint::mark(b"font"))
    );
    let typeset = &assets.get("css/type.css").unwrap().published;
    let logo = &assets.get("logo.png").unwrap().published;
    let stylesheet = assets.get("css/site.css").unwrap();
    assert_eq!(
        text(stylesheet),
        format!(
            "@import '{}';\n\
             @font-face {{ src: url(\"../{}?v=2\") format('woff2'); }}\n\
             .logo {{ background: url(/{logo}); }}\n\
             .elsewhere {{ background: url(https://example.com/a.png); }}\n",
            typeset.trim_start_matches("css/"),
            font.published,
        )
    );
    assert_eq!(
        mark_of(&stylesheet.published),
        fingerprint::mark(stylesheet.bytes())
    );
}

#[test]
fn a_changed_font_renames_the_stylesheet_using_it() {
    let files = [
        ("assets/site.css", "@font-face { src: url(a.woff2); }\n"),
        ("assets/a.woff2", "font"),
    ];
    let site = Site::new("renamed", &files);
    let (before, _, _) = site.gather("");
    site.write("assets/a.woff2", "another font");
    let (after, _, _) = site.gather("");
    assert_ne!(
        before.get("site.css").unwrap().published,
        after.get("site.css").unwrap().published
    );
}

#[test]
fn stylesheets_importing_each_other_share_a_fingerprint() {
    let site = Site::new(
        "ring",
        &[
            ("assets/a.css", "@import 'b.css';\n.a {}\n"),
            ("assets/b.css", "@import 'a.css';\n.b {}\n"),
        ],
    );
    let (assets, problems, _) = site.gather("");
    assert_eq!(problems, []);
    let a = assets.get("a.css").unwrap();
    let b = assets.get("b.css").unwrap();
    assert_eq!(mark_of(&a.published), mark_of(&b.published));
    assert_eq!(text(a), format!("@import '{}';\n.a {{}}\n", b.published));
    assert_eq!(text(b), format!("@import '{}';\n.b {{}}\n", a.published));
}

#[test]
fn the_template_functions_give_tags_and_addresses() {
    let site = Site::new(
        "functions",
        &[
            ("assets/site.css", "body {}\n"),
            ("assets/font.woff2", "font"),
            ("assets/menu.js", "document.body.dataset.ready = '';\n"),
        ],
    );
    let (assets, _, _) = site.gather("");
    let mut reads = Reads::default();
    let mut page = PageScripts::default();
    let css = &assets.get("site.css").unwrap().published;
    let font = &assets.get("font.woff2").unwrap().published;
    let menu = &assets.get("menu.js").unwrap().published;
    assert_eq!(
        assets.css("site.css", &mut reads).unwrap(),
        format!("<link rel=\"stylesheet\" href=\"/{css}\">")
    );
    assert_eq!(
        assets.asset("/font.woff2", &mut reads).unwrap(),
        format!("/{font}")
    );
    assert_eq!(
        assets.js("menu.js", &mut page, &mut reads).unwrap(),
        format!("<script src=\"/{menu}\" defer></script>")
    );
    assert_eq!(
        assets
            .css("https://fonts.example/a.css", &mut reads)
            .unwrap(),
        "<link rel=\"stylesheet\" href=\"https://fonts.example/a.css\">"
    );
    let ids: Vec<AssetId> = ["site.css", "font.woff2", "menu.js"]
        .iter()
        .map(|name| assets.names[*name])
        .collect();
    assert_eq!(
        reads.keys(),
        ids.iter().map(|&id| Key::Asset(id)).collect::<Vec<_>>()
    );
}

#[test]
fn the_first_module_on_a_page_brings_the_import_map() {
    let site = Site::new(
        "modules",
        &[
            (
                "assets/app.js",
                "import { greet } from './lib/greet.js';\ngreet();\n",
            ),
            ("assets/lib/greet.js", "export function greet() {}\n"),
            ("assets/menu.js", "document.title = '';\n"),
            ("assets/site.css", "body {}\n"),
        ],
    );
    let (assets, _, _) = site.gather("/blog/");
    let app = &assets.get("app.js").unwrap().published;
    let greet = &assets.get("lib/greet.js").unwrap().published;
    let menu = &assets.get("menu.js").unwrap().published;
    let mut page = PageScripts::default();
    let mut reads = Reads::default();
    assert_eq!(
        assets.js("menu.js", &mut page, &mut reads).unwrap(),
        format!("<script src=\"/blog/{menu}\" defer></script>")
    );
    assert_eq!(
        assets.js("app.js", &mut page, &mut reads).unwrap(),
        format!(
            "<script type=\"importmap\">\n{{\n  \"imports\": {{\n    \
             \"/blog/app.js\": \"/blog/{app}\",\n    \
             \"/blog/lib/greet.js\": \"/blog/{greet}\",\n    \
             \"/blog/menu.js\": \"/blog/{menu}\"\n  }}\n}}\n</script>\n\
             <script type=\"module\" src=\"/blog/{app}\"></script>"
        )
    );
    assert_eq!(
        assets.js("lib/greet.js", &mut page, &mut reads).unwrap(),
        format!("<script type=\"module\" src=\"/blog/{greet}\"></script>")
    );
    // A new page carries its own map.
    let mut next = PageScripts::default();
    assert!(
        assets
            .js("app.js", &mut next, &mut reads)
            .unwrap()
            .starts_with("<script type=\"importmap\">")
    );
}

#[test]
fn a_script_loading_modules_as_it_runs_brings_the_map_too() {
    let site = Site::new(
        "dynamic",
        &[
            (
                "assets/menu.js",
                "button.onclick = () => import('./later.js');\n",
            ),
            ("assets/later.js", "export const later = 1;\n"),
        ],
    );
    let (assets, _, _) = site.gather("");
    let mut page = PageScripts::default();
    let tags = assets
        .js("menu.js", &mut page, &mut Reads::default())
        .unwrap();
    assert!(tags.starts_with("<script type=\"importmap\">"), "{tags}");
    assert!(tags.ends_with(" defer></script>"), "{tags}");
}

#[test]
fn a_name_in_the_map_is_safe_inside_its_element() {
    let site = Site::new(
        "modules",
        &[
            ("assets/app.js", "import './a&b\\'c.js';\n"),
            ("assets/a&b'c.js", "export const a = 1;\n"),
        ],
    );
    let (assets, _, _) = site.gather("");
    let tags = assets
        .js("app.js", &mut PageScripts::default(), &mut Reads::default())
        .unwrap();
    assert!(tags.contains("\"/a\\u0026b\\u0027c.js\": "), "{tags}");
}

#[test]
fn a_missing_file_offers_the_nearest_name_or_its_place_in_static() {
    let site = Site::new(
        "missing",
        &[("assets/site.css", "body {}\n"), ("static/robots.txt", "")],
    );
    let (assets, _, _) = site.gather("");
    let mut reads = Reads::default();
    assert_eq!(
        assets.css("sit.css", &mut reads),
        Err(Kind::MissingFile {
            path: "assets/sit.css".into(),
            hint: Some(Hint::Closest("site.css".into())),
        })
    );
    assert_eq!(
        assets.css("assets/site.css", &mut reads),
        Err(Kind::MissingFile {
            path: "assets/assets/site.css".into(),
            hint: Some(Hint::Closest("site.css".into())),
        })
    );
    assert_eq!(
        assets.asset("robots.txt", &mut reads),
        Err(Kind::MissingFile {
            path: "assets/robots.txt".into(),
            hint: Some(Hint::InStatic("/robots.txt".into())),
        })
    );
    assert_eq!(
        assets.asset("print.css", &mut reads),
        Err(Kind::MissingFile {
            path: "assets/print.css".into(),
            hint: None,
        })
    );
    assert!(reads.keys().is_empty());
}

#[test]
fn what_sass_says_is_a_warning_at_the_line_that_said_it() {
    let site = Site::new(
        "warned",
        &[
            ("assets/site.scss", "@use 'grid';\nbody { color: red; }\n"),
            ("assets/print.scss", "@use 'grid';\n@debug \"print\";\n"),
            (
                "assets/_grid.scss",
                "$columns: 12;\n@warn \"$columns is deprecated\";\n",
            ),
        ],
    );
    let (assets, problems, sources) = site.gather("");
    let said: Vec<(&Kind, String, usize)> = problems
        .iter()
        .map(|problem| {
            let place = problem.place.unwrap();
            let line = sources
                .line_column(place.source, place.span.start as usize)
                .line;
            (
                &problem.kind,
                sources.path(place.source).display().to_string(),
                line,
            )
        })
        .collect();
    let file = |name: &str| Path::new("assets").join(name).display().to_string();
    assert_eq!(
        said,
        [
            (
                &Kind::SassWarning {
                    message: "$columns is deprecated".into()
                },
                file("_grid.scss"),
                2
            ),
            (
                &Kind::SassWarning {
                    message: "print".into()
                },
                file("print.scss"),
                2
            ),
        ]
    );
    assert!(
        Kind::SassWarning {
            message: "x".into()
        }
        .quietenable()
    );
    assert!(assets.get("site.css").is_some());
    assert!(assets.get("print.css").is_some());
}

#[test]
fn sass_stopping_is_placed_where_it_stopped() {
    let site = Site::new(
        "stopped",
        &[
            ("assets/site.scss", "@use 'parts';\nbody { color: red; }\n"),
            ("assets/_parts.scss", "a {\n  color: $nowhere;\n}\n"),
        ],
    );
    let (assets, problems, sources) = site.gather("");
    let [problem] = &problems[..] else {
        panic!("{problems:?}")
    };
    assert!(
        matches!(problem.kind, Kind::SassStopped { .. }),
        "{problem:?}"
    );
    let place = problem.place.unwrap();
    assert_eq!(
        sources.path(place.source),
        Path::new("assets").join("_parts.scss")
    );
    assert_eq!(sources.spelling(place), "$nowhere");
    assert_eq!(
        sources
            .line_column(place.source, place.span.start as usize)
            .line,
        2
    );
    // Sass has told it; the page asking for it isn't told again.
    assert_eq!(
        assets.css("site.css", &mut Reads::default()).unwrap(),
        "<link rel=\"stylesheet\" href=\"/site.css\">"
    );
}

/// Each way Sass stops, as Statac words it: the headline, the suggestion,
/// and the place, the line with `«»` round the text under the caret.
#[test]
fn why_sass_stopped_is_told_in_its_own_words_at_the_text_it_is_about() {
    type Case = (
        Files,
        &'static str,
        Option<&'static str>,
        &'static str,
        usize,
        &'static str,
    );
    type Files = &'static [(&'static str, &'static str)];
    const PART: &str = "$x: 1;\n";
    let cases: &[Case] = &[
        (
            &[("site.scss", "a {\n  @error \"Something is wrong.\";\n}\n")],
            "`Something is wrong.`",
            None,
            "site.scss",
            2,
            "  «@error \"Something is wrong.\"»;",
        ),
        (
            &[("site.scss", "@error Plain words;\n")],
            "`Plain words`",
            None,
            "site.scss",
            1,
            "«@error Plain words»;",
        ),
        (
            &[
                ("site.scss", "@use \"part\";\na { @include part.m; }\n"),
                (
                    "_part.scss",
                    "@mixin m {\n  @error \"#{1 + 1} is too many.\";\n}\n",
                ),
            ],
            "`2 is too many.`",
            None,
            "_part.scss",
            2,
            "  «@error \"#{1 + 1} is too many.\"»;",
        ),
        (
            &[("site.scss", "@error \"Undefined variable.\";\n")],
            "`Undefined variable.`",
            None,
            "site.scss",
            1,
            "«@error \"Undefined variable.\"»;",
        ),
        (
            &[("site.scss", "@error $nowhere;\n")],
            "no variable called `$nowhere`",
            None,
            "site.scss",
            1,
            "@error «$nowhere»;",
        ),
        (
            &[("site.scss", "a { color: $nowhere; }\n")],
            "no variable called `$nowhere`",
            None,
            "site.scss",
            1,
            "a { color: «$nowhere»; }",
        ),
        (
            &[
                ("site.scss", "@use \"part\";\na { color: part.$y; }\n"),
                ("_part.scss", PART),
            ],
            "no variable called `part.$y`",
            None,
            "site.scss",
            2,
            "a { color: «part.$y»; }",
        ),
        (
            &[("site.scss", "a { @include m; }\n")],
            "no mixin called `m`",
            None,
            "site.scss",
            1,
            "a { @include «m»; }",
        ),
        (
            &[
                ("site.scss", "@use \"part\";\na { b: part.nosuch(1); }\n"),
                ("_part.scss", PART),
            ],
            "no function called `part.nosuch`",
            None,
            "site.scss",
            2,
            "a { b: «part.nosuch»(1); }",
        ),
        (
            &[("site.scss", "a { b: get-function(nosuch); }\n")],
            "no function called `nosuch`",
            None,
            "site.scss",
            1,
            "a { b: «get-function(nosuch)»; }",
        ),
        (
            &[("site.scss", "a {\n  color: red\n  margin: 0;\n}\n")],
            "a `;` is missing",
            None,
            "site.scss",
            2,
            "  color: red«»",
        ),
        (
            &[("site.scss", "a {\n  color: red // dark\n  margin: 0;\n}\n")],
            "a `;` is missing",
            None,
            "site.scss",
            2,
            "  color: red«» // dark",
        ),
        (
            &[("site.scss", "a {\n  color: red;\n")],
            "`{` is never closed",
            None,
            "site.scss",
            1,
            "a «{»",
        ),
        (
            &[("site.scss", "a { b: #{1 + 2; }\n")],
            "`#{` is never closed",
            None,
            "site.scss",
            1,
            "a { b: «#{»1 + 2; }",
        ),
        (
            &[("site.scss", "a { b: 1; }\n}\n")],
            "`}` closes nothing",
            None,
            "site.scss",
            2,
            "«}»",
        ),
        (
            &[("site.scss", "a { color red; }\n")],
            "a `{` is missing",
            Some("Write it as `color: red;`."),
            "site.scss",
            1,
            "a { color red«;» }",
        ),
        (
            &[("site.scss", "a b;\n")],
            "a `{` is missing",
            None,
            "site.scss",
            1,
            "a b«;»",
        ),
        (
            &[("site.scss", "a { b: (1 + 2; }\n")],
            "`(` is never closed",
            None,
            "site.scss",
            1,
            "a { b: «(»1 + 2; }",
        ),
        (
            &[("site.scss", "a { b: url(http://x; }\n")],
            "`(` is never closed",
            None,
            "site.scss",
            1,
            "a { b: url«(»http://x; }",
        ),
        (
            &[("site.scss", "a { b: [1, 2; }\n")],
            "`[` is never closed",
            None,
            "site.scss",
            1,
            "a { b: «[»1, 2; }",
        ),
        (
            &[("site.scss", "@function f { @return 1; }\n")],
            "a `(` is missing",
            None,
            "site.scss",
            1,
            "@function f «{» @return 1; }",
        ),
        (
            &[("site.scss", "a { b: \"x; }\n")],
            "`\"` is never closed",
            None,
            "site.scss",
            1,
            "a { b: «\"»x; }",
        ),
        (
            &[("site.scss", "a { b: 'x; }\n")],
            "`'` is never closed",
            None,
            "site.scss",
            1,
            "a { b: «'»x; }",
        ),
        (
            &[("site.scss", "a { color: ; }\n")],
            "a value is missing",
            None,
            "site.scss",
            1,
            "a { color: «;» }",
        ),
        (
            &[("site.scss", "@media { a { b: c } }\n")],
            "a name is missing",
            None,
            "site.scss",
            1,
            "@media «{» a { b: c } }",
        ),
        (
            &[("site.scss", "@use 1;\n")],
            "a text in quotes is missing",
            None,
            "site.scss",
            1,
            "@use 1«;»",
        ),
        (
            &[("site.scss", "a { b: c; }\n/* x\n")],
            "`/*` is never closed",
            None,
            "site.scss",
            2,
            "«/*» x",
        ),
        (
            &[("site.scss", "a[b= { }\n")],
            "the stylesheet ends too soon",
            None,
            "site.scss",
            1,
            "a[b«=» { }",
        ),
        (
            &[("site.scss", "a { b: c !importnt; }\n")],
            "unknown flag `!importnt`",
            Some("Did you mean `!important`?"),
            "site.scss",
            1,
            "a { b: c «!importnt»; }",
        ),
        (
            &[("site.scss", "a { b: calc(1px+2px); }\n")],
            "`+` needs a space on each side in a calculation",
            None,
            "site.scss",
            1,
            "a { b: calc(1px«+»2px); }",
        ),
        (
            &[("site.scss", "& { b: c; }\n")],
            "`&` has nothing to stand for outside a rule",
            None,
            "site.scss",
            1,
            "«&» { b: c; }",
        ),
        (
            &[("site.scss", "@else { a { b: c } }\n")],
            "`@else` can't be used here",
            None,
            "site.scss",
            1,
            "«@else» { a { b: c } }",
        ),
        (
            &[("site.scss", "@return 1;\n")],
            "`@return` works only inside `@function`",
            None,
            "site.scss",
            1,
            "«@return 1»;",
        ),
        (
            &[("site.scss", "a { b: c; }\n@use \"sass:math\";\n")],
            "`@use` must come before any other rule",
            None,
            "site.scss",
            2,
            "«@use \"sass:math\"»;",
        ),
        (
            &[
                ("site.scss", "a { b: c; }\n@forward \"part\";\n"),
                ("_part.scss", PART),
            ],
            "`@forward` must come before any other rule",
            None,
            "site.scss",
            2,
            "«@forward \"part\"»;",
        ),
        (
            &[("site.scss", "@content;\n")],
            "`@content` works only inside `@mixin`",
            None,
            "site.scss",
            1,
            "«@content»;",
        ),
        (
            &[("site.scss", "@extend .a;\n")],
            "`@extend` works only inside a rule",
            None,
            "site.scss",
            1,
            "«@extend» .a;",
        ),
        (
            &[("site.scss", "@function f() { a { b: c } @return 1; }\n")],
            "`@function` can't hold style rules",
            None,
            "site.scss",
            1,
            "@function f() { «a { b: c }» @return 1; }",
        ),
        (
            &[(
                "site.scss",
                "@mixin m { a: b; }\na { @include m { c: d; } }\n",
            )],
            "the mixin `m` takes no content block",
            None,
            "site.scss",
            2,
            "a { @include «m» { c: d; } }",
        ),
        (
            &[("site.scss", "@function f() { $x: 1; }\na { b: f(); }\n")],
            "`@function f` ends without `@return`",
            None,
            "site.scss",
            2,
            "a { b: «f()»; }",
        ),
        (
            &[("site.scss", "a { b: darken(red); }\n")],
            "`darken` is missing `$amount`",
            None,
            "site.scss",
            1,
            "a { b: «darken(red)»; }",
        ),
        (
            &[("site.scss", "@mixin m($a) { b: $a; }\na { @include m; }\n")],
            "`m` is missing `$a`",
            None,
            "site.scss",
            2,
            "a { @include «m»; }",
        ),
        (
            &[("site.scss", "a { b: darken(red, 1%, 2); }\n")],
            "`darken` takes 2 arguments, not 3",
            None,
            "site.scss",
            1,
            "a { b: «darken(red, 1%, 2)»; }",
        ),
        (
            &[(
                "site.scss",
                "@mixin m($a) { b: $a; }\na { @include m(1, 2); }\n",
            )],
            "`m` takes 1 argument, not 2",
            None,
            "site.scss",
            2,
            "a { @include «m»(1, 2); }",
        ),
        (
            &[(
                "site.scss",
                "@use \"sass:color\";\na { b: color.adjust(red, $nope: 1); }\n",
            )],
            "`color.adjust` has no argument called `$nope`",
            None,
            "site.scss",
            2,
            "a { b: «color.adjust(red, $nope: 1)»; }",
        ),
        (
            &[("site.scss", "@use \"nosuch\";\n")],
            "no stylesheet called `nosuch`",
            None,
            "site.scss",
            1,
            "«@use \"nosuch\"»;",
        ),
        (
            &[("site.scss", "@use \"prt\";\n"), ("_part.scss", PART)],
            "no stylesheet called `prt`",
            Some("Did you mean `part`?"),
            "site.scss",
            1,
            "«@use \"prt\"»;",
        ),
        (
            &[("site.scss", "@use \"sass:nosuch\";\n")],
            "Sass has no module called `sass:nosuch`",
            None,
            "site.scss",
            1,
            "«@use \"sass:nosuch\"»;",
        ),
        (
            &[("site.scss", "a { b: math.div(1, 2); }\n")],
            "no module called `math`",
            Some("Add `@use \"sass:math\";` at the top of the file."),
            "site.scss",
            1,
            "a { b: «math».div(1, 2); }",
        ),
        (
            &[("site.scss", "a { b: x.y(1); }\n")],
            "no module called `x`",
            None,
            "site.scss",
            1,
            "a { b: «x».y(1); }",
        ),
        (
            &[
                ("site.scss", "@use \"sass:math\";\n@use \"part\" as math;\n"),
                ("_part.scss", PART),
            ],
            "two modules are called `math`",
            None,
            "site.scss",
            2,
            "«@use \"part\" as math»;",
        ),
        (
            &[
                ("site.scss", "@use \"part\";\na { b: part.$-x; }\n"),
                ("_part.scss", "$-x: 1;\n"),
            ],
            "`$-x` is private to its module",
            None,
            "site.scss",
            2,
            "a { b: «part.$-x»; }",
        ),
        (
            &[
                ("site.scss", "@use \"part\" with ($x: 2);\n"),
                ("_part.scss", PART),
            ],
            "`$x` isn't set with `!default` in that module",
            None,
            "site.scss",
            1,
            "@use \"part\" with («$x: 2»);",
        ),
        (
            &[
                ("site.scss", "@use \"part\";\n"),
                ("_part.scss", "@use \"site\";\n"),
            ],
            "this stylesheet loads itself",
            None,
            "_part.scss",
            1,
            "«@use \"site\"»;",
        ),
        (
            &[
                ("site.scss", "@import \"part\";\n"),
                ("_part.scss", "@import \"site\";\n"),
            ],
            "this stylesheet loads itself",
            None,
            "_part.scss",
            1,
            "@import «\"site\"»;",
        ),
        (
            &[("site.scss", "a { b: 1px + 1em; }\n")],
            "can't combine `px` and `em`",
            Some("Mix units with `calc()`."),
            "site.scss",
            1,
            "a { b: «1px + 1em»; }",
        ),
        (
            &[("site.scss", "a { b: 1px + red; }\n")],
            "can't work out `1px + red`",
            None,
            "site.scss",
            1,
            "a { b: «1px + red»; }",
        ),
        (
            &[("site.scss", "$x: 1px;\na { b: $x * $x; }\n")],
            "`1px*px` can't be written as CSS",
            None,
            "site.scss",
            1,
            "«»$x: 1px;",
        ),
        (
            &[("site.scss", "a { b: map-get(1, 2); }\n")],
            "`$map` needs a map, not `1`",
            None,
            "site.scss",
            1,
            "a { b: «map-get(1, 2)»; }",
        ),
        (
            &[("site.scss", "a { b: red(1); }\n")],
            "`$color` needs a colour, not `1`",
            None,
            "site.scss",
            1,
            "a { b: «red(1)»; }",
        ),
        (
            &[("site.scss", "@use \"sass:meta\";\na { b: meta.call(1); }\n")],
            "`$function` needs a function, not `1`",
            None,
            "site.scss",
            2,
            "a { b: «meta.call(1)»; }",
        ),
        (
            &[(
                "site.scss",
                "@use \"sass:math\";\na { b: math.sqrt(1px); }\n",
            )],
            "`$number` needs a number without units, not `1px`",
            None,
            "site.scss",
            2,
            "a { b: «math.sqrt(1px)»; }",
        ),
        (
            &[("site.scss", "a { b: nth(1 2, 5); }\n")],
            "`$n` is 5, but the list has 2 items",
            None,
            "site.scss",
            1,
            "a { b: «nth(1 2, 5)»; }",
        ),
        (
            &[("site.scss", "a { b: nth(1, 5); }\n")],
            "`$n` is 5, but the list has 1 item",
            None,
            "site.scss",
            1,
            "a { b: «nth(1, 5)»; }",
        ),
        (
            &[("site.scss", "$m: (a: 1, a: 2);\n")],
            "the map has `a` twice",
            None,
            "site.scss",
            1,
            "$m: (a: 1, «a»: 2);",
        ),
        (
            &[(
                "site.scss",
                "@for $i from 1px through 2em { a { b: $i } }\n",
            )],
            "`@for` can't count from `1px` to `2em`",
            None,
            "site.scss",
            1,
            "@for $i from «1px through 2em» { a { b: $i } }",
        ),
        (
            &[("site.scss", ".a { b: c; }\n.d { @extend .a.e; }\n")],
            "`@extend` can't take a compound selector",
            Some("Write it as `@extend .a, .e`."),
            "site.scss",
            2,
            ".d { «@extend .a.e»; }",
        ),
        (
            &[("site.sass", "a\n  b: c;\n")],
            "a `;` can't be used in a `.sass` file",
            None,
            "site.sass",
            2,
            "  b: c«;»",
        ),
        (
            &[("site.sass", "a\n  b: c\n\td: e\n")],
            "a tab where this file indents with spaces",
            None,
            "site.sass",
            3,
            "«\t»d: e",
        ),
        (
            &[("site.sass", "a\n\tb: c\n  d: e\n")],
            "a space where this file indents with tabs",
            None,
            "site.sass",
            3,
            "«  »d: e",
        ),
        (
            &[("site.sass", "  a\n    b: c\n")],
            "the first line is indented",
            None,
            "site.sass",
            1,
            "«  »a",
        ),
        (
            &[("site.scss", "@for $i from 1 { }\n")],
            "expected \"to\" or \"through\"",
            None,
            "site.scss",
            1,
            "@for $i from 1 «{» }",
        ),
        (
            &[("site.scss", "@use \"1x\";\n")],
            "the default namespace \"1x\" is not a valid Sass identifier",
            None,
            "site.scss",
            1,
            "«@use \"1x\"»;",
        ),
    ];
    for (i, (files, headline, suggestion, file, line, marked)) in cases.iter().enumerate() {
        let files: Vec<(String, &str)> = files
            .iter()
            .map(|(name, text)| (format!("assets/{name}"), *text))
            .collect();
        let files: Vec<(&str, &str)> = files.iter().map(|(n, t)| (n.as_str(), *t)).collect();
        let site = Site::new(&format!("sass-why-{i}"), &files);
        let (_, problems, sources) = site.gather("");
        let [problem] = &problems[..] else {
            panic!("{files:?}: {problems:?}")
        };
        let place = problem.place.unwrap();
        let text = sources.text(place.source);
        let (start, end) = (place.span.start as usize, place.span.end as usize);
        let from = text[..start].rfind('\n').map_or(0, |at| at + 1);
        let to = text[end..].find('\n').map_or(text.len(), |at| end + at);
        let seen = (
            crate::wording::headline(&problem.kind),
            crate::wording::suggestion(&problem.kind),
            sources.path(place.source).to_path_buf(),
            sources.line_column(place.source, start).line,
            format!(
                "{}«{}»{}",
                &text[from..start],
                &text[start..end],
                &text[end..to]
            ),
        );
        let wanted = (
            format!("Sass stopped: {headline}"),
            suggestion.map(str::to_owned),
            Path::new("assets").join(file),
            *line,
            marked.to_string(),
        );
        assert_eq!(seen, wanted, "{files:?}");
    }
}

/// A value CSS can't write is told by the stylesheet's name alone where
/// Sass gives no place for it, and at the text where it gives one.
#[test]
fn a_value_css_cant_write_is_told_by_the_stylesheet_where_sass_gives_no_place() {
    let cases = [
        ("$x: 1px;\na { b: $x * $x; }\n", true),
        ("a { b: (a: 1) + 1; }\n", false),
    ];
    for (i, (text, whole)) in cases.into_iter().enumerate() {
        let site = Site::new(&format!("sass-not-css-{i}"), &[("assets/site.scss", text)]);
        let (_, problems, _) = site.gather("");
        let [problem] = &problems[..] else {
            panic!("{text}: {problems:?}")
        };
        assert!(
            matches!(
                &problem.kind,
                Kind::SassStopped {
                    fault: SassFault::NotCss { .. }
                }
            ),
            "{text}: {problem:?}"
        );
        assert_eq!(problem.kind.of_a_whole_file(), whole, "{text}");
    }
}

#[test]
fn a_worker_named_by_an_asset_is_warned_of_at_its_name() {
    let site = Site::new(
        "workers",
        &[
            (
                "assets/app.js",
                "const w = new Worker('./worker.js', { type: 'module' });\n\
                 const x = new Worker('https://example.com/w.js');\n",
            ),
            ("assets/worker.js", "import { a } from './util.js';\n"),
            ("assets/util.js", "export const a = 1;\n"),
        ],
    );
    let (_, problems, sources) = site.gather("");
    let [problem] = &problems[..] else {
        panic!("{problems:?}")
    };
    assert_eq!(
        problem.kind,
        Kind::WorkerOutsideImportMap {
            written: "./worker.js".into()
        }
    );
    let place = problem.place.unwrap();
    assert_eq!(sources.path(place.source), Path::new("assets/app.js"));
    assert_eq!(sources.spelling(place), "./worker.js");
}

#[test]
fn hidden_files_stay_out_of_assets_but_travel_from_static() {
    let site = Site::new(
        "hidden",
        &[
            ("assets/.notes", ""),
            ("assets/site.css", ""),
            ("static/.well-known/security.txt", ""),
        ],
    );
    let (assets, problems, _) = site.gather("");
    assert_eq!(problems, []);
    assert_eq!(assets.iter().count(), 1);
    let statics: Vec<&str> = assets.statics().iter().map(|f| f.name.as_str()).collect();
    assert_eq!(statics, [".well-known/security.txt"]);
}

#[test]
fn every_script_published_is_listed_with_where_it_came_from() {
    let site = Site::new(
        "travelled",
        &[
            ("assets/app.js", ""),
            ("assets/site.css", ""),
            ("static/vendor/lib.mjs", ""),
            ("static/robots.txt", ""),
        ],
    );
    let (assets, _, _) = site.gather("/blog");
    let app = &assets.get("app.js").unwrap().published;
    assert_eq!(
        assets.travelled(),
        [
            Travelled {
                address: format!("/blog/{app}"),
                from: "assets/app.js".into(),
            },
            Travelled {
                address: "/blog/vendor/lib.mjs".into(),
                from: "static/vendor/lib.mjs".into(),
            },
        ]
    );
}

#[test]
fn two_files_claiming_one_name_are_both_named() {
    let site = Site::new(
        "claimed",
        &[
            ("assets/site.css", "a {}\n"),
            ("assets/site.scss", "b {}\n"),
        ],
    );
    let (_, problems, _) = site.gather("");
    assert_eq!(
        problems,
        [Problem::nowhere(Kind::SameAddress {
            first: "assets/site.css".into(),
            second: "assets/site.scss".into(),
            alike: Alike::Exactly,
            address: "/site.css".into(),
        })]
    );
}

#[test]
fn two_files_meeting_again_in_the_other_order_are_told_once() {
    let outputs: Vec<(String, Named)> = [
        ("Photo-1.png", "static/Photo-1.png"),
        ("photo-1.png", "content/photo-1.png"),
        ("photo-1-400.jpg", "content/photo-1.png"),
        ("Photo-1-400.jpg", "static/Photo-1.png"),
    ]
    .into_iter()
    .map(|(rel, source)| (rel.to_owned(), source.into()))
    .collect();
    assert_eq!(
        same_address(&outputs, ""),
        [Problem::nowhere(Kind::SameAddress {
            first: "static/Photo-1.png".into(),
            second: "content/photo-1.png".into(),
            alike: Alike::ByCase,
            address: "/photo-1.png".into(),
        })]
    );
}

#[test]
fn a_static_file_over_an_asset_is_an_error() {
    let site = Site::new("over", &[("assets/site.css", "body {}\n")]);
    let (assets, _, _) = site.gather("");
    let published = assets.get("site.css").unwrap().published.clone();
    site.write(&format!("static/{published}"), "");
    let (_, problems, _) = site.gather("");
    assert_eq!(
        problems,
        [Problem::nowhere(Kind::SameAddress {
            first: "assets/site.css".into(),
            second: format!("static/{published}").into(),
            alike: Alike::Exactly,
            address: format!("/{published}"),
        })]
    );
}

#[test]
fn a_page_over_an_asset_is_an_error() {
    let site = Site::new("page-over", &[("assets/site.css", "body {}\n")]);
    let (assets, _, _) = site.gather("");
    let published = assets.get("site.css").unwrap().published.clone();
    let mut claims = assets.claims();
    claims.push((published.clone(), "content/post-1.md".into()));
    assert_eq!(
        same_address(&claims, ""),
        [Problem::nowhere(Kind::SameAddress {
            first: "assets/site.css".into(),
            second: "content/post-1.md".into(),
            alike: Alike::Exactly,
            address: format!("/{published}"),
        })]
    );
}

/// What [`same_address`] tells, found the long way: every folder above
/// every output looked up again, however many outputs share it.
fn told_one_by_one(outputs: &[(String, Named)]) -> Vec<Kind> {
    let mut told: Vec<Kind> = Vec::new();
    let mut tell = |first: &Named, second: &Named, alike: Alike, rel: &str| {
        let twice = |kind: &Kind| {
            matches!(kind, Kind::SameAddress { first: a, second: b, .. }
                if **a == *first && **b == *second)
        };
        if !told.iter().any(twice) {
            told.push(Kind::SameAddress {
                first: Box::new(first.clone()),
                second: Box::new(second.clone()),
                alike,
                address: format!("/{rel}"),
            });
        }
    };
    let mut files: Vec<(String, usize)> = Vec::new();
    let mut folders: Vec<(String, String, Named)> = Vec::new();
    for (i, (rel, source)) in outputs.iter().enumerate() {
        let key = platform::folded(rel);
        match files.iter().find(|(held, _)| *held == key) {
            Some(&(_, first)) => {
                let (first_rel, first_source) = &outputs[first];
                tell(first_source, source, platform::alike(first_rel, rel), rel);
            }
            None => files.push((key, i)),
        }
        let pieces: Vec<&str> = rel.split('/').collect();
        let source_pieces: Vec<&str> = match source {
            Named::File(path) => path.split('/').collect(),
            _ => Vec::new(),
        };
        for depth in 1..pieces.len() {
            let spelled = pieces[..depth].join("/");
            let kept = source_pieces.len().saturating_sub(pieces.len() - depth);
            let folded = |pieces: &[&str]| -> Vec<String> {
                pieces.iter().map(|piece| platform::folded(piece)).collect()
            };
            let mirrors = kept > 0
                && folded(&source_pieces[kept - 1..source_pieces.len() - 1])
                    == folded(&pieces[depth - 1..pieces.len() - 1]);
            let from = match mirrors {
                true => Named::File(source_pieces[..kept].join("/")),
                false => source.clone(),
            };
            let key = platform::folded(&spelled);
            match folders.iter().find(|(held, _, _)| *held == key) {
                Some((_, first_spelled, first_from)) if *first_spelled != spelled => {
                    let alike = platform::alike(first_spelled, &spelled);
                    tell(first_from, &from, alike, &spelled);
                }
                Some(_) => {}
                None => folders.push((key, spelled, from)),
            }
        }
    }
    files.sort();
    for (key, i) in &files {
        if let Some((_, spelled, from)) = folders.iter().find(|(held, _, _)| held == key) {
            let (rel, source) = &outputs[*i];
            tell(from, source, platform::alike(spelled, rel), rel);
        }
    }
    told
}

#[test]
fn a_collision_names_the_same_files_however_many_share_a_folder() {
    // Folders spelled two ways, by files that mirror them and files that
    // don't, several to a folder; a file where a folder goes; two files
    // at one address.
    let pool: [(&str, Named); 9] = [
        ("Blog/a.html", "content/Blog/a.md".into()),
        ("blog/b.html", "content/posts/b.md".into()),
        ("blog/c.html", "content/posts/c.md".into()),
        ("blog/d.html", "content/blog/d.md".into()),
        ("BLOG/sub/e.html", "content/posts/e.md".into()),
        ("blog/Sub/f.html", "static/blog/Sub/f.html".into()),
        ("blog/sub", "static/blog/sub".into()),
        ("blog/c.html", "content/posts/c2.md".into()),
        ("blog/sub/g.html", Named::Item("item 2".into())),
    ];
    // Every order of every five of them.
    let mut orders = 0;
    let mut order = Vec::new();
    fn each(pool: &[(&str, Named)], order: &mut Vec<usize>, orders: &mut usize) {
        if order.len() == 5 {
            let outputs: Vec<(String, Named)> = order
                .iter()
                .map(|&at| (pool[at].0.to_owned(), pool[at].1.clone()))
                .collect();
            let told: Vec<Kind> = same_address(&outputs, "")
                .into_iter()
                .map(|problem| problem.kind)
                .collect();
            assert_eq!(told, told_one_by_one(&outputs), "{outputs:?}");
            *orders += 1;
            return;
        }
        for at in 0..pool.len() {
            if !order.contains(&at) {
                order.push(at);
                each(pool, order, orders);
                order.pop();
            }
        }
    }
    each(&pool, &mut order, &mut orders);
    assert_eq!(orders, 9 * 8 * 7 * 6 * 5);
}

#[test]
fn names_a_disk_holds_as_one_collide() {
    let outputs = |pairs: &[(&str, &str)]| -> Vec<(String, Named)> {
        pairs
            .iter()
            .map(|(rel, source)| (rel.to_string(), (*source).into()))
            .collect()
    };
    let kinds = |pairs: &[(&str, &str)]| -> Vec<Kind> {
        same_address(&outputs(pairs), "")
            .into_iter()
            .map(|problem| problem.kind)
            .collect()
    };
    assert_eq!(
        kinds(&[
            ("Logo.png", "static/Logo.png"),
            ("logo.png", "static/logo.png")
        ]),
        [Kind::SameAddress {
            first: "static/Logo.png".into(),
            second: "static/logo.png".into(),
            alike: Alike::ByCase,
            address: "/logo.png".into(),
        }]
    );
    assert_eq!(
        kinds(&[
            ("caf\u{e9}.txt", "static/caf\u{e9}.txt"),
            ("cafe\u{301}.txt", "static/cafe\u{301}.txt"),
        ]),
        [Kind::SameAddress {
            first: "static/caf\u{e9}.txt".into(),
            second: "static/cafe\u{301}.txt".into(),
            alike: Alike::ByNormalisation,
            address: "/cafe\u{301}.txt".into(),
        }]
    );
    // One folder spelled two ways, named once.
    assert_eq!(
        kinds(&[
            ("Fonts/a.woff2", "static/Fonts/a.woff2"),
            ("fonts/b.woff2", "static/fonts/b.woff2"),
            ("fonts/c.woff2", "static/fonts/c.woff2"),
        ]),
        [Kind::SameAddress {
            first: "static/Fonts".into(),
            second: "static/fonts".into(),
            alike: Alike::ByCase,
            address: "/fonts".into(),
        }]
    );
    // A file where an asset's folder goes.
    assert_eq!(
        kinds(&[
            ("fonts/a.1a2b3c4d.woff2", "assets/fonts/a.woff2"),
            ("fonts", "static/fonts"),
        ]),
        [Kind::SameAddress {
            first: "assets/fonts".into(),
            second: "static/fonts".into(),
            alike: Alike::Exactly,
            address: "/fonts".into(),
        }]
    );
    // A page against an asset, by case.
    assert_eq!(
        kinds(&[
            ("site.1a2b3c4d.css", "assets/site.css"),
            ("Site.1a2b3c4d.css", "content/post-1.md"),
        ]),
        [Kind::SameAddress {
            first: "assets/site.css".into(),
            second: "content/post-1.md".into(),
            alike: Alike::ByCase,
            address: "/Site.1a2b3c4d.css".into(),
        }]
    );
    // Two pages whose folders differ by case: a page's file doesn't mirror
    // its address, so the files themselves are named, once.
    assert_eq!(
        kinds(&[
            ("post-2/index.html", "content/Post-2.md"),
            ("POST-2/index.html", "content/post-3.md"),
        ]),
        [Kind::SameAddress {
            first: "content/Post-2.md".into(),
            second: "content/post-3.md".into(),
            alike: Alike::ByCase,
            address: "/POST-2/index.html".into(),
        }]
    );
    assert_eq!(
        kinds(&[
            ("fonts/a.woff2", "static/fonts/a.woff2"),
            ("fonts/b.woff2", "static/fonts/b.woff2"),
        ]),
        []
    );
}

// Windows can't hold these names, so the site can't be written there.
#[cfg(unix)]
#[test]
fn a_name_some_disk_cant_hold_is_an_error() {
    let site = Site::new(
        "unwritable",
        &[("static/con.txt", ""), ("assets/a:b.css", "")],
    );
    let (_, problems, _) = site.gather("");
    let kinds: Vec<Kind> = problems.into_iter().map(|problem| problem.kind).collect();
    assert_eq!(
        kinds,
        [
            Kind::UnwritableName {
                path: "assets/a:b.css".into(),
                why: Unwritable::Character(':'),
            },
            Kind::UnwritableName {
                path: "static/con.txt".into(),
                why: Unwritable::Device("con".into()),
            },
        ]
    );
}

#[test]
fn publishing_writes_assets_under_their_folder_and_static_at_the_top() {
    let site = Site::new(
        "publish",
        &[
            ("assets/site.css", "body {}\n"),
            ("static/robots.txt", "User-agent: *\n"),
        ],
    );
    let (assets, problems, _) = site.gather("");
    assert_eq!(problems, []);
    let out = site.0.join("out");
    let staging = crate::publish::begin(&site.0, &out).unwrap();
    assert_eq!(assets.publish(&staging), []);
    staging.finish().unwrap();
    let published = &assets.get("site.css").unwrap().published;
    assert_eq!(
        fs::read_to_string(out.join(published)).unwrap(),
        "body {}\n"
    );
    assert_eq!(
        fs::read_to_string(out.join("robots.txt")).unwrap(),
        "User-agent: *\n"
    );
}

/// A file beside the site folder, for a site to reach out to.
struct Outside(PathBuf);

impl Outside {
    fn new(name: &str, text: &str) -> Outside {
        let path =
            std::env::temp_dir().join(format!("statac-outside-{name}-{}.scss", std::process::id()));
        fs::write(&path, text).unwrap();
        Outside(path)
    }
}

impl Drop for Outside {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn reached_out(problems: &[Problem]) -> Vec<&Kind> {
    problems
        .iter()
        .map(|problem| &problem.kind)
        .filter(|kind| matches!(kind, Kind::OutsideSite { .. }))
        .collect()
}

#[test]
fn sass_reads_nothing_outside_assets() {
    let outside = Outside::new("relative", ".kept { content: \"kept\"; }\n");
    let stem = outside.0.file_stem().unwrap().to_str().unwrap().to_owned();
    let written = format!("../../{stem}");
    let site = Site::new(
        "sass-outside",
        &[(
            "assets/site.scss",
            &format!("@use \"{written}\";\nbody {{}}\n"),
        )],
    );
    let (assets, problems, sources) = site.gather("");
    let [problem] = &problems[..] else {
        panic!("{problems:?}")
    };
    assert_eq!(
        problem.kind,
        Kind::OutsideSite {
            reach: Reach::Load(written.clone()),
        }
    );
    let place = problem.place.unwrap();
    assert_eq!(sources.path(place.source), Path::new("assets/site.scss"));
    assert_eq!(
        sources
            .line_column(place.source, place.span.start as usize)
            .line,
        1
    );
    assert!(assets.get("site.css").is_none());
}

#[test]
fn sass_reads_nothing_by_an_absolute_path() {
    let outside = Outside::new("absolute", ".kept { content: \"kept\"; }\n");
    let written = outside.0.with_extension("").to_str().unwrap().to_owned();
    let site = Site::new(
        "sass-absolute",
        &[(
            "assets/site.scss",
            &format!("@import \"{written}\";\nbody {{}}\n"),
        )],
    );
    let (assets, problems, _) = site.gather("");
    assert_eq!(
        reached_out(&problems),
        [&Kind::OutsideSite {
            reach: Reach::Load(written),
        }]
    );
    assert!(assets.get("site.css").is_none());
}

#[test]
fn sass_still_reads_partials_in_other_folders_of_assets() {
    let site = Site::new(
        "sass-within",
        &[
            (
                "assets/css/site.scss",
                "@use '../parts/colours';\nbody { color: colours.$ink; }\n",
            ),
            ("assets/parts/_colours.scss", "$ink: #333;\n"),
        ],
    );
    let (assets, problems, _) = site.gather("");
    assert_eq!(problems, []);
    assert!(text(assets.get("css/site.css").unwrap()).contains("#333"));
}

#[cfg(unix)]
fn link(path: &str, within: &str) -> Kind {
    Kind::OutsideSite {
        reach: Reach::Link {
            path: path.into(),
            within: Some(within.into()),
        },
    }
}

#[cfg(unix)]
#[test]
fn a_link_out_of_the_site_is_never_followed() {
    use std::os::unix::fs::symlink;
    let outside = Outside::new("link", "private\n");
    let site = Site::new("link-out", &[("static/robots.txt", "")]);
    symlink(&outside.0, site.0.join("static/key.txt")).unwrap();
    symlink(std::env::temp_dir(), site.0.join("static/elsewhere")).unwrap();
    let (assets, problems, _) = site.gather("");
    assert_eq!(
        reached_out(&problems),
        [
            &link("static/elsewhere", "static/"),
            &link("static/key.txt", "static/"),
        ]
    );
    let names: Vec<&str> = assets.statics().iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["robots.txt"]);
}

#[cfg(unix)]
#[test]
fn a_link_out_of_its_own_folder_is_never_followed() {
    use std::os::unix::fs::symlink;
    let site = Site::new(
        "link-folder",
        &[
            ("statac.yaml", "webmention_token: secret\n"),
            (".git/config", "[core]\n"),
            ("assets/site.css", "body {}\n"),
            ("static/robots.txt", ""),
        ],
    );
    symlink(
        site.0.join("statac.yaml"),
        site.0.join("static/settings.txt"),
    )
    .unwrap();
    symlink(site.0.join(".git"), site.0.join("static/repository")).unwrap();
    symlink(
        site.0.join("static/robots.txt"),
        site.0.join("assets/robots.txt"),
    )
    .unwrap();
    symlink(
        site.0.join("assets/site.css"),
        site.0.join("static/site.css"),
    )
    .unwrap();
    let (assets, problems, _) = site.gather("");
    assert_eq!(
        reached_out(&problems),
        [
            &link("assets/robots.txt", "assets/"),
            &link("static/repository", "static/"),
            &link("static/settings.txt", "static/"),
            &link("static/site.css", "static/"),
        ]
    );
    let names: Vec<&str> = assets.statics().iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["robots.txt"]);
    assert!(assets.get("robots.txt").is_none());
}

#[cfg(unix)]
#[test]
fn a_link_within_its_folder_is_followed() {
    use std::os::unix::fs::symlink;
    let site = Site::new(
        "link-within",
        &[
            ("static/notes/a.txt", "a\n"),
            ("static/notes/deep/b.txt", "b\n"),
        ],
    );
    symlink(
        site.0.join("static/notes/a.txt"),
        site.0.join("static/a.txt"),
    )
    .unwrap();
    symlink(site.0.join("static/notes/deep"), site.0.join("static/deep")).unwrap();
    let (assets, problems, _) = site.gather("");
    assert_eq!(problems, []);
    let names: Vec<&str> = assets.statics().iter().map(|f| f.name.as_str()).collect();
    // A folder reached twice is walked once.
    assert_eq!(names, ["a.txt", "notes/a.txt", "notes/deep/b.txt"]);
}

#[cfg(unix)]
#[test]
fn a_link_written_relative_stays_within_its_folder_or_is_refused() {
    use std::os::unix::fs::symlink;
    let site = Site::new(
        "link-relative",
        &[
            ("statac.yaml", "title: Site\n"),
            ("static/y.txt", "y\n"),
            ("assets/site.css", "body {}\n"),
        ],
    );
    symlink("../static/y.txt", site.0.join("static/x.txt")).unwrap();
    symlink("../statac.yaml", site.0.join("static/settings.txt")).unwrap();
    symlink("../static/y.txt", site.0.join("assets/x.txt")).unwrap();
    let (assets, problems, _) = site.gather("");
    assert_eq!(
        reached_out(&problems),
        [
            &link("assets/x.txt", "assets/"),
            &link("static/settings.txt", "static/"),
        ]
    );
    let names: Vec<&str> = assets.statics().iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["x.txt", "y.txt"]);
}

#[cfg(unix)]
#[test]
fn a_folder_that_is_itself_a_link_out_is_refused() {
    use std::os::unix::fs::symlink;
    let elsewhere = Site::new("link-folder-target", &[("static/secret.txt", "secret\n")]);
    let site = Site::new("link-folder-site", &[("statac.yaml", "title: Site\n")]);
    symlink(elsewhere.0.join("static"), site.0.join("static")).unwrap();
    let inside = Site::new("link-folder-inside", &[("public/a.txt", "a\n")]);
    symlink(inside.0.join("public"), inside.0.join("static")).unwrap();
    let (assets, problems, _) = site.gather("");
    assert_eq!(
        reached_out(&problems),
        [&Kind::OutsideSite {
            reach: Reach::Link {
                path: "static".into(),
                within: None,
            },
        }]
    );
    assert!(assets.statics().is_empty());
    let (assets, problems, _) = inside.gather("");
    assert_eq!(reached_out(&problems), [&link("static", "static/")]);
    assert!(assets.statics().is_empty());
}

#[cfg(unix)]
#[test]
fn a_link_pointed_elsewhere_after_the_walk_is_read_where_it_led() {
    use std::os::unix::fs::symlink;
    let outside = Outside::new("repointed", "private\n");
    let site = Site::new("link-repointed", &[("static/notes/a.txt", "a\n")]);
    symlink(
        site.0.join("static/notes/a.txt"),
        site.0.join("static/a.txt"),
    )
    .unwrap();
    let (assets, problems, _) = site.gather("");
    assert_eq!(problems, []);
    fs::remove_file(site.0.join("static/a.txt")).unwrap();
    symlink(&outside.0, site.0.join("static/a.txt")).unwrap();
    let out = site.0.join("out");
    let staging = crate::publish::begin(&site.0, &out).unwrap();
    assert_eq!(assets.publish(&staging), []);
    staging.finish().unwrap();
    assert_eq!(fs::read_to_string(out.join("a.txt")).unwrap(), "a\n");
}

#[cfg(unix)]
#[test]
fn a_link_put_in_place_of_a_file_after_the_walk_is_refused() {
    use std::os::unix::fs::symlink;
    let outside = Outside::new("swapped", "private\n");
    let site = Site::new(
        "link-swapped",
        &[("assets/site.css", "body {}\n"), ("static/a.txt", "a\n")],
    );
    let real = fs::canonicalize(&site.0).unwrap();
    let walked = walk(&site.0, &real, ASSETS, false, &mut Vec::new());
    let (assets, problems, _) = site.gather("");
    assert_eq!(problems, []);
    for file in ["assets/site.css", "static/a.txt"] {
        fs::remove_file(site.0.join(file)).unwrap();
        symlink(&outside.0, site.0.join(file)).unwrap();
    }
    let [(_, at)] = &walked[..] else {
        panic!("{walked:?}")
    };
    let refused = read_walked(&real, at).unwrap_err();
    assert_eq!(Reason::of(&refused), Reason::Link);
    let out = site.0.join("out");
    let staging = crate::publish::begin(&site.0, &out).unwrap();
    assert_eq!(
        assets.publish(&staging),
        [Problem::nowhere(Kind::CantRead {
            path: "static/a.txt".into(),
            reason: Reason::Link,
        })]
    );
}

#[cfg(unix)]
#[test]
fn sass_reads_a_partial_through_a_link_within_assets() {
    use std::os::unix::fs::symlink;
    let site = Site::new(
        "sass-link",
        &[
            (
                "assets/site.scss",
                "@use 'colours';\nbody { color: colours.$ink; }\n",
            ),
            ("assets/parts/_colours.scss", "$ink: #333;\n"),
        ],
    );
    symlink(
        site.0.join("assets/parts/_colours.scss"),
        site.0.join("assets/_colours.scss"),
    )
    .unwrap();
    let (assets, problems, _) = site.gather("");
    assert_eq!(problems, []);
    assert!(text(assets.get("site.css").unwrap()).contains("#333"));
}
