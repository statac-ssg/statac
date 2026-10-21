//! The server: the built site's files from disk, the live client added to
//! each page as it is answered, and the held request the client waits on.
//!
//! Nothing is written into the built files for any of it, so the folder
//! stays what a build would write.

use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, OnceLock, PoisonError, RwLock};
use std::time::Duration;

use super::live::Hub;
use super::server::Server;
use super::server::model::{Body, HeaderValue, Method, Request, Response, StatusCode, header};
use crate::content::address::on_the_wire;
use crate::html::escaped;
use crate::problem::{Kind, Problem, Reason, Unheard};
use crate::{platform, wording};

/// Where the server's own answers are, which no built file can be: Statac
/// keeps every name opening `.statac` at the top of a built site.
pub const LIVE: &str = "/.statac-live/";

const CLIENT: &str = include_str!("client.js");

/// What stands in the client for the panel's words, and for the ways it
/// offers to write a name, which serve puts in their places.
const WORDS: &str = "/* the panel's words */ {}";
const WAYS: &str = "/* the ways to write a name */ {}";

/// The client, with the panel's words and ways in it.
fn client() -> &'static str {
    static CLIENT_WITH_WORDS: OnceLock<String> = OnceLock::new();
    CLIENT_WITH_WORDS.get_or_init(|| {
        CLIENT
            .replacen(WORDS, super::panel::words(), 1)
            .replacen(WAYS, super::panel::ways(), 1)
    })
}

/// How long a request is held before it is answered with nothing new.
const LONGEST_WAIT: Duration = Duration::from_secs(25);

/// How many ports after the one asked for are tried where it is taken.
pub const NEXT_PORTS: u16 = 10;

/// What the server answers from.
pub struct Site {
    folder: PathBuf,
    hub: Arc<Hub>,
    /// The path the site is published under, as `/blog`, or empty.
    base: RwLock<String>,
    /// The port answered on, once one is being tried.
    port: AtomicU16,
}

impl Site {
    pub fn new(folder: PathBuf, hub: Arc<Hub>, base: String) -> Site {
        Site {
            folder,
            hub,
            base: RwLock::new(base),
            port: AtomicU16::new(0),
        }
    }

