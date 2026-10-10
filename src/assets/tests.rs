use super::*;

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
fn a_name_in_the_map_cant_close_its_element() {
    assert_eq!(
        json_string("a</script>\"\\"),
        "\"a\\u003c/script>\\\"\\\\\""
    );
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
