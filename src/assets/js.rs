//! What a script shows of itself: whether it is a module, whether it loads
//! modules as it runs, and which workers it names. Read far enough to step
//! over comments, strings and template literals, which is all these
//! questions need; a regular expression holding a quote can mislead it to
//! the end of a line at most.

use std::ops::Range;

#[derive(Default, Debug, PartialEq, Eq)]
pub(super) struct Script {
    /// An `import` or `export` statement, or `import.meta`: only a module
    /// can hold one.
    pub(super) module: bool,
    /// An `import(...)`, which resolves through the import map.
    pub(super) loads_modules: bool,
    /// The insides of each string naming a worker's script:
    /// `new Worker("...")`, `new SharedWorker("...")`, either with
    /// `new URL("...", import.meta.url)`, and `importScripts("...")`.
    pub(super) workers: Vec<Range<usize>>,
}

pub(super) fn read(js: &str) -> Script {
    let bytes = js.as_bytes();
    let mut script = Script::default();
    let mut read = Reader { bytes, at: 0 };
    // The last byte that wasn't space or a comment, to tell `x.import` from
    // `import`.
    let mut before = b';';
    while let Some(byte) = read.peek() {
        match byte {
            b'/' if bytes.get(read.at + 1) == Some(&b'/') => {
                read.line_comment();
                continue;
            }
            b'/' if bytes.get(read.at + 1) == Some(&b'*') => {
                read.block_comment();
                continue;
            }
            b'"' | b'\'' | b'`' => {
                read.string();
            }
            byte if byte.is_ascii_whitespace() => {
                read.at += 1;
                continue;
            }
            byte if starts_a_word(byte) => {
                let word = read.word();
                if before != b'.' {
                    match word {
                        "import" => match read.next_significant() {
                            Some(b'(') => script.loads_modules = true,
                            Some(b'.') => script.module = true,
                            Some(next) if next == b'{' || next == b'*' || is_quote(next) => {
                                script.module = true
                            }
                            Some(next) if starts_a_word(next) => script.module = true,
                            _ => {}
                        },
                        "export" => {
                            if read.next_significant().is_some_and(|next| {
                                next == b'{' || next == b'*' || starts_a_word(next)
                            }) {
                                script.module = true;
                            }
                        }
                        "new" => {
                            let at = read.at;
                            if let Some(named) = read.worker() {
                                script.workers.push(named);
                            } else {
                                read.at = at;
                            }
                        }
                        "importScripts" => {
                            let at = read.at;
                            let named = read.imported_scripts();
                            if named.is_empty() {
                                read.at = at;
                            }
                            script.workers.extend(named);
                        }
                        _ => {}
                    }
                }
                before = b'a';
                continue;
            }
            _ => read.at += 1,
        }
        before = byte;
    }
    script
}

fn starts_a_word(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_' || byte == b'$' || byte >= 0x80
}