    pub fn base(&self) -> String {
        self.base
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub fn set_base(&self, base: String) {
        *self.base.write().unwrap_or_else(PoisonError::into_inner) = base;
    }
}

/// Answers for `site` on this machine only, at `port` or, where another
/// program has it, the first free port of the next few. Port 0 asks for
/// any free port. Gives the port taken. `unanswered` is handed what kept
/// the browser from being answered, once for each run of such faults.
pub fn listen(
    site: &Arc<Site>,
    port: u16,
    unanswered: impl Fn(&io::Error) + Send + Sync + 'static,
) -> Result<u16, Problem> {
    let unanswered: Arc<Unanswered> = Arc::new(unanswered);
    let cant = |port, last, why| Problem::nowhere(Kind::CantListen { port, last, why });
    let first = match port {
        0 => free_port().map_err(|error| cant(0, 0, Unheard::System(Reason::of(&error))))?,
        port => port,
    };
    let last = first.saturating_add(NEXT_PORTS);
    let mut trying = first;
    loop {
        match open(site, trying, &unanswered) {
            Ok(()) => return Ok(trying),
            Err(Refused::InUse) if trying < last => trying += 1,
            Err(Refused::InUse) => return Err(cant(first, last, Unheard::InUse)),
            Err(Refused::System(error)) => {
                return Err(cant(trying, trying, Unheard::System(Reason::of(&error))));
            }
        }
    }
}

enum Refused {
    InUse,
    System(io::Error),
}

type Unanswered = dyn Fn(&io::Error) + Send + Sync;

const LOOPBACKS: [IpAddr; 2] = [
    IpAddr::V6(Ipv6Addr::LOCALHOST),
    IpAddr::V4(Ipv4Addr::LOCALHOST),
];

fn open(site: &Arc<Site>, port: u16, unanswered: &Arc<Unanswered>) -> Result<(), Refused> {
    // A program listening on every address of the machine leaves these
    // two free to bind on some systems, and would still be the one a
    // browser reached by another name.
    let answered = LOOPBACKS.iter().any(|&ip| {
        TcpStream::connect_timeout(&SocketAddr::new(ip, port), Duration::from_millis(200)).is_ok()
    });
    if answered {
        return Err(Refused::InUse);
    }
    // The server binds its addresses in order and leaves the first
    // listening where the second fails, so each is tried here first.
    let mut addresses = Vec::new();
    for ip in LOOPBACKS {
        match TcpListener::bind(SocketAddr::new(ip, port)) {
            Ok(_) => addresses.push(SocketAddr::new(ip, port)),
            Err(error) if error.kind() == io::ErrorKind::AddrInUse => return Err(Refused::InUse),
            // A machine with no IPv6 answers on IPv4 alone.
            Err(_) if ip.is_ipv6() => {}
            Err(error) => return Err(Refused::System(error)),
        }
    }
    // Before the first request can arrive, which is held to it.
    site.port.store(port, Ordering::Relaxed);
    let site = site.clone();
    let unanswered = unanswered.clone();
    let mut server = Server::new(
        move |request| answer(&site, request),
        move |error| unanswered(error),
    )
    .with_global_timeout(Duration::from_secs(60))
    .with_max_concurrent_connections(512);
    for address in addresses {
        server = server.bind(address);
    }
    match server.spawn() {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AddrInUse => Err(Refused::InUse),
        Err(error) => Err(Refused::System(error)),
    }
}

fn free_port() -> io::Result<u16> {
    Ok(TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?
        .local_addr()?
        .port())
}

/// Whether `written`, a host and port as a request names them, is this
/// machine by one of its own names, at the port served on. Any other name
/// may be one a page elsewhere pointed at this machine to read the site
/// through the reader's browser.
fn own_host(written: &str, port: u16) -> bool {
    let Some((host, at)) = written.rsplit_once(':') else {
        return false;
    };
    let own = ["localhost", "127.0.0.1", "[::1]"]
        .iter()
        .any(|own| host.eq_ignore_ascii_case(own));
    own && at.parse() == Ok(port) && !at.starts_with('+')
}

fn answer(site: &Site, request: &Request<()>) -> Response<Body> {
    let port = site.port.load(Ordering::Relaxed);
    let named = |name: header::HeaderName| request.headers().get(name).map(HeaderValue::to_str);
    let host = matches!(named(header::HOST), Some(Ok(host)) if own_host(host, port));
    // A request may name the machine in its first line too, which then
    // stands for the header.
    let line = request
        .uri()
        .authority()
        .is_none_or(|authority| own_host(authority.as_str(), port));
    let live = request.uri().path().starts_with(LIVE);
    // A page of another site may not be told what the site's author is
    // doing. A browser says where a request came from in one of two
    // headers: `Origin` is left off a request for a script or a picture,
    // which `Sec-Fetch-Site` still marks. A request with neither comes
    // from no page at all.
    let origin = match named(header::ORIGIN) {
        Some(origin) if live => {
            matches!(origin, Ok(origin) if origin.strip_prefix("http://").is_some_and(|rest| own_host(rest, port)))
        }
        _ => true,
    };
    let fetched = match named(header::HeaderName::from_static("sec-fetch-site")) {
        Some(from) if live => matches!(from, Ok("same-origin")),
        _ => true,
    };
    if !(host && line && origin && fetched) {
        return respond(
            StatusCode::FORBIDDEN,
            "text/plain; charset=utf-8",
            wording::answers_only_at(&format!("http://localhost:{port}/")),
        );
    }
    let head = request.method() == Method::HEAD;
    if request.method() != Method::GET && !head {
        return plain(StatusCode::METHOD_NOT_ALLOWED, wording::READING_ONLY);
    }
    let path = request.uri().path();
    let query = request.uri().query().unwrap_or("");
    let mut response = match path.strip_prefix(LIVE) {
        Some("client.js") => respond(StatusCode::OK, "text/javascript; charset=utf-8", client()),
        Some("wait") => {
            let since = param(query, "since")
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
            let longest = param(query, "timeout")
                .and_then(|value| value.parse().ok())
                .map_or(LONGEST_WAIT, Duration::from_millis)
                .min(LONGEST_WAIT);
            // Asked by a page that shows whether a round is building, a
            // round starting is answered too.
            let building = param(query, "building").map(|building| building == "1");
            let reply = site.hub.wait(since, building, longest);
            respond(StatusCode::OK, "application/json", reply.to_json())
        }
        Some("panel") => {
            let page = param(query, "page").and_then(|page| page_file(site, page));
            let panel = site.hub.panel().to_json(page.as_deref());
            respond(StatusCode::OK, "application/json", panel)
        }
        Some("problems") => {
            let kind = param(query, "kind").map(|kind| unescaped(kind.as_bytes()));
            let kind = kind.as_deref().map(String::from_utf8_lossy);
            let problems = site.hub.panel().problems(kind.as_deref());
            respond(StatusCode::OK, "application/json", problems)
        }
        Some("search") => {
            // A form writes a space as `+`, and a `+` escaped.
            let words = param(query, "words").unwrap_or("").replace('+', " ");
            let words = String::from_utf8_lossy(&unescaped(words.as_bytes())).into_owned();
            let typing = param(query, "typing") == Some("1");
            let page = param(query, "page").and_then(|page| page_file(site, page));
            let found = site.hub.panel().search(&words, typing, page.as_deref());
            respond(StatusCode::OK, "application/json", found)
        }
        Some("waiting") => respond(
            StatusCode::OK,
            "text/plain; charset=utf-8",
            site.hub.waiting().to_string(),
        ),
        Some(_) => not_found(site, false),
        None => {
            let range = request
                .headers()
                .get(header::RANGE)
                .and_then(|value| value.to_str().ok());
            file(site, path, query, range)
        }
    };
    if head {
        *response.body_mut() = Body::empty();
    }
    response
}

fn file(site: &Site, path: &str, query: &str, range: Option<&str>) -> Response<Body> {
    let base = site.base();
    let within = match path.strip_prefix(base.as_str()) {
        Some(rest) if rest.is_empty() || rest.starts_with('/') => rest,
        // The top of the machine's address isn't the site's own top.
        _ if path == "/" => return redirect(StatusCode::FOUND, &format!("{base}/"), query),
        _ => return not_found(site, true),
    };
    let Some(relative) = decode(within) else {
        return plain(StatusCode::BAD_REQUEST, wording::ADDRESS_REFUSED);
    };
    let mut at = platform::join(&site.folder, &relative);
    if at.is_dir() {
        if !within.ends_with('/') {
            // Written from the path as read, never as sent: `//host` sent
            // back would send the browser to another machine.
            let slash = if relative.is_empty() { "" } else { "/" };
            let to = format!("{base}/{relative}{slash}");
            return redirect(StatusCode::MOVED_PERMANENTLY, &to, query);
        }
        at.push("index.html");
    }
    // Read before the file, so a round landing in between costs at most
    // one reload more, never one missed.
    let generation = site.hub.generation();
    let Ok(bytes) = std::fs::read(&at) else {
        return not_found(site, false);
    };
    let kind = content_type(&at, &bytes);
    if kind.starts_with("text/html") {
        return respond(StatusCode::OK, kind, inject(&bytes, generation));
    }
    let length = bytes.len() as u64;
    let mut response = match range.map(|range| ranged(range, length)) {
        None | Some(Ranged::Whole) => respond(StatusCode::OK, kind, bytes),
        Some(Ranged::Part(from, to)) => {
            let part = bytes[from as usize..=to as usize].to_vec();
            let mut response = respond(StatusCode::PARTIAL_CONTENT, kind, part);
            set(
                &mut response,
                header::CONTENT_RANGE,
                &format!("bytes {from}-{to}/{length}"),
            );
            response
        }
        Some(Ranged::Outside) => {
            let mut response = plain(StatusCode::RANGE_NOT_SATISFIABLE, "");
            set(
                &mut response,
                header::CONTENT_RANGE,
                &format!("bytes */{length}"),
            );
            response
        }
    };
    set(&mut response, header::ACCEPT_RANGES, "bytes");
    response
}

/// The file the page at `asked` is published as, from the top of the built
/// site. `asked` is the path of an address as a query carries it, escaped
/// once more. None where it names no file of the site.
fn page_file(site: &Site, asked: &str) -> Option<String> {
    let path = String::from_utf8(unescaped(asked.as_bytes())).ok()?;
    let base = site.base();
    let within = path.strip_prefix(base.as_str())?;
    if !(within.is_empty() || within.starts_with('/')) {
        return None;
    }
    let relative = decode(within)?;
    let is_file = |name: &str| name.ends_with(".html") || name.ends_with(".htm");
    let file = match (relative.is_empty(), within.ends_with('/')) {
        (true, _) => "index.html".to_owned(),
        (false, false) if is_file(&relative) => relative,
        (false, _) => format!("{relative}/index.html"),
    };
    platform::join(&site.folder, &file)
        .is_file()
        .then_some(file)
}

/// The answer where nothing is published: the site's own `404.html` where
/// it built one, a plain page otherwise. `outside` is whether the address
/// asked for is outside the path the site is published under.
fn not_found(site: &Site, outside: bool) -> Response<Body> {
    let generation = site.hub.generation();
    let page = match std::fs::read(site.folder.join("404.html")) {
        Ok(page) => page,
        Err(_) => plain_page(&site.base(), outside).into_bytes(),
    };
    respond(
        StatusCode::NOT_FOUND,
        "text/html; charset=utf-8",
        inject(&page, generation),
    )
}

/// Statac's own page for an address with nothing at it. It carries the
/// live client like any other, so a page made later at the address shows
/// without being asked for again.
fn plain_page(base: &str, outside: bool) -> String {
    let under = match outside && !base.is_empty() {
        true => {
            let top = escaped(&format!("{base}/"));
            let href = escaped(&on_the_wire(&format!("{base}/")));
            format!(
                "<p>{} <a href=\"{href}\">{top}</a>.</p>\n",
                wording::PUBLISHED_UNDER
            )
        }
        false => String::new(),
    };
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <meta name=\"color-scheme\" content=\"light dark\">\n\
         <title>{}</title>\n\
         <style>body{{font:1.125rem/1.5 system-ui,sans-serif;max-width:40rem;\
         margin:4rem auto;padding:0 1rem}}h1{{font-size:1.5rem}}</style>\n\
         </head>\n<body>\n<main>\n<h1>{}</h1>\n{under}</main>\n</body>\n</html>\n",
        wording::NOT_FOUND_TITLE,
        wording::NOT_FOUND,
    )
}

