mod scratch;

use scratch::Scratch;
use std::process::Command;

fn statac(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_statac"))
        .args(args)
        .env("NO_COLOR", "1")
        .output()
        .unwrap()
}

#[test]
fn a_mistake_on_the_command_line_exits_with_2_on_standard_error() {
    let output = statac(&["biuld"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let said = String::from_utf8(output.stderr).unwrap();
    assert!(
        said.starts_with("error  Unknown command: `biuld`\n"),
        "{said}"
    );
}

#[test]
fn a_mistake_in_the_address_for_a_run_is_a_mistake_on_the_command_line() {
    let folder = scratch("address");
    made(&folder);
    let site = folder.to_string_lossy();
    for (command, option, says) in [
        (
            "build",
            "--url=preview.example.org",
            "error  Wrong kind of value: `--url` is a full URL, such as `https://example.org`, \
             not `preview.example.org`\n  Write it as `https://preview.example.org`.\n",
        ),
        (
            "check",
            "--url=mailto:someone@example.org",
            "error  Wrong kind of value: `--url` is a full URL, such as `https://example.org`, \
             not `mailto:someone@example.org`\n  More: ",
        ),
        (
            "build",
            "--url=",
            "error  Missing value: `--url=` needs a full URL\n",
        ),
        (
            "serve",
            "--url=https://preview.example.org",
            "error  Unknown option: `statac serve` has no `--url`\n  \
             `--url` is for `statac build` and `statac check`.\n",
        ),
    ] {
        let output = statac(&[command, option, &site]);
        assert_eq!(output.status.code(), Some(2), "{option}");
        assert!(output.stdout.is_empty(), "{option}: {output:?}");
        let said = String::from_utf8(output.stderr).unwrap();
        assert!(said.starts_with(says), "{option}: {said}");
        // Nothing of the site is named, and nothing is built.
        assert!(!said.contains("statac.yaml"), "{option}: {said}");
        assert!(!folder.join("_site").exists(), "{option}");
    }
    std::fs::remove_dir_all(&folder).unwrap();
}

#[test]
fn a_question_answered_exits_with_0() {
    for args in [&["--help"][..], &["--version"], &["explain", "broken-link"]] {
        let output = statac(args);
        assert_eq!(output.status.code(), Some(0), "{args:?}");
        assert!(!output.stdout.is_empty(), "{args:?}");
        assert!(output.stderr.is_empty(), "{args:?}");
    }
}

#[test]
fn colour_is_never_written_into_a_pipe() {
    let output = Command::new(env!("CARGO_BIN_EXE_statac"))
        .arg("biuld")
        .env_remove("NO_COLOR")
        .output()
        .unwrap();
    assert!(!output.stderr.contains(&0x1b));
}

/// A folder of its own for each test, named `label`, with nothing there.
fn scratch(label: &str) -> Scratch {
    Scratch::new("command-line", label)
}

/// A new site made in `folder` with every default.
fn made(folder: &std::path::Path) {
    let output = statac(&["new", "--yes", &folder.to_string_lossy()]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
}

#[test]
fn the_options_that_wrote_statacs_own_files_into_a_site_are_refused_and_write_nothing() {
    let folder = scratch("retired");
    made(&folder);
    let before: Vec<_> = std::fs::read_dir(&folder)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    for option in ["--update", "--missing", "--cards", "--no-starter"] {
        let output = statac(&["new", option, &folder.to_string_lossy()]);
        assert_eq!(output.status.code(), Some(2), "{option}: {output:?}");
        assert!(output.stdout.is_empty(), "{option}: {output:?}");
        let said = String::from_utf8(output.stderr).unwrap();
        assert!(
            said.starts_with(&format!("error  Unknown option: `{option}`\n")),
            "{said}"
        );
        assert!(said.contains("Statac no longer writes templates into a site"));
    }
    let after: Vec<_> = std::fs::read_dir(&folder)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(after, before);
}

#[test]
fn refresh_says_why_a_token_file_isnt_used() {
    let folder = scratch("token-file");
    made(&folder);
    std::fs::create_dir_all(folder.join(".statac")).unwrap();
    std::fs::write(folder.join(".statac/token"), "secret-1\nsecret-2\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_statac"))
        .args(["refresh", &folder.to_string_lossy()])
        .env("NO_COLOR", "1")
        .env_remove("STATAC_WEBMENTION_TOKEN")
        .output()
        .unwrap();
    let said = String::from_utf8(output.stdout).unwrap();
    assert!(
        said.starts_with("`.statac/token` wasn't used: it holds more than one line.\n"),
        "{said}"
    );
    assert!(!said.contains("secret"), "{said}");
}

/// `statac` run in `folder`.
fn statac_in(folder: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_statac"))
        .args(args)
        .current_dir(folder)
        .env("NO_COLOR", "1")
        .env_remove("STATAC_WEBMENTION_TOKEN")
        .output()
        .unwrap()
}

/// Every file and folder under `folder`, by its path in it.
fn everything(folder: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    let mut waiting = vec![folder.to_path_buf()];
    while let Some(at) = waiting.pop() {
        for entry in std::fs::read_dir(&at).unwrap() {
            let path = entry.unwrap().path();
            found.push(path.strip_prefix(folder).unwrap().to_path_buf());
            if path.is_dir() {
                waiting.push(path);
            }
        }
    }
    found.sort();
    found
}

/// The nearest folder at or above `folder` with either mark of a site.
fn site_above(folder: &std::path::Path) -> Option<std::path::PathBuf> {
    let whole = std::fs::canonicalize(folder).unwrap();
    whole
        .ancestors()
        .find(|at| {
            std::fs::read_dir(at).is_ok_and(|mut entries| {
                entries.any(|entry| entry.is_ok_and(|entry| entry.file_name() == "statac.yaml"))
            }) || std::fs::symlink_metadata(at.join("content")).is_ok_and(|found| found.is_dir())
        })
        .map(std::path::Path::to_path_buf)
}

#[test]
fn a_command_run_inside_a_site_works_on_the_site_and_names_files_from_it() {
    let site = scratch("inside");
    made(&site);
    std::fs::write(
        site.join("content/post-2.md"),
        "---\ntitle: Post 2\n---\n[Nowhere](/nowhere/)\n",
    )
    .unwrap();
    let three_down = site.join("content/a/b");
    std::fs::create_dir_all(&three_down).unwrap();
    let checked = statac_in(&site, &["check", "--today=2026-10-04"]);
    assert_eq!(checked.status.code(), Some(0), "{checked:?}");
    let said = String::from_utf8(checked.stdout).unwrap();
    assert!(said.contains("  content/post-2.md, line 4\n"), "{said}");
    for below in [site.join("content"), three_down.clone()] {
        let output = statac_in(&below, &["check", "--today=2026-10-04"]);
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        assert_eq!(String::from_utf8(output.stdout).unwrap(), said);
    }

    let output = statac_in(&three_down, &["build", "--today=2026-10-04"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let built = String::from_utf8(output.stdout).unwrap();
    let last = built.lines().last().unwrap();
    let out = ["..", "..", "..", "_site"].join(std::path::MAIN_SEPARATOR_STR);
    assert!(last.contains(&format!(" to `{out}` in ")), "{built}");
    assert!(site.join("_site/.statac-built").is_file());
    assert!(everything(&three_down).is_empty());

    let output = statac_in(&site.join("content"), &["stamp", "--today=2026-10-04"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let output = statac_in(&three_down, &["refresh", "--today=2026-10-04"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "Nothing to fetch.\n"
    );
    assert!(everything(&three_down).is_empty());
}

const NO_SITE: &str = "  Run it in a site's folder, or make one with `statac new`.\n  \
                       More: https://statac.dev/e/no-site-here/\n";

#[test]
fn a_folder_in_no_site_is_refused_and_left_as_it_was() {
    let folder = scratch("in-no-site");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("post-1.md"), "# Post 1\n").unwrap();
    let output = statac_in(&folder, &["check"]);
    // A site above the system's temporary folder is this machine's, and
    // `check` reads it and writes nothing: nothing is asked of it here but
    // that this folder is left alone.
    if site_above(&folder).is_none() {
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            format!(
                "error  No site here: `.` has no `content/` folder and no `statac.yaml`\n\
                 {NO_SITE}\n1 error: 1 no-site-here.\n"
            )
        );
        let output = statac_in(&folder, &["check", "--json"]);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            format!(
                "{{\"statac\":\"{}\",\"format\":1,\"site\":null,\"problems\":[\
                 {{\"severity\":\"error\",\"kind\":\"no-site-here\",\
                 \"message\":\"No site here: `.` has no `content/` folder and no `statac.yaml`\",\
                 \"file\":null,\"start\":null,\"end\":null,\
                 \"suggestion\":\"Run it in a site\\u0027s folder, or make one with `statac new`.\",\
                 \"explanation\":\"https://statac.dev/e/no-site-here/\",\"quietened\":false,\
                 \"related\":[],\"pages\":[]}}]}}\n",
                env!("CARGO_PKG_VERSION")
            )
        );
        let output = statac_in(&folder, &["search", "post"]);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            format!(
                "error  No site here: `.` has no `content/` folder and no `statac.yaml`\n\
                 {NO_SITE}"
            )
        );
    }
    assert_eq!(everything(&folder), [std::path::PathBuf::from("post-1.md")]);
}

#[test]
fn a_folder_named_is_taken_as_named_and_never_looked_above() {
    let site = scratch("named");
    made(&site);
    let notes = site.join("notes");
    std::fs::create_dir_all(&notes).unwrap();
    std::fs::write(notes.join("post-1.md"), "# Post 1\n").unwrap();
    let refused = |args: &[&str]| {
        let output = statac_in(&site, args);
        assert_eq!(output.status.code(), Some(1), "{args:?} {output:?}");
        assert!(output.stderr.is_empty(), "{args:?} {output:?}");
        String::from_utf8(output.stdout).unwrap()
    };
    let headline = "error  No site here: `notes` has no `content/` folder and no `statac.yaml`\n";
    let stopped = format!("{headline}{NO_SITE}\nWrote nothing: 1 error stopped the build.\n");
    let counted = format!("{headline}{NO_SITE}\n1 error: 1 no-site-here.\n");
    assert_eq!(refused(&["build", "notes"]), stopped);
    assert_eq!(refused(&["build", "--site=notes"]), stopped);
    assert_eq!(refused(&["serve", "notes", "--offline"]), stopped);
    assert_eq!(refused(&["check", "notes"]), counted);
    assert_eq!(refused(&["refresh", "notes"]), counted);
    assert_eq!(refused(&["stamp", "notes"]), format!("{headline}{NO_SITE}"));
    assert_eq!(
        refused(&["search", "post", "notes"]),
        format!("{headline}{NO_SITE}")
    );
    assert_eq!(everything(&notes), [std::path::PathBuf::from("post-1.md")]);
    assert_eq!(
        refused(&["check", "not-there"]),
        "error  Can't read `not-there`: it doesn't exist\n  \
         More: https://statac.dev/e/cant-read/\n\n1 error: 1 cant-read.\n"
    );

    // Either mark is enough, inside another site or not.
    std::fs::write(notes.join("statac.yaml"), "title: Notes\n").unwrap();
    let output = statac_in(&site, &["check", "notes"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    std::fs::remove_file(notes.join("statac.yaml")).unwrap();
    std::fs::create_dir(notes.join("content")).unwrap();
    let output = statac_in(&site, &["check", "notes"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
}

#[test]
fn new_reads_lines_handed_to_it_as_its_answers() {
    use std::io::Write;
    let folder = scratch("answers-handed-over");
    let mut running = Command::new(env!("CARGO_BIN_EXE_statac"))
        .args(["new", &folder.to_string_lossy()])
        .env("NO_COLOR", "1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = running.stdin.take().unwrap();
    stdin.write_all(b"Site 1\nhttps://example.org\n").unwrap();
    drop(stdin);
    let output = running.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let said = String::from_utf8(output.stdout).unwrap();
    assert!(!said.contains('?'), "{said}");
    let settings = std::fs::read_to_string(folder.join("statac.yaml")).unwrap();
    assert!(
        settings.starts_with("title: Site 1\nurl: https://example.org\n"),
        "{settings}"
    );
}

#[test]
fn new_and_import_inside_a_site_make_their_site_where_they_are_told() {
    let site = scratch("making-inside");
    made(&site);
    let empty = site.join("content/empty");
    std::fs::create_dir_all(&empty).unwrap();
    let output = statac_in(&empty, &["new", "--yes"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(empty.join("statac.yaml").is_file());

    let old = site.join("old");
    std::fs::create_dir_all(&old).unwrap();
    std::fs::write(old.join("_config.yml"), "title: Site 1\n").unwrap();
    let output = statac_in(&site, &["import", "old", "--today=2026-10-04"]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(site.join("old-statac/statac.yaml").is_file());
}

#[test]
fn serve_run_inside_a_site_starts_on_the_site() {
    use std::io::BufRead;
    let site = scratch("serve-inside");
    made(&site);
    let below = site.join("content/a");
    std::fs::create_dir_all(&below).unwrap();
    let mut serving = Command::new(env!("CARGO_BIN_EXE_statac"))
        .args(["serve", "--offline"])
        .current_dir(&below)
        .env("NO_COLOR", "1")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let mut said = String::new();
    let mut lines = std::io::BufReader::new(serving.stdout.take().unwrap()).lines();
    for line in lines.by_ref() {
        let line = line.unwrap();
        said.push_str(&line);
        said.push('\n');
        if line == "Press Ctrl+C to stop." {
            break;
        }
    }
    let _ = serving.kill();
    let _ = serving.wait();
    let folder = ["..", "..", ".statac", "serve"].join(std::path::MAIN_SEPARATOR_STR);
    assert!(
        said.contains(&format!("\nServing `{folder}` at ")),
        "{said}"
    );
    assert!(site.join(".statac/serve/.statac-built").is_file());
    assert!(everything(&below).is_empty());
}

#[test]
fn serve_says_nothing_of_a_connection_closed_before_its_answer() {
    use std::io::{BufRead, Read, Write};
    use std::net::{Ipv4Addr, TcpStream};
    let site = scratch("serve-closed");
    made(&site);
    // More than the system holds for a connection nobody reads from, so
    // the answer is still being written when the connection closes.
    std::fs::create_dir_all(site.join("assets")).unwrap();
    std::fs::write(site.join("assets/large.bin"), vec![0; 8 << 20]).unwrap();
    let mut serving = Command::new(env!("CARGO_BIN_EXE_statac"))
        .args(["serve", "--offline"])
        .current_dir(&site)
        .env("NO_COLOR", "1")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut port = None;
    let mut lines = std::io::BufReader::new(serving.stdout.take().unwrap()).lines();
    for line in lines.by_ref() {
        let line = line.unwrap();
        if let Some((_, at)) = line.rsplit_once(" at http://localhost:") {
            port = at.trim_end_matches('/').parse::<u16>().ok();
        }
        if line == "Press Ctrl+C to stop." {
            break;
        }
    }
    let port = port.expect("serve says where it answers");
    let large = everything(&site.join(".statac/serve"))
        .into_iter()
        .filter_map(|path| Some(path.file_name()?.to_str()?.to_owned()))
        .find(|name| name.starts_with("large."))
        .expect("the large file is published");
    let ask = |target: &str| {
        let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
        write!(
            stream,
            "GET {target} HTTP/1.1\r\nHost: localhost:{port}\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        stream
    };
    for target in ["/", &format!("/{large}")].repeat(4) {
        drop(ask(target));
    }
    // Each answer has failed by the time a later one is read whole.
    std::thread::sleep(std::time::Duration::from_millis(500));
    let mut answer = String::new();
    ask("/").read_to_string(&mut answer).unwrap();
    assert!(answer.starts_with("HTTP/1.1 200 OK\r\n"), "{answer}");
    let _ = serving.kill();
    let _ = serving.wait();
    let mut said = String::new();
    serving
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut said)
        .unwrap();
    assert_eq!(said, "");
}

#[test]
fn lsp_speaks_only_the_protocol_on_standard_output() {
    use std::io::Write;
    let messages = [
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#,
        r#"{"jsonrpc":"2.0","id":2,"method":"shutdown"}"#,
        r#"{"jsonrpc":"2.0","method":"exit"}"#,
    ];
    let mut child = Command::new(env!("CARGO_BIN_EXE_statac"))
        .arg("lsp")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    for message in messages {
        write!(input, "Content-Length: {}\r\n\r\n{message}", message.len()).unwrap();
    }
    drop(input);
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    let said = String::from_utf8(output.stdout).unwrap();
    let answers: Vec<&str> = said.split("Content-Length: ").skip(1).collect();
    assert_eq!(answers.len(), 2, "{said}");
    assert!(said.starts_with("Content-Length: "), "{said}");
    assert!(
        answers[1].ends_with(r#"{"jsonrpc":"2.0","id":2,"result":null}"#),
        "{said}"
    );
}
