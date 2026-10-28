# Code statac-avif-sys carries

`vendor/avif` is libavif and `vendor/avif/ext/aom` is libaom, each from
its release tarball less the folders an encode-only build does not need.
`build.rs` compiles both; nothing is downloaded.

## libavif

- libavif 1.4.2: BSD-2-Clause. https://github.com/AOMediaCodec/libavif/archive/refs/tags/v1.4.2.tar.gz. Licence: `vendor/avif/LICENSE`, which also gives the licences of the files below.
- `vendor/avif/src/obu.c`, from dav1d: BSD-2-Clause. `vendor/avif/LICENSE`
- `vendor/avif/third_party/libyuv`, part of libyuv: BSD-3-Clause. `vendor/avif/LICENSE`
- `vendor/avif/contrib/gdk-pixbuf`: BSD-2-Clause. `vendor/avif/LICENSE`

## libaom

- libaom 3.15.1: BSD-2-Clause. https://storage.googleapis.com/aom-releases/libaom-3.15.1.tar.gz. Licence: `vendor/avif/ext/aom/LICENSE`. Patents: Alliance for Open Media Patent License 1.0, `vendor/avif/ext/aom/PATENTS`.

Third-party folders inside libaom, each with the source named in its
`README.libaom`:

- `third_party/fastfeat`, FAST feature detectors: BSD-3-Clause. https://github.com/edrosten/fast-C-src. `vendor/avif/ext/aom/third_party/fastfeat/LICENSE`
- `third_party/libyuv`: BSD-3-Clause. https://chromium.googlesource.com/libyuv/libyuv/. `vendor/avif/ext/aom/third_party/libyuv/LICENSE`
- `third_party/vector`: MIT. https://github.com/goldsborough/vector. `vendor/avif/ext/aom/third_party/vector/LICENSE`
- `third_party/x86inc`, from x264: ISC. https://git.videolan.org/git/x264.git. `vendor/avif/ext/aom/third_party/x86inc/LICENSE`