/// The page with the live client in its head, or wherever it can go.
fn inject(html: &[u8], generation: u64) -> Vec<u8> {
    let tag = format!("<script src=\"{LIVE}client.js\" data-live=\"{generation}\" defer></script>");
    let found = |end_tag: &[u8]| {
        html.windows(end_tag.len())
            .position(|window| window.eq_ignore_ascii_case(end_tag))
    };
    let at = found(b"</head>")
        .or_else(|| found(b"</body>"))
        .unwrap_or(html.len());
    let mut out = Vec::with_capacity(html.len() + tag.len());
    out.extend_from_slice(&html[..at]);
    out.extend_from_slice(tag.as_bytes());
    out.extend_from_slice(&html[at..]);
    out
}

/// The path of an address as a file's path from the top of the built site,
/// with forward slashes. Nothing where it could name a file anywhere else,
/// or isn't text: a piece that climbs out, and any name no built file has,
/// which covers a drive, a device and the other slash.
fn decode(path: &str) -> Option<String> {
    let raw = unescaped(path.as_bytes());
    let text = String::from_utf8(raw).ok()?;
    let mut pieces = Vec::new();
    for piece in text.split('/') {
        if piece.is_empty() || piece == "." {
            continue;
        }
        if piece == ".." || platform::unwritable(piece).is_some() {
            return None;
        }
        pieces.push(piece);
    }
    Some(pieces.join("/"))
}

/// `bytes` with each `%` and the two hex digits after it as the byte they
/// spell. A `%` that opens no byte is itself.
fn unescaped(bytes: &[u8]) -> Vec<u8> {
    let mut raw = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        let hex = |byte: u8| char::from(byte).to_digit(16);
        if bytes[at] == b'%'
            && let (Some(high), Some(low)) = (
                bytes.get(at + 1).copied().and_then(hex),
                bytes.get(at + 2).copied().and_then(hex),
            )
        {
            raw.push((high * 16 + low) as u8);
            at += 3;
        } else {
            raw.push(bytes[at]);
            at += 1;
        }
    }
    raw
}

/// What a `Range` asks of a file `length` bytes long.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Ranged {
    /// Nothing this server answers in part: the whole file is right.
    Whole,
    /// From one byte to another, both counted in.
    Part(u64, u64),
    /// Bytes the file doesn't have.
    Outside,
}

