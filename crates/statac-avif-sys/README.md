# statac-avif-sys

libavif 1.4.2 with libaom 3.15.1, built from the source this crate
carries, for encoding only. [Statac](https://statac.dev), a static site
generator, uses it to write the AVIF copies it makes of a site's images.

It is published so that Statac can be installed from crates.io, and is
kept for Statac's use alone. It follows Statac's needs, not libavif's
releases, and makes no promise to anyone else: its interface may change
in any release, whatever its version says.

For libavif in your own project, use libavif itself:
[github.com/AOMediaCodec/libavif](https://github.com/AOMediaCodec/libavif).

## Building

Nothing is downloaded: the build script builds libavif and libaom from
`vendor/` with CMake. It needs a C and C++ compiler, CMake and Perl, and
NASM on a machine with an Intel or AMD processor.

Statac's source and issues are at
[git.statac.dev/statac/statac](https://git.statac.dev/statac/statac).

## Licence

The crate's own code is offered under the
[MIT licence](https://git.statac.dev/statac/statac/src/branch/main/LICENSE-MIT)
or the
[Apache License 2.0](https://git.statac.dev/statac/statac/src/branch/main/LICENSE-APACHE),
at your choice. libavif and libaom, and the code of others they hold,
keep their own licences:
[`licences.md`](https://git.statac.dev/statac/statac/src/branch/main/crates/statac-avif-sys/licences.md)
lists each, and their texts are in
[`vendor/`](https://git.statac.dev/statac/statac/src/branch/main/crates/statac-avif-sys/vendor).