fn is_quote(byte: u8) -> bool {
    matches!(byte, b'"' | b'\'' | b'`')
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    fn line_comment(&mut self) {
        while self.peek().is_some_and(|byte| byte != b'\n') {
            self.at += 1;
        }
    }

    fn block_comment(&mut self) {
        self.at += 2;
        while self.at < self.bytes.len() && !self.bytes[self.at..].starts_with(b"*/") {
            self.at += 1;
        }
        self.at = (self.at + 2).min(self.bytes.len());
    }

    fn space(&mut self) {
        loop {
            match self.peek() {
                Some(byte) if byte.is_ascii_whitespace() => self.at += 1,
                Some(b'/') if self.bytes.get(self.at + 1) == Some(&b'/') => self.line_comment(),
                Some(b'/') if self.bytes.get(self.at + 1) == Some(&b'*') => self.block_comment(),
                _ => return,
            }
        }
    }

    /// The next byte that isn't space or a comment, without moving past it.
    fn next_significant(&mut self) -> Option<u8> {
        let at = self.at;
        self.space();
        let next = self.peek();
        self.at = at;
        next
    }

    /// Past a string or template literal, and where its insides sit when
    /// it is plain text: closed, and without a `${` or an escape.
    fn string(&mut self) -> Option<Range<usize>> {
        let quote = self.bytes[self.at];
        self.at += 1;
        let start = self.at;
        let mut plain = true;
        while let Some(byte) = self.peek() {
            match byte {
                b'\\' => {
                    plain = false;
                    self.at += 2;
                }
                b'\n' if quote != b'`' => return None,
                b'$' if quote == b'`' && self.bytes.get(self.at + 1) == Some(&b'{') => {
                    plain = false;
                    self.at += 1;
                }
                byte if byte == quote => {
                    self.at += 1;
                    return plain.then_some(start..self.at - 1);
                }
                _ => self.at += 1,
            }
        }
        self.at = self.bytes.len();
        None
    }

    fn word(&mut self) -> &'a str {
        let start = self.at;
        while self
            .peek()
            .is_some_and(|byte| starts_a_word(byte) || byte.is_ascii_digit())
        {
            self.at += 1;
        }
        // Words end at ASCII bytes, so this never splits a character.
        std::str::from_utf8(&self.bytes[start..self.at]).unwrap_or("")
    }

    fn expect(&mut self, byte: u8) -> Option<()> {
        self.space();
        (self.peek() == Some(byte)).then(|| self.at += 1)
    }

    fn expect_word(&mut self, word: &str) -> Option<()> {
        self.space();
        (self.peek().is_some_and(starts_a_word) && self.word() == word).then_some(())
    }

    fn literal(&mut self) -> Option<Range<usize>> {
        self.space();
        if !self.peek().is_some_and(is_quote) {
            return None;
        }
        self.string()
    }

    /// After `new`: `Worker(` or `SharedWorker(` and the string naming the
    /// script, directly or through `new URL(`.
    fn worker(&mut self) -> Option<Range<usize>> {
        self.space();
        if !self.peek().is_some_and(starts_a_word) {
            return None;
        }
        if !matches!(self.word(), "Worker" | "SharedWorker") {
            return None;
        }
        self.expect(b'(')?;
        let at = self.at;
        if let Some(named) = self.literal() {
            return Some(named);
        }
        self.at = at;
        self.expect_word("new")?;
        self.expect_word("URL")?;
        self.expect(b'(')?;
        self.literal()
    }

    /// After `importScripts`: each string it is handed.
    fn imported_scripts(&mut self) -> Vec<Range<usize>> {
        let mut named = Vec::new();
        if self.expect(b'(').is_none() {
            return named;
        }
        while let Some(one) = self.literal() {
            named.push(one);
            if self.expect(b',').is_none() {
                break;
            }
        }
        named
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_module_is_known_by_its_statements() {
        for js in [
            "import { a } from './a.js';\n",
            "import a from './a.js';\n",
            "import * as a from './a.js';\n",
            "import './side.js';\n",
            "const a = 1;\nexport default a;\n",
            "export { a };\n",
            "console.log(import.meta.url);\n",
        ] {
            assert!(read(js).module, "{js}");
        }
        for js in [
            "// import { a } from './a.js'\nrun();\n",
            "/*\nimport { a } from './a.js'\n*/\n",
            "const s = `\nimport { a } from './a.js'\n`;\n",
            "const s = 'export { a }';\n",
            "const important = 1;\nexports.a = 2;\n",
            "loader.import('a');\n",
        ] {
            assert!(!read(js).module, "{js}");
        }
    }

    #[test]
    fn a_dynamic_import_loads_modules() {
        let script = read("button.onclick = () => import('./later.js');\n");
        assert!(script.loads_modules);
        assert!(!script.module);
    }

    #[test]
    fn workers_are_found_by_the_string_naming_them() {
        let js = "const a = new Worker('worker.js');\n\
                  const b = new SharedWorker(\"/shared.js\", { type: 'module' });\n\
                  const c = new Worker(new URL('./url.js', import.meta.url));\n\
                  importScripts('one.js', \"two.js\");\n\
                  const d = new Worker(name);\n\
                  const e = new Worker(`${base}/x.js`);\n\
                  // new Worker('comment.js');\n\
                  const f = new Map('not-a-worker.js');\n";
        let named: Vec<&str> = read(js).workers.into_iter().map(|r| &js[r]).collect();
        assert_eq!(
            named,
            ["worker.js", "/shared.js", "./url.js", "one.js", "two.js"]
        );
    }
}