fn ranged(range: &str, length: u64) -> Ranged {
    let Some(spec) = range.trim().strip_prefix("bytes=") else {
        return Ranged::Whole;
    };
    // Several ranges at once would need an answer in parts.
    let Some((from, to)) = spec.split_once('-').filter(|_| !spec.contains(',')) else {
        return Ranged::Whole;
    };
    let number = |text: &str| text.trim().parse::<u64>().ok();
    let (from, to) = match (from.trim().is_empty(), to.trim().is_empty()) {
        // The last so many bytes.
        (true, false) => match number(to) {
            Some(0) => return Ranged::Outside,
            Some(last) => (length.saturating_sub(last), length.saturating_sub(1)),
            None => return Ranged::Whole,
        },
        (false, true) => match number(from) {
            Some(from) => (from, length.saturating_sub(1)),
            None => return Ranged::Whole,
        },
        (false, false) => match (number(from), number(to)) {
            (Some(from), Some(to)) if from <= to => (from, to.min(length.saturating_sub(1))),
            _ => return Ranged::Whole,
        },
        (true, true) => return Ranged::Whole,
    };
    match from < length {
        true => Ranged::Part(from, to),
        false => Ranged::Outside,
    }
}

fn param<'a>(query: &'a str, name: &str) -> Option<&'a str> {
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(key, _)| *key == name)
        .map(|(_, value)| value)
}

/// What a file is served as: a picture by what it holds, whatever it is
/// called, and any other file by its name's ending. An SVG is one only by
/// its name, since a browser runs what an SVG holds.
fn content_type(path: &Path, bytes: &[u8]) -> &'static str {
    use crate::pictures::Kind;
    let named = named_type(path);
    let pictures = [
        "image/png",
        "image/jpeg",
        "image/webp",
        "image/avif",
        "image/gif",
        "application/octet-stream",
    ];
    if !pictures.contains(&named) {
        return named;
    }
    match crate::pictures::begins_as(bytes) {
        Some(Kind::Jpeg) => "image/jpeg",
        Some(Kind::Png | Kind::AnimatedPng) => "image/png",
        Some(Kind::WebP | Kind::AnimatedWebP) => "image/webp",
        Some(Kind::Gif) => "image/gif",
        Some(Kind::Avif) => "image/avif",
        Some(Kind::Heic) => "image/heic",
        Some(Kind::Heif) => "image/heif",
        Some(Kind::Svg) | None => named,
    }
}

fn named_type(path: &Path) -> &'static str {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match extension.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" | "map" => "application/json",
        "webmanifest" => "application/manifest+json",
        "xml" => "application/xml",
        "atom" => "application/atom+xml",
        "rss" => "application/rss+xml",
        "txt" | "md" => "text/plain; charset=utf-8",
        "csv" => "text/csv; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "gif" => "image/gif",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "mp3" => "audio/mpeg",
        "m4a" => "audio/mp4",
        "ogg" | "oga" | "opus" => "audio/ogg",
        "wav" => "audio/wav",
        "flac" => "audio/flac",
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "vtt" => "text/vtt; charset=utf-8",
        "pdf" => "application/pdf",
        "wasm" => "application/wasm",
        "zip" => "application/zip",
        "gz" => "application/gzip",
        _ => "application/octet-stream",
    }
}

fn set(response: &mut Response<Body>, name: header::HeaderName, value: &str) {
    if let Ok(value) = HeaderValue::from_str(value) {
        response.headers_mut().insert(name, value);
    }
}

fn respond(status: StatusCode, kind: &'static str, body: impl Into<Body>) -> Response<Body> {
    let mut response = Response::new(body.into());
    *response.status_mut() = status;
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(kind));
    // Every reload must be given the new files, and a page a browser kept
    // would carry an old generation.
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    // No page of another site may embed an answer, or have it read as
    // another kind of file than it says it is.
    headers.insert(
        header::HeaderName::from_static("cross-origin-resource-policy"),
        HeaderValue::from_static("same-origin"),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}

fn plain(status: StatusCode, text: &'static str) -> Response<Body> {
    respond(status, "text/plain; charset=utf-8", text)
}

