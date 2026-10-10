//! Times reading a folder of Markdown posts into trees, on one thread and
//! on every core: `cargo run --release --example markdown_bench <folder>`.

use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use statac::markdown;
use statac::source::{SourceId, Sources, Span};

const RUNS: usize = 5;

fn main() {
    let folder = std::env::args()
        .nth(1)
        .expect("give the folder of posts to read");
    let mut sources = Sources::default();
    let mut posts: Vec<(SourceId, Span)> = Vec::new();
    let mut bytes = 0;
    let mut paths: Vec<_> = std::fs::read_dir(&folder)
        .expect("a readable folder")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == "md"))
        .collect();
    paths.sort();
    for path in paths {
        let text = std::fs::read_to_string(&path).expect("a readable post");
        bytes += text.len();
        let body = statac::yaml::split(&text)
            .expect("closed front matter")
            .body;
        posts.push((sources.add(path, text), body));
    }
    let threads = std::thread::available_parallelism().map_or(1, usize::from);
    println!(
        "{} posts, {:.1} MB, {threads} threads",
        posts.len(),
        bytes as f64 / 1e6
    );

    let read_one = |&(id, body): &(SourceId, Span)| {
        markdown::read(&sources, id, body, &|_| Some(false), "")
            .root
            .children
            .len()
    };
    let single = timed(|| posts.iter().map(read_one).sum());
    let parallel = timed(|| {
        let next = AtomicUsize::new(0);
        let total = AtomicUsize::new(0);
        std::thread::scope(|scope| {
            for _ in 0..threads {
                scope.spawn(|| {
                    let mut sum = 0;
                    while let Some(post) = posts.get(next.fetch_add(1, Ordering::Relaxed)) {
                        sum += read_one(post);
                    }
                    total.fetch_add(sum, Ordering::Relaxed);
                });
            }
        });
        total.into_inner()
    });
    println!(
        "1 thread:   median {:.1} ms, fastest {:.1} ms",
        single.0, single.1
    );
    println!(
        "{threads} threads: median {:.1} ms, fastest {:.1} ms",
        parallel.0, parallel.1
    );
}

/// The median and fastest of several runs, in milliseconds.
fn timed(run: impl Fn() -> usize) -> (f64, f64) {
    let mut times: Vec<f64> = (0..RUNS)
        .map(|_| {
            let start = Instant::now();
            black_box(run());
            start.elapsed().as_secs_f64() * 1000.0
        })
        .collect();
    times.sort_by(f64::total_cmp);
    (times[RUNS / 2], times[0])
}
