//! A small site's assets and static files, gathered and published into an
//! output folder, end to end.

use std::fs;
use std::path::{Path, PathBuf};

use statac::assets::{self, PageScripts, Travelled};
use statac::problem::{Kind, Problem, Severity};
use statac::publish::{self, MARKER};
use statac::reads::Reads;
use statac::source::Sources;

struct Folder(PathBuf);

impl Folder {
    fn new() -> Folder {
        let path = std::env::temp_dir().join(format!("statac-site-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        Folder(path)
    }

    fn write(&self, path: &str, text: &str) {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn errors(problems: &[Problem]) -> Vec<&Problem> {
    problems
        .iter()
        .filter(|problem| problem.kind.severity() == Severity::Error)
        .collect()
}

#[test]
fn a_site_is_gathered_and_published_whole_or_not_at_all() {
    let folder = Folder::new();
    let site = folder.0.join("site");
    for (path, text) in [
        (
            "site/assets/site.scss",
            "@use 'type';\n\
             @font-face { font-family: a; src: url(fonts/a.woff2) format('woff2'); }\n\
             body { font-family: a; }\n",
        ),
        (
            "site/assets/_type.scss",
            "$size: 2rem;\nh1 { font-size: $size; }\n",
        ),
        ("site/assets/fonts/a.woff2", "font"),
        (
            "site/assets/app.js",
            "import { greet } from './util.js';\n\
             export const name = 'post-1';\n\
             const worker = new Worker('./worker.js', { type: 'module' });\n\
             greet();\n",
        ),
        (
            "site/assets/util.js",
            "import { name } from './app.js';\nexport function greet() { return name; }\n",
        ),
        ("site/assets/worker.js", "self.onmessage = () => {};\n"),
        ("site/static/robots.txt", "User-agent: *\n"),
        ("site/static/vendor/lib.js", "window.lib = {};\n"),
    ] {
        folder.write(path, text);
    }

    let mut sources = Sources::default();
    let (gathered, problems) = assets::gather(&site, "", None, &mut sources);

    // The worker is the one thing said.
    let [warning] = &problems[..] else {
        panic!("{problems:?}")
    };
    assert_eq!(
        warning.headline(),
        "Worker outside the import map: `./worker.js` isn't published under that name"
    );
    let place = warning.place.unwrap();
    assert_eq!(sources.path(place.source), Path::new("assets/app.js"));
    assert_eq!(
        sources
            .line_column(place.source, place.span.start as usize)
            .line,
        3
    );

    let published = |name: &str| gathered.get(name).unwrap().published.clone();
    let (css, font) = (published("site.css"), published("fonts/a.woff2"));
    let (app, util, worker) = (
        published("app.js"),
        published("util.js"),
        published("worker.js"),
    );
    for name in [&css, &font, &app, &util, &worker] {
        let mark = name.rsplit('.').nth(1).unwrap();
        assert_eq!(mark.len(), 8, "{name}");
        assert!(mark.chars().all(|c| c.is_ascii_hexdigit()), "{name}");
    }
    assert!(gathered.get("_type.css").is_none());

    // The template functions.
    let mut reads = Reads::default();
    let mut page = PageScripts::default();
    assert_eq!(
        gathered.css("site.css", &mut reads).unwrap(),
        format!("<link rel=\"stylesheet\" href=\"/{css}\">")
    );
    assert_eq!(
        gathered.asset("fonts/a.woff2", &mut reads).unwrap(),
        format!("/{font}")
    );
    let map = format!(
        "<script type=\"importmap\">\n{{\n  \"imports\": {{\n    \
         \"/app.js\": \"/{app}\",\n    \
         \"/util.js\": \"/{util}\",\n    \
         \"/worker.js\": \"/{worker}\"\n  }}\n}}\n</script>"
    );
    assert_eq!(gathered.import_map(&mut Reads::default()), map);
    assert_eq!(
        gathered.js("app.js", &mut page, &mut reads).unwrap(),
        format!("{map}\n<script type=\"module\" src=\"/{app}\"></script>")
    );
    assert_eq!(
        gathered.js("util.js", &mut page, &mut reads).unwrap(),
        format!("<script type=\"module\" src=\"/{util}\"></script>")
    );
    assert_eq!(
        gathered.css("sit.css", &mut reads).unwrap_err(),
        Kind::MissingFile {
            path: "assets/sit.css".into(),
            hint: Some(statac::problem::Hint::Closest("site.css".into())),
        }
    );

    // What travelled.
    assert_eq!(
        gathered.travelled(),
        [
            Travelled {
                address: format!("/{app}"),
                from: "assets/app.js".into(),
            },
            Travelled {
                address: format!("/{util}"),
                from: "assets/util.js".into(),
            },
            Travelled {
                address: format!("/{worker}"),
                from: "assets/worker.js".into(),
            },
            Travelled {
                address: "/vendor/lib.js".into(),
                from: "static/vendor/lib.js".into(),
            },
        ]
    );

    // Published.
    let out = folder.0.join("out");
    let staging = publish::begin(&site, &out).unwrap();
    assert_eq!(gathered.publish(&staging), []);
    staging.finish().unwrap();
    let stylesheet = read(&out.join(&css));
    assert!(stylesheet.contains("font-size: 2rem"), "{stylesheet}");
    assert!(
        stylesheet.contains(&format!("url({font}) format(\"woff2\")"))
            || stylesheet.contains(&format!("url({font}) format('woff2')")),
        "{stylesheet}"
    );
    assert_eq!(read(&out.join(&font)), "font");
    assert!(read(&out.join(&app)).starts_with("import { greet } from './util.js';"));
    assert_eq!(read(&out.join("robots.txt")), "User-agent: *\n");
    assert_eq!(read(&out.join("vendor/lib.js")), "window.lib = {};\n");
    assert!(out.join(MARKER).is_file());
    assert!(!out.join("assets/site.css").exists());
    assert!(!out.join("assets/app.js").exists());

    // A static file over an asset's address stops the build, and the
    // published site stays as it was.
    folder.write(&format!("site/static/{util}"), "");
    let (gathered, problems) = assets::gather(&site, "", None, &mut Sources::default());
    let errors = errors(&problems);
    let [collision] = &errors[..] else {
        panic!("{problems:?}")
    };
    assert_eq!(
        collision.headline(),
        format!(
            "Same address: `assets/util.js` and `static/{util}` are both published at \
             `/{util}`"
        )
    );
    let staging = publish::begin(&site, &out).unwrap();
    gathered.publish(&staging);
    drop(staging);
    assert_eq!(read(&out.join("robots.txt")), "User-agent: *\n");
    assert_eq!(read(&out.join(&util)), read(&site.join("assets/util.js")));
    let beside: Vec<_> = fs::read_dir(&folder.0)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(beside.len(), 2, "{beside:?}");

    // And a mistyped output folder is never replaced.
    let refused = publish::begin(&site, &site).unwrap_err();
    assert!(
        matches!(refused.kind, Kind::NotStatacsFolder { .. }),
        "{refused:?}"
    );
    assert!(site.join("assets/app.js").is_file());
}