fn redirect(status: StatusCode, to: &str, query: &str) -> Response<Body> {
    let mut to = on_the_wire(to);
    if !query.is_empty() {
        to.push('?');
        to.push_str(query);
    }
    let mut response = plain(status, "");
    set(&mut response, header::LOCATION, &to);
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serve::live::{Status, Told};
    use crate::testing::Scratch;

    /// A few pages and files written to `scratch`, served under `base`.
    fn serving(scratch: &Scratch, base: &str) -> Site {
        scratch.write("index.html", "<html><head></head><body>home</body></html>");
        scratch.write("posts/post-1/index.html", "<p>post 1</p>");
        scratch.write("site.1a2b3c4d.css", "body { margin: 0 }");
        scratch.write("notes.txt", "0123456789");
        let site = Site::new(scratch.0.clone(), Arc::new(Hub::new()), base.into());
        site.port.store(3000, Ordering::Relaxed);
        site
    }

    struct Answer {
        status: u16,
        kind: String,
        location: Option<String>,
        content_range: Option<String>,
        body: String,
    }

    fn asked(site: &Site, method: Method, target: &str, range: Option<&str>) -> Answer {
        let mut headers = vec![(header::HOST, "localhost:3000")];
        headers.extend(range.map(|range| (header::RANGE, range)));
        sent(site, method, target, &headers)
    }

    fn sent(
        site: &Site,
        method: Method,
        target: &str,
        headers: &[(header::HeaderName, &str)],
    ) -> Answer {
        let mut request = Request::builder().method(method).uri(target);
        for (name, value) in headers {
            request = request.header(name, *value);
        }
        let response = answer(site, &request.body(()).unwrap());
        let text = |name: header::HeaderName| {
            response
                .headers()
                .get(name)
                .map(|value| value.to_str().unwrap().to_owned())
        };
        assert_eq!(text(header::CACHE_CONTROL).as_deref(), Some("no-store"));
        assert_eq!(
            text(header::HeaderName::from_static(
                "cross-origin-resource-policy"
            ))
            .as_deref(),
            Some("same-origin")
        );
        assert_eq!(
            text(header::X_CONTENT_TYPE_OPTIONS).as_deref(),
            Some("nosniff")
        );
        Answer {
            status: response.status().as_u16(),
            kind: text(header::CONTENT_TYPE).unwrap(),
            location: text(header::LOCATION),
            content_range: text(header::CONTENT_RANGE),
            body: response.into_body().to_string().unwrap(),
        }
    }

    fn get(site: &Site, target: &str) -> Answer {
        asked(site, Method::GET, target, None)
    }

    fn tag(site: &Site) -> String {
        format!(
            "<script src=\"/.statac-live/client.js\" data-live=\"{}\" defer></script>",
            site.hub.generation()
        )
    }

    #[test]
    fn only_a_request_for_this_machine_by_its_own_name_is_answered() {
        let scratch = Scratch::new("hosts");
        let site = serving(&scratch, "");
        let with = |target: &str, headers: &[(header::HeaderName, &str)]| {
            sent(&site, Method::GET, target, headers)
        };
        for host in [
            "localhost:3000",
            "LocalHost:3000",
            "127.0.0.1:3000",
            "[::1]:3000",
        ] {
            assert_eq!(with("/", &[(header::HOST, host)]).status, 200, "{host}");
        }
        for host in [
            "example.org",
            "example.org:3000",
            "localhost.example.org:3000",
            "localhost:3001",
            "localhost",
            "127.0.0.1",
            "127.0.0.2:3000",
            "[::1]",
            "localhost:+3000",
            "",
        ] {
            let refused = with("/", &[(header::HOST, host)]);
            assert_eq!(refused.status, 403, "{host}");
            assert_eq!(
                refused.body,
                "statac serve answers only at http://localhost:3000/"
            );
        }
        assert_eq!(with("/", &[]).status, 403, "no host");
        // The first line's name stands for the header's.
        assert_eq!(
            with(
                "http://example.org:3000/",
                &[(header::HOST, "localhost:3000")]
            )
            .status,
            403
        );
        assert_eq!(
            with(
                "http://localhost:3000/",
                &[(header::HOST, "localhost:3000")]
            )
            .status,
            200
        );
    }

    #[test]
    fn another_sites_page_is_told_nothing_of_the_rounds() {
        let scratch = Scratch::new("origins");
        let site = serving(&scratch, "");
        let host = (header::HOST, "localhost:3000");
        let from = |target: &str, origin: &str| {
            sent(
                &site,
                Method::GET,
                target,
                &[host.clone(), (header::ORIGIN, origin)],
            )
            .status
        };
        let wait = "/.statac-live/wait?since=0&timeout=0";
        for own in [
            "http://localhost:3000",
            "http://127.0.0.1:3000",
            "http://[::1]:3000",
        ] {
            assert_eq!(from(wait, own), 200, "{own}");
        }
        for foreign in [
            "https://example.org",
            "http://example.org:3000",
            "http://localhost:3001",
            "https://localhost:3000",
            "null",
        ] {
            assert_eq!(from(wait, foreign), 403, "{foreign}");
            assert_eq!(from("/.statac-live/client.js", foreign), 403, "{foreign}");
            assert_eq!(from("/.statac-live/waiting", foreign), 403, "{foreign}");
            assert_eq!(
                from("/.statac-live/panel?page=/", foreign),
                403,
                "{foreign}"
            );
            assert_eq!(from("/.statac-live/problems", foreign), 403, "{foreign}");
            assert_eq!(
                from("/.statac-live/search?words=a", foreign),
                403,
                "{foreign}"
            );
        }
        assert_eq!(
            sent(&site, Method::GET, wait, std::slice::from_ref(&host)).status,
            200
        );
        // A file of the site is answered, for the browser to hold back.
        assert_eq!(from("/site.1a2b3c4d.css", "https://example.org"), 200);
    }

    #[test]
    fn a_request_a_browser_marks_as_another_sites_is_refused() {
        let scratch = Scratch::new("fetched");
        let site = serving(&scratch, "");
        let marked = |target: &str, from: &str| {
            let headers = [
                (header::HOST, "localhost:3000"),
                (header::HeaderName::from_static("sec-fetch-site"), from),
            ];
            sent(&site, Method::GET, target, &headers)
        };
        let wait = "/.statac-live/wait?since=0&timeout=0";
        for target in [
            wait,
            "/.statac-live/client.js",
            "/.statac-live/waiting",
            "/.statac-live/panel?page=%2F",
            "/.statac-live/problems?kind=broken-link",
            "/.statac-live/search?words=a&typing=1&page=%2F",
        ] {
            for from in ["cross-site", "same-site", "none", ""] {
                assert_eq!(marked(target, from).status, 403, "{target} {from}");
            }
            assert_eq!(marked(target, "same-origin").status, 200, "{target}");
            assert_eq!(get(&site, target).status, 200, "{target}");
        }
        assert_eq!(marked(wait, "same-origin").kind, "application/json");
        // A page is a reader's to open from anywhere.
        assert_eq!(marked("/", "cross-site").status, 200);
        assert_eq!(marked("/", "none").status, 200);
    }

    #[test]
    fn the_client_goes_in_the_head_or_wherever_it_can() {
        let injected = |html: &str| String::from_utf8(inject(html.as_bytes(), 7)).unwrap();
        let tag = "<script src=\"/.statac-live/client.js\" data-live=\"7\" defer></script>";
        assert_eq!(
            injected("<html><HEAD><title>x</title></HEAD><body></body></html>"),
            format!("<html><HEAD><title>x</title>{tag}</HEAD><body></body></html>")
        );
        assert_eq!(
            injected("<body><p>x</p></body>"),
            format!("<body><p>x</p>{tag}</body>")
        );
        assert_eq!(injected("<p>x</p>"), format!("<p>x</p>{tag}"));
        // Bytes that aren't text are passed on as they are.
        let odd = inject(b"\xff</head>", 7);
        assert!(odd.starts_with(b"\xff<script") && odd.ends_with(b"</head>"));
    }

    #[test]
    fn addresses_that_could_name_a_file_elsewhere_are_refused() {
        assert_eq!(decode("/a/b.html").as_deref(), Some("a/b.html"));
        assert_eq!(decode("/a%20b.html").as_deref(), Some("a b.html"));
        assert_eq!(decode("/caf%C3%A9/").as_deref(), Some("caf\u{e9}"));
        assert_eq!(decode("//a/./b/").as_deref(), Some("a/b"));
        assert_eq!(decode("/").as_deref(), Some(""));
        assert_eq!(
            decode("/.well-known/security.txt").as_deref(),
            Some(".well-known/security.txt")
        );
        // A `%` that opens no byte is itself.
        assert_eq!(decode("/a%").as_deref(), Some("a%"));
        assert_eq!(decode("/100%zz").as_deref(), Some("100%zz"));
        for refused in [
            "/../x",
            "/a/../../x",
            "/%2e%2e/x",
            "/%2E%2E%2Fx",
            "/a%5c..%5cx",
            "/a\\..\\x",
            "/c:/windows/win.ini",
            "/C%3A/x",
            "/a%00.html",
            "/con",
            "/NUL.txt",
            "/%ff",
        ] {
            assert_eq!(decode(refused), None, "{refused}");
        }
    }

    #[test]
    fn a_refused_address_is_answered_400() {
        let scratch = Scratch::new("refused");
        let site = serving(&scratch, "");
        for target in ["/%2e%2e/secret.txt", "/a%5cb", "/posts/..%2f..%2fx"] {
            let answer = get(&site, target);
            assert_eq!(answer.status, 400, "{target}");
            assert_eq!(answer.body, "This address can't name a file of the site.");
        }
    }

    #[test]
    fn pages_are_answered_with_the_client_and_other_files_as_they_are() {
        let scratch = Scratch::new("files");
        let site = serving(&scratch, "");
        let home = get(&site, "/");
        assert_eq!(
            (home.status, home.kind.as_str()),
            (200, "text/html; charset=utf-8")
        );
        assert_eq!(
            home.body,
            format!("<html><head>{}</head><body>home</body></html>", tag(&site))
        );
        assert_eq!(
            get(&site, "/index.html?x=1").body,
            get(&site, "/").body,
            "a query names the same file"
        );
        let post = get(&site, "/posts/post-1/");
        assert_eq!(post.body, format!("<p>post 1</p>{}", tag(&site)));
        let css = get(&site, "/site.1a2b3c4d.css");
        assert_eq!(
            (css.status, css.kind.as_str(), css.body.as_str()),
            (200, "text/css; charset=utf-8", "body { margin: 0 }")
        );
        // What is on disk is what a build wrote.
        assert_eq!(
            std::fs::read_to_string(scratch.0.join("index.html")).unwrap(),
            "<html><head></head><body>home</body></html>"
        );
    }

    #[test]
    fn a_folder_asked_for_without_its_slash_is_sent_to_it() {
        let scratch = Scratch::new("slash");
        let site = serving(&scratch, "");
        scratch.write("caf\u{e9} 1/index.html", "<p>x</p>");
        let sent = get(&site, "/posts/post-1?page=2");
        assert_eq!(sent.status, 301);
        assert_eq!(sent.location.as_deref(), Some("/posts/post-1/?page=2"));
        assert_eq!(
            get(&site, "/caf%C3%A9%201").location.as_deref(),
            Some("/caf%C3%A9%201/")
        );
        // Never to another machine, however the address was written.
        let sent = get(&site, "//posts/post-1");
        assert_eq!(sent.location.as_deref(), Some("/posts/post-1/"));
    }

    #[test]
    fn a_site_under_a_path_is_served_under_it() {
        let scratch = Scratch::new("under");
        let site = serving(&scratch, "/blog");
        assert_eq!(get(&site, "/blog/posts/post-1/").status, 200);
        assert_eq!(get(&site, "/blog/site.1a2b3c4d.css").status, 200);
        let top = get(&site, "/blog");
        assert_eq!((top.status, top.location.as_deref()), (301, Some("/blog/")));
        let root = get(&site, "/");
        assert_eq!(
            (root.status, root.location.as_deref()),
            (302, Some("/blog/"))
        );
        let outside = get(&site, "/posts/post-1/");
        assert_eq!(outside.status, 404);
        assert!(
            outside
                .body
                .contains("<p>The site is published under <a href=\"/blog/\">/blog/</a>.</p>"),
            "{}",
            outside.body
        );
        assert_eq!(get(&site, "/blogs/").status, 404);
        // The server's own answers stay at the top.
        assert_eq!(get(&site, "/.statac-live/client.js").status, 200);
        site.set_base(String::new());
        assert_eq!(get(&site, "/posts/post-1/").status, 200);
    }

    #[test]
    fn a_miss_is_answered_with_a_plain_page_or_the_sites_own() {
        let scratch = Scratch::new("miss");
        let site = serving(&scratch, "");
        let miss = get(&site, "/posts/post-2/");
        assert_eq!(
            (miss.status, miss.kind.as_str()),
            (404, "text/html; charset=utf-8")
        );
        assert!(miss.body.contains("<title>Not found</title>"));
        assert!(
            miss.body
                .contains("<h1>Nothing is published at this address.</h1>")
        );
        assert!(!miss.body.contains("published under"));
        assert!(miss.body.contains(&format!("{}</head>", tag(&site))));
        assert_eq!(get(&site, "/.statac-live/other").status, 404);

        scratch.write("404.html", "<html><head></head><body>Lost</body></html>");
        let miss = get(&site, "/posts/post-2/");
        assert_eq!(miss.status, 404);
        assert_eq!(
            miss.body,
            format!("<html><head>{}</head><body>Lost</body></html>", tag(&site))
        );
    }

    #[test]
    fn part_of_a_file_is_answered_where_one_range_asks() {
        assert_eq!(ranged("bytes=0-3", 10), Ranged::Part(0, 3));
        assert_eq!(ranged("bytes=4-", 10), Ranged::Part(4, 9));
        assert_eq!(ranged("bytes=-3", 10), Ranged::Part(7, 9));
        assert_eq!(ranged("bytes=-30", 10), Ranged::Part(0, 9));
        assert_eq!(ranged("bytes=2-99", 10), Ranged::Part(2, 9));
        assert_eq!(ranged("bytes=10-", 10), Ranged::Outside);
        assert_eq!(ranged("bytes=-0", 10), Ranged::Outside);
        assert_eq!(ranged("bytes=0-", 0), Ranged::Outside);
        for whole in [
            "bytes=0-1,4-5",
            "bytes=5-2",
            "bytes=a-b",
            "bytes=-",
            "lines=1-2",
        ] {
            assert_eq!(ranged(whole, 10), Ranged::Whole, "{whole}");
        }

        let scratch = Scratch::new("ranges");
        let site = serving(&scratch, "");
        let part = asked(&site, Method::GET, "/notes.txt", Some("bytes=2-5"));
        assert_eq!((part.status, part.body.as_str()), (206, "2345"));
        assert_eq!(part.content_range.as_deref(), Some("bytes 2-5/10"));
        let outside = asked(&site, Method::GET, "/notes.txt", Some("bytes=50-"));
        assert_eq!(outside.status, 416);
        assert_eq!(outside.content_range.as_deref(), Some("bytes */10"));
        // A page is answered whole: the client added to it moves every byte.
        let page = asked(&site, Method::GET, "/", Some("bytes=0-3"));
        assert_eq!(page.status, 200);
    }

    #[test]
    fn only_reading_is_answered() {
        let scratch = Scratch::new("methods");
        let site = serving(&scratch, "");
        let post = asked(&site, Method::POST, "/", None);
        assert_eq!(
            (post.status, post.body.as_str()),
            (405, "Only GET and HEAD are answered.")
        );
        let head = asked(&site, Method::HEAD, "/notes.txt", None);
        assert_eq!(
            (head.status, head.kind.as_str(), head.body.as_str()),
            (200, "text/plain; charset=utf-8", "")
        );
        for target in [
            "/.statac-live/panel?page=%2F",
            "/.statac-live/problems",
            "/.statac-live/search?words=a",
        ] {
            assert_eq!(asked(&site, Method::POST, target, None).status, 405);
            let head = asked(&site, Method::HEAD, target, None);
            assert_eq!(
                (head.status, head.kind.as_str(), head.body.as_str()),
                (200, "application/json", "")
            );
        }
    }

    #[test]
    fn a_wait_is_answered_with_what_happened_since() {
        let scratch = Scratch::new("wait");
        let site = serving(&scratch, "");
        let since = site.hub.generation();
        let nothing = get(
            &site,
            &format!("/.statac-live/wait?since={since}&timeout=0"),
        );
        assert_eq!(nothing.kind, "application/json");
        assert!(
            nothing.body.contains("\"kind\":\"none\"")
                && nothing.body.contains("\"building\":false"),
            "{}",
            nothing.body
        );
        // A page that knows a round to be building is answered once none is.
        let asked = std::time::Instant::now();
        let told = get(
            &site,
            &format!("/.statac-live/wait?since={since}&timeout=20000&building=1"),
        );
        assert!(told.body.contains("\"building\":false"), "{}", told.body);
        assert!(asked.elapsed() < Duration::from_secs(10));
        site.hub.publish(
            Told::Reload,
            vec!["content/post-1.md".into()],
            Status::default(),
        );
        let reload = get(&site, &format!("/.statac-live/wait?since={since}"));
        assert!(
            reload.body.contains("\"kind\":\"reload\"")
                && reload.body.contains("\"changed\":[\"content/post-1.md\"]"),
            "{}",
            reload.body
        );
        assert_eq!(get(&site, "/.statac-live/waiting").body, "0");
    }

    #[test]
    fn the_panel_is_asked_for_by_the_address_of_the_page_it_is_on() {
        let scratch = Scratch::new("panel");
        let site = serving(&scratch, "/blog");
        scratch.write("caf\u{e9} 1/index.html", "<p>x</p>");
        for (asked, file) in [
            ("%2Fblog%2F", Some("index.html")),
            ("/blog", Some("index.html")),
            ("%2Fblog%2Findex.html", Some("index.html")),
            (
                "%2Fblog%2Fposts%2Fpost-1%2F",
                Some("posts/post-1/index.html"),
            ),
            ("%2Fblog%2Fposts%2Fpost-1", Some("posts/post-1/index.html")),
            // Escaped once as an address is, and once more for the query.
            (
                "%2Fblog%2Fcaf%25C3%25A9%25201%2F",
                Some("caf\u{e9} 1/index.html"),
            ),
            ("%2Fblog%2Fposts%2Fpost-2%2F", None),
            ("%2Fblog%2Fnotes.txt", None),
            ("%2Fposts%2Fpost-1%2F", None),
            ("%2Fblogs%2F", None),
            ("%2Fblog%2F..%2F..%2Fsecret%2F", None),
            ("%2Fblog%2F%252e%252e%2Fsecret%2F", None),
            ("%ff", None),
        ] {
            assert_eq!(page_file(&site, asked).as_deref(), file, "{asked}");
        }
        let answer = get(
            &site,
            "/.statac-live/panel?page=%2Fblog%2Fposts%2Fpost-1%2F",
        );
        assert_eq!(
            (answer.status, answer.kind.as_str()),
            (200, "application/json")
        );
        // Nothing has been built yet for there to be anything to say.
        assert_eq!(
            answer.body,
            "{\"v\":3,\"problems\":[],\"stopped\":null,\"explanations\":{},\"round\":null,\
             \"rounds\":[],\
             \"site\":{\"kinds\":[],\"gone\":null,\"drafts\":[],\"waiting\":[],\"written\":[]},\
             \"root\":null,\"page\":null,\"names\":[]}"
        );
        let answer = get(&site, "/.statac-live/search?words=post");
        assert_eq!(answer.body, "{\"v\":3,\"drafts\":false,\"languages\":null}");
    }

    #[test]
    fn the_client_is_handed_every_word_the_panel_puts_together() {
        let scratch = Scratch::new("words");
        let site = serving(&scratch, "");
        let client = get(&site, "/.statac-live/client.js");
        assert_eq!(client.kind, "text/javascript; charset=utf-8");
        assert!(!client.body.contains(WORDS), "the words stand in its place");
        let ways = format!("const ways = {};", super::super::panel::ways());
        assert!(client.body.contains(&ways), "{}", client.body);
        assert!(
            client.body.contains(
                "const words = {\"name\":[{\"text\":\"Statac\"}],\
                 \"building\":[{\"text\":\"Building\"}],"
            ) && client.body.contains(
                "\"warnings\":{\"one\":[{\"text\":\"{n} warning\"}],\
                 \"many\":[{\"text\":\"{n} warnings\"}]},"
            ) && client.body.contains(
                "\"layoutFrom\":[{\"text\":\"Layout \"},{\"code\":\"{name}\"},\
                 {\"text\":\", from \"},{\"code\":\"{file}\"}],"
            ),
            "{}",
            client.body
        );
        // Every word the client asks for by its name is one it is handed.
        let words = super::super::panel::words();
        for asking in ["told(\"", "say(\"", "woven(\""] {
            for (at, _) in CLIENT.match_indices(asking) {
                let name = &CLIENT[at + asking.len()..];
                let name = &name[..name.find('"').unwrap()];
                assert!(words.contains(&format!("\"{name}\":")), "{name}");
            }
        }
        // And every editor the settings offer.
        let offered = CLIENT.match_indices("\", \"editor").count();
        assert!(offered >= 6, "{offered}");
        for (at, _) in CLIENT.match_indices("\", \"editor") {
            let name = &CLIENT[at + 4..];
            let name = &name[..name.find('"').unwrap()];
            assert!(words.contains(&format!("\"{name}\":")), "{name}");
        }
    }

    #[test]
    fn the_sites_problems_are_asked_for_of_one_kind_or_every_kind() {
        let scratch = Scratch::new("problems");
        let site = serving(&scratch, "/blog");
        for target in [
            "/.statac-live/problems",
            "/.statac-live/problems?kind=broken-link",
            "/.statac-live/problems?kind=broken%2Dlink",
            "/.statac-live/problems?kind=%ff",
        ] {
            let answer = get(&site, target);
            assert_eq!(
                (answer.status, answer.kind.as_str()),
                (200, "application/json"),
                "{target}"
            );
            // Nothing has been built yet for there to be any.
            assert_eq!(
                answer.body, "{\"v\":3,\"problems\":[],\"more\":0,\"explanations\":{}}",
                "{target}"
            );
        }
    }

    #[test]
    fn files_are_typed_by_their_names() {
        for (name, kind) in [
            ("index.html", "text/html; charset=utf-8"),
            ("feed.xml", "application/xml"),
            ("photo-1.AVIF", "image/avif"),
            ("font-1.woff2", "font/woff2"),
            ("site.webmanifest", "application/manifest+json"),
            ("LICENCE", "application/octet-stream"),
        ] {
            assert_eq!(content_type(Path::new(name), b""), kind, "{name}");
        }
    }

    #[test]
    fn a_picture_is_typed_by_what_it_holds_whatever_it_is_called() {
        let jpeg: &[u8] = b"\xff\xd8\xff\xe0";
        let png: &[u8] = b"\x89PNG\r\n\x1a\n";
        let svg: &[u8] = b"<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>";
        for (name, bytes, kind) in [
            ("photo-1.gif", jpeg, "image/jpeg"),
            ("photo-1.jfif", jpeg, "image/jpeg"),
            ("photo-1", png, "image/png"),
            ("photo-1.jpg", b"GIF89a".as_slice(), "image/gif"),
            // A file that begins as no picture keeps what its name says.
            ("photo-1.png", b"words".as_slice(), "image/png"),
            // What is no picture by its name is never read as one, and an
            // SVG is one only by its name.
            ("page.html", jpeg, "text/html; charset=utf-8"),
            ("notes.txt", png, "text/plain; charset=utf-8"),
            ("drawing.png", svg, "image/png"),
            ("drawing", svg, "application/octet-stream"),
        ] {
            assert_eq!(content_type(Path::new(name), bytes), kind, "{name}");
        }
    }

    #[test]
    fn a_taken_port_gives_way_to_the_next() {
        let scratch = Scratch::new("ports");
        let site = Arc::new(serving(&scratch, ""));
        let taken = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let asked = taken.local_addr().unwrap().port();
        let port = listen(&site, asked, |_| {}).unwrap();
        assert!(port > asked && port <= asked + NEXT_PORTS, "{asked} {port}");
    }
}
