use std::sync::atomic::{AtomicUsize, Ordering};

use super::Helper;
use super::resolve::{Arrival, Leads, Site};
use super::scan::{Attribute, scan};
use crate::config::Settings;
use crate::source::Sources;

fn settings(yaml: &str) -> Settings {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let folder = std::env::temp_dir().join(format!(
        "statac-links-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("statac.yaml"), yaml).unwrap();
    let (settings, problems) = crate::config::read(&folder, &mut Sources::default());
    let _ = std::fs::remove_dir_all(&folder);
    assert!(problems.is_empty(), "{problems:?}");
    settings
}

fn site(yaml: &str) -> Site {
    let files = [
        "index.html",
        "posts/post-1/index.html",
        "posts/post-1/photo-1.jpg",
        "site.0123abcd.css",
        "feed.xml",
        "404.html",
    ];
    Site::new(&settings(yaml), files.iter().map(|file| (*file).to_owned()))
}

fn here(path: &str, fragment: Option<&str>) -> Leads {
    Leads::Here(Arrival {
        path: path.to_owned(),
        fragment: fragment.map(str::to_owned),
        origin: None,
        outside: false,
        home: false,
    })
}

#[test]
fn a_link_is_followed_as_a_browser_follows_it() {
    let site = site("url: https://example.org/\nother_hosts: [www.example.org]\n");
    let from = "/posts/post-1/";
    let table: &[(&str, Leads)] = &[
        ("photo-1.jpg", here("/posts/post-1/photo-1.jpg", None)),
        ("../post-2/", here("/posts/post-2/", None)),
        ("./../../a/./b/../c", here("/a/c", None)),
        ("#notes", here("/posts/post-1/", Some("notes"))),
        ("?page=2", here("/posts/post-1/", None)),
        ("/caf%C3%A9/", here("/café/", None)),
        ("  /padded/  ", here("/padded/", None)),
        ("https://other.example/", Leads::Away),
        ("mailto:someone@example.org", Leads::Away),
        ("//cdn.example/x.js", Leads::Away),
        ("javascript:void(0)", Leads::Away),
    ];
    for (written, leads) in table {
        assert_eq!(&site.leads(written, from), leads, "{written}");
    }
    let named = |written: &str| match site.leads(written, from) {
        Leads::Here(arrival) => (arrival.path, arrival.origin),
        Leads::Away => panic!("{written} leads away"),
    };
    assert_eq!(
        named("https://EXAMPLE.org/about/#team"),
        ("/about/".to_owned(), Some("https://EXAMPLE.org".to_owned()))
    );
    assert_eq!(
        named("http://www.example.org"),
        ("/".to_owned(), Some("http://www.example.org".to_owned()))
    );
    assert_eq!(
        named("//example.org/a"),
        ("/a".to_owned(), Some("//example.org".to_owned()))
    );
}

/// The site `yaml` describes, built for one run at `address`.
fn site_at(yaml: &str, address: &str) -> Site {
    let mut settings = settings(yaml);
    settings.at(address);
    let files = ["index.html", "posts/post-1/index.html"];
    Site::new(&settings, files.iter().map(|file| (*file).to_owned()))
}

#[test]
fn a_link_to_the_sites_own_address_leads_into_a_site_built_at_another() {
    let home = |path: &str| {
        Leads::Here(Arrival {
            path: path.to_owned(),
            fragment: None,
            origin: Some("https://example.org".to_owned()),
            outside: false,
            home: true,
        })
    };
    let yaml = "url: https://example.org/blog/\nother_hosts: [www.example.org]\n";
    // Each address a run is given: on another host, at its top and under
    // a path, and on the site's own host.
    let table: &[(&str, &str, &str)] = &[
        ("https://preview.example.org/", "/", "/posts/post-1/"),
        (
            "https://preview.example.org/42/",
            "/42/",
            "/42/posts/post-1/",
        ),
        (
            "https://example.org/preview/",
            "/preview/",
            "/preview/posts/post-1/",
        ),
    ];
    for (address, top, from) in table {
        let site = site_at(yaml, address);
        assert_eq!(site.base(), top.trim_end_matches('/'), "{address}");
        assert_eq!(site.leads("../", from), here("/posts/", None), "{address}");
        assert_eq!(site.leads(top, from), here("/", None), "{address}");
        assert_eq!(
            site.leads("https://example.org/blog/posts/post-1/", from),
            home("/posts/post-1/"),
            "{address}"
        );
        assert_eq!(site.leads("https://example.org/blog", from), home("/"));
        let Leads::Here(elsewhere) = site.leads("https://example.org/video/", from) else {
            panic!("{address}: the site's own host leads away");
        };
        assert!(elsewhere.outside && !elsewhere.home, "{address}");
        let Leads::Here(also) = site.leads("https://www.example.org/blog/posts/", from) else {
            panic!("{address}: a host the site is also at leads away");
        };
        assert_eq!(
            (also.path.as_str(), also.home),
            ("/posts/", true),
            "{address}"
        );
        assert_eq!(site.base_of(&also), "/blog", "{address}");
        assert_eq!(site.leads("https://other.example/blog/", from), Leads::Away);
    }
    // A site with no address of its own has none to be named by.
    let site = site_at("title: Site\n", "https://preview.example.org/42/");
    assert_eq!(
        site.leads("/42/posts/post-1/", "/42/"),
        here("/posts/post-1/", None)
    );
    assert_eq!(site.leads("https://example.org/", "/42/"), Leads::Away);
}

/// Under a site path, a link elsewhere on the host is somewhere the site
/// publishes nothing; one `served_elsewhere` names, or into what an `after_build`
/// hook makes, is left alone.
#[test]
fn a_site_under_a_path_owns_only_that_path() {
    let site = site(
        "url: https://example.org/blog/\nserved_elsewhere: [/video/]\nafter_build: echo\n\
         after_build_outputs: [pagefind/]\n",
    );
    let from = "/blog/posts/post-1/";
    assert_eq!(site.leads("/blog/", from), here("/", None));
    assert_eq!(site.leads("/blog", from), here("/", None));
    assert_eq!(site.leads("../../about/", from), here("/about/", None));
    assert_eq!(site.leads("/video/talk/", from), Leads::Away);
    assert_eq!(
        site.leads("/videos/", from),
        Leads::Here(Arrival {
            path: "/videos/".into(),
            fragment: None,
            origin: None,
            outside: true,
            home: false,
        })
    );
    assert_eq!(site.leads("/blog/pagefind/ui.js", from), Leads::Away);
    assert_eq!(site.nearest("/posts/post-1", None), Some("/posts/post-1/"));
}

#[test]
fn an_address_without_its_last_slash_reaches_a_folder_page() {
    let site = site("");
    let file = |path: &str| site.file_at(path);
    assert_eq!(file("/"), Some("index.html".to_owned()));
    assert_eq!(
        file("/posts/post-1/"),
        Some("posts/post-1/index.html".to_owned())
    );
    assert_eq!(
        file("/posts/post-1"),
        Some("posts/post-1/index.html".to_owned())
    );
    assert_eq!(
        file("/posts/post-1/photo-1.jpg"),
        Some("posts/post-1/photo-1.jpg".to_owned())
    );
    assert_eq!(file("/Posts/post-1/"), None);
    assert_eq!(file("/posts/"), None);
    assert_eq!(site.nearest("/Posts/post-1/", None), Some("/posts/post-1/"));
    assert_eq!(
        site.nearest("/elsewhere/photo-1.jpg", None),
        Some("/posts/post-1/photo-1.jpg")
    );
}

#[test]
fn the_nearest_address_is_another_pages_and_the_same_last_piece_first() {
    let files = [
        "index.html",
        "post-1/index.html",
        "post-3/index.html",
        "posts/post-2/index.html",
        "posts/post-21/index.html",
        "notes/post-4/index.html",
        "posts/post-4/index.html",
        "tags/post-5/index.html",
    ];
    let files = files.iter().map(|file| (*file).to_owned());
    let site = Site::new(&settings(""), files).listing(["tags/post-5/index.html".to_owned()]);
    let table = [
        // The page the link is on is never offered.
        ("/post-2/", "post-1/index.html", Some("/posts/post-2/")),
        ("/post-9/", "post-1/index.html", Some("/post-3/")),
        ("/post-9/", "post-3/index.html", Some("/post-1/")),
        // The same last piece, published once, before a nearer spelling.
        ("/post-2/", "index.html", Some("/posts/post-2/")),
        ("/posts/post-22/", "index.html", Some("/posts/post-2/")),
        ("/POST-2", "index.html", Some("/posts/post-2/")),
        // Published twice, it is weighed as any other.
        ("/post-4/", "index.html", Some("/post-1/")),
        (
            "/notes/post-4/",
            "notes/post-4/index.html",
            Some("/posts/post-4/"),
        ),
        // A list's own page is weighed as any other.
        ("/post-5/", "index.html", Some("/post-1/")),
        ("/tags/post-55/", "index.html", Some("/tags/post-5/")),
        // The nearest spelling, as before.
        ("/post-11/", "index.html", Some("/post-1/")),
        ("/elsewhere/", "index.html", None),
    ];
    for (path, from, nearest) in table {
        assert_eq!(
            site.nearest(path, Some(from)),
            nearest,
            "{path} from {from}"
        );
    }
}

#[test]
fn a_page_is_read_for_its_links_and_ids_as_a_browser_reads_it() {
    let html = "<!doctype html><base href=\"/posts/\">\
        <h2 id=\"one\">One</h2><a name=\"two\"></a><p id='three&amp;four'>\
        <a href=\"/a/?x=1&amp;y=2\" class=\"next\">A</a>\
        <img src=\"b.jpg\" srcset=\"b-1.jpg 1x, b,2.jpg 2x,c.jpg\">\
        <video poster=\"d.jpg\"></video><a href=\"\">empty</a>\
        <!-- <a href=\"/commented/\" id=\"gone\"> -->\
        <script>document.write('<a href=\"/scripted/\">')</script>\
        <textarea><a href=\"/typed/\"></textarea>";
    let scanned = scan(html);
    assert_eq!(scanned.ids, ["one", "three&four", "two"]);
    assert_eq!(scanned.base.as_deref(), Some("/posts/"));
    let links: Vec<(Attribute, String, String)> = scanned
        .links
        .iter()
        .map(|link| (link.attribute, link.raw.clone(), link.address()))
        .collect();
    let link =
        |attribute, raw: &str, address: &str| (attribute, raw.to_owned(), address.to_owned());
    assert_eq!(
        links,
        [
            link(Attribute::Href, "/a/?x=1&amp;y=2", "/a/?x=1&y=2"),
            link(Attribute::Src, "b.jpg", "b.jpg"),
            link(Attribute::Srcset, "b-1.jpg", "b-1.jpg"),
            link(Attribute::Srcset, "b,2.jpg", "b,2.jpg"),
            link(Attribute::Srcset, "c.jpg", "c.jpg"),
            link(Attribute::Poster, "d.jpg", "d.jpg"),
        ]
    );
    // Each where it is on the page, a candidate of a `srcset` too.
    for link in &scanned.links {
        assert_eq!(&html[link.at..link.at + link.raw.len()], link.raw);
    }
    let srcset = html.find("srcset=\"").unwrap() + "srcset=\"".len();
    assert!(scanned.links[2..5].iter().all(|link| link.value == srcset));
}

#[test]
fn an_address_inside_the_site_is_spelled_as_what_stands_for_it_takes_it() {
    assert_eq!(Helper::Call.spelled("/about/#team"), "@url(/about/#team)");
    assert_eq!(Helper::Call.spelled("/a (b)/"), "@url(\"/a (b)/\")");
    assert_eq!(Helper::Call.spelled("/a,b/"), "@url(\"/a,b/\")");
    assert_eq!(
        Helper::Function.spelled("/about/"),
        "{{ url(\"/about/\") }}"
    );
}
