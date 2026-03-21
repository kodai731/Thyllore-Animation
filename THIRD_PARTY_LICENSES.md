# Third-Party Licenses

This document lists all third-party dependencies used by the Thyllore Animation project,
organized by category and license type.

Last updated: 2026-03-16

---

## 1. Vendored Dependencies (bundled in repository)

These libraries are directly included in the repository source tree.

### Dear ImGui (C++)

- **License:** MIT
- **Copyright:** (c) 2014-2023 Omar Cornut
- **Source:** https://github.com/ocornut/imgui
- **Vendored at:** `vendor/imgui`, `vendor/imgui-sys`, `vendor/imgui-winit-support`
- **Usage:** Immediate-mode GUI for debugging and editor UI
- **Rust bindings:** imgui-rs (MIT OR Apache-2.0)

---

## 2. Fonts

| Font | File | License | Copyright |
|------|------|---------|-----------|
| Roboto | `assets/fonts/Roboto-Regular.ttf` | Apache-2.0 | Google Inc. |
| M+ 1p | `assets/fonts/mplus-1p-regular.ttf` | SIL Open Font License 1.1 (OFL-1.1) | M+ Fonts Project |

---

## 3. 3D Models

| Model | File | License | Author |
|-------|------|---------|--------|
| Phoenix Bird | `assets/models/phoenix-bird/`, `tests/testmodels/glTF/skinning/glb/phoenixBird.glb` | CC BY 4.0 | NORBERTO-3D |
| Yard Grass | `assets/models/yard_grass/`, `tests/testmodels/glTF/morphing/yard_grass/` | CC BY 4.0 | ebmclachlan |

- **Phoenix Bird Source:** https://sketchfab.com/3d-models/phoenix-bird-844ba0cf144a413ea92c779f18912042
- **Yard Grass Source:** https://sketchfab.com/3d-models/yard-grass-3a67e76decc849c694c228eb590a9902
- **License URL:** https://creativecommons.org/licenses/by/4.0/

---

## 4. ML Model & Training Data

### ONNX Runtime

- **License:** MIT
- **Copyright:** Microsoft Corporation
- **Usage:** Inference engine for the Curve Copilot model (via `ort` crate)

### Curve Copilot ONNX Model

- **File:** `ml/model/curve_copilot.onnx`
- **Training data sources:**
  - **CMU Motion Capture Database** — Free for all uses (http://mocap.cs.cmu.edu/)
  - **100Style Dataset** — Academic/research use

---

## 5. Cargo Dependencies

All Rust crate dependencies pulled via Cargo, grouped by license. Crates with multiple
versions in the dependency tree are listed once with a version range. Crates offering
a choice of licenses (e.g., "MIT OR Apache-2.0") are listed under their combined
license group.

### MIT OR Apache-2.0

The majority of the Rust ecosystem uses this dual license.

- ahash 0.8.12
- android-activity 0.5.2
- anyhow 1.0.102
- arrayvec 0.7.6
- as-raw-xcb-connection 1.0.1
- async-broadcast 0.7.2
- async-channel 2.5.0
- async-executor 1.14.0
- async-fs 2.2.0
- async-io 2.6.0
- async-lock 3.4.2
- async-net 2.0.0
- async-process 2.5.0
- async-recursion 1.1.1
- async-signal 0.2.13
- async-task 4.7.1
- async-trait 0.1.89
- atomic-waker 1.1.2
- autocfg 1.5.0
- base64 0.13–0.22
- base64ct 1.8.3
- bitflags 1.3–2.11
- blocking 1.6.2
- bumpalo 3.20.2
- cc 1.2.56
- cfg-if 1.0.4
- chrono 0.4.44
- cocoa 0.25.0
- cocoa-foundation 0.1.2
- concurrent-queue 2.5.0
- core-foundation 0.9–0.10
- core-foundation-sys 0.8.7
- core-graphics 0.23.2
- core-graphics-types 0.1.3
- crc32fast 1.5.0
- crossbeam-utils 0.8.21
- der 0.7.10
- displaydoc 0.2.5
- enumflags2 0.7.12
- enumflags2_derive 0.7.12
- equivalent 1.0.2
- errno 0.3.14
- event-listener 5.4.1
- event-listener-strategy 0.5.4
- fastrand 2.3.0
- fbxcel 0.9.0
- fdeflate 0.3.7
- find-msvc-tools 0.1.9
- flate2 1.1.9
- form_urlencoded 1.2.2
- futures-channel 0.3.32
- futures-core 0.3.32
- futures-io 0.3.32
- futures-lite 2.6.1
- futures-macro 0.3.32
- futures-task 0.3.32
- futures-util 0.3.32
- getrandom 0.3–0.4
- gltf 1.4.1
- gltf-derive 1.4.1
- gltf-json 1.4.1
- hashbrown 0.15–0.16
- heck 0.5.0
- hermit-abi 0.1–0.5
- hex 0.4.3
- http 1.4.0
- httparse 1.10.1
- iana-time-zone 0.1.65
- iana-time-zone-haiku 0.1.2
- idna 1.1.0
- idna_adapter 1.2.1
- image 0.25.9
- imgui 0.11.0
- imgui-sys 0.11.0
- imgui-winit-support 0.11.0
- indexmap 2.13.0
- itoa 1.0.17
- jobserver 0.1.34
- js-sys 0.3.90
- lazy_static 1.5.0
- leb128fmt 0.1.0
- libc 0.2.182
- lock_api 0.4.14
- log 0.4.29
- memmap2 0.5–0.9
- metal 0.27.0
- native-tls 0.2.18
- ndarray 0.17.2
- ndk 0.8.0
- ndk-context 0.1.1
- ndk-sys 0.5.0
- num-complex 0.4.6
- num-integer 0.1.46
- num-traits 0.2.19
- once_cell 1.21.3
- openssl-probe 0.2.1
- ordered-stream 0.2.0
- ort 2.0.0-rc.11
- ort-sys 2.0.0-rc.11
- parking 2.2.1
- parking_lot 0.12.5
- parking_lot_core 0.9.12
- paste 1.0.15
- pem-rfc7468 0.7.0
- percent-encoding 2.3.2
- pin-project-lite 0.2.16
- piper 0.2.4
- pkg-config 0.3.32
- png 0.17–0.18
- polling 3.11.0
- portable-atomic 1.13.1
- portable-atomic-util 0.2.5
- ppv-lite86 0.2.21
- prettyplease 0.2.37
- proc-macro-crate 3.4.0
- proc-macro2 1.0.106
- quote 1.0.44
- rand 0.9.2
- rand_chacha 0.9.0
- rand_core 0.9.5
- regex 1.12.3
- regex-automata 0.4.14
- regex-syntax 0.8.10
- rle-decode-fast 1.0.3
- ron 0.8.1
- rustls-pki-types 1.14.0
- rustversion 1.0.22
- scopeguard 1.2.0
- security-framework 3.7.0
- security-framework-sys 2.17.0
- semver 1.0.27
- serde 1.0.228
- serde_core 1.0.228
- serde_derive 1.0.228
- serde_json 1.0.149
- serde_repr 0.1.20
- shlex 1.3.0
- signal-hook-registry 1.4.8
- smallvec 1.15.1
- smol_str 0.2.2
- stable_deref_trait 1.2.1
- syn 2.0.117
- tempfile 3.26.0
- thiserror 1.0.69
- thiserror-impl 1.0.69
- toml_datetime 0.7.5
- toml_edit 0.23.10
- toml_parser 1.0.9
- ttf-parser 0.25.1
- unicode-segmentation 1.12.0
- unicode-xid 0.2.6
- unty 0.0.4
- ureq 3.2.0
- ureq-proto 0.5.3
- url 2.5.8
- utf-8 0.7.6
- utf8_iter 1.0.4
- uuid 1.21.0
- wasm-bindgen 0.2.113
- wasm-bindgen-futures 0.4.63
- wasm-bindgen-macro 0.2.113
- wasm-bindgen-macro-support 0.2.113
- wasm-bindgen-shared 0.2.113
- web-sys 0.3.90
- web-time 0.2.4
- winapi-wsapoll 0.1.2
- windows-core 0.62.2
- windows-implement 0.60.2
- windows-interface 0.59.3
- windows-link 0.2.1
- windows-result 0.4.1
- windows-strings 0.5.1
- windows-sys 0.45–0.61
- windows-targets 0.42–0.52
- windows_aarch64_gnullvm 0.42–0.52
- windows_aarch64_msvc 0.42–0.52
- windows_i686_gnu 0.42–0.52
- windows_i686_gnullvm 0.52.6
- windows_i686_msvc 0.42–0.52
- windows_x86_64_gnu 0.42–0.52
- windows_x86_64_gnullvm 0.42–0.52
- windows_x86_64_msvc 0.42–0.52
- x11rb 0.10–0.13
- x11rb-protocol 0.10–0.13
- zeroize 1.8.2

### MIT

- android-properties 0.2.2
- android_system_properties 0.1.5
- ashpd 0.11.1
- atty 0.2.14
- bincode 2.0.1
- bincode_derive 2.0.1
- block 0.1.6
- block-sys 0.2.1
- block2 0.3–0.6
- bytes 1.11.1
- calloop 0.12.4
- calloop-wayland-source 0.2.0
- cesu8 1.1.0
- cfg_aliases 0.1.1
- clipboard-win 3.1.1
- combine 4.6.7
- copypasta 0.8.2
- dispatch 0.2.0
- downcast-rs 1.2.1
- endi 1.1.1
- env_logger 0.7.1
- foreign-types 0.3–0.5
- foreign-types-macros 0.2.3
- foreign-types-shared 0.1–0.3
- humantime 1.3.0
- icrate 0.0.4
- id-arena 2.3.0
- inflections 1.1.1
- jni 0.21.1
- jni-sys 0.3.0
- libflate 1.4.0
- libflate_lz77 1.2.0
- libredox 0.1.12
- malloc_buf 0.0.6
- matrixmultiply 0.3.10
- memoffset 0.6–0.9
- mint 0.5.9
- nix 0.24.3
- objc 0.2.7
- objc-foundation 0.1.1
- objc-sys 0.3.5
- objc2 0.4–0.6
- objc2-encode 3.0–4.1
- objc2-foundation 0.3.2
- objc_exception 0.1.2
- objc_id 0.1.1
- openssl-macros 0.1.1
- openssl-sys 0.9.111
- orbclient 0.3.50
- pollster 0.4.0
- pretty_env_logger 0.4.0
- quick-error 1.2.3
- quick-xml 0.38.4
- rawpointer 0.2.1
- redox_syscall 0.3–0.7
- rfd 0.15.4
- schannel 0.1.28
- scoped-tls 1.0.1
- sctk-adwaita 0.8.3
- simd-adler32 0.3.8
- slab 0.4.12
- smithay-client-toolkit 0.16–0.18
- smithay-clipboard 0.6.6
- socks 0.3.4
- strict-num 0.1.1
- synstructure 0.13.2
- tobj 3.2.5
- tracing 0.1.44
- tracing-attributes 0.1.31
- tracing-core 0.1.36
- uds_windows 1.1.0
- urlencoding 2.1.3
- vcpkg 0.2.15
- version_check 0.9.5
- virtue 0.0.18
- wayland-backend 0.3.12
- wayland-client 0.29–0.31
- wayland-commons 0.29.5
- wayland-csd-frame 0.3.0
- wayland-cursor 0.29–0.31
- wayland-protocols 0.29–0.32
- wayland-protocols-plasma 0.2.0
- wayland-protocols-wlr 0.2.0
- wayland-scanner 0.29–0.31
- wayland-sys 0.29–0.31
- winapi 0.3.9
- winapi-i686-pc-windows-gnu 0.4.0
- winapi-x86_64-pc-windows-gnu 0.4.0
- winnow 0.7.14
- x11-clipboard 0.7.1
- x11-dl 2.21.0
- xcursor 0.3.10
- xkbcommon-dl 0.4.2
- xml-rs 0.8.28
- zbus 5.14.0
- zbus_macros 5.14.0
- zbus_names 4.3.1
- zmij 1.0.21
- zvariant 5.10.0
- zvariant_derive 5.10.0
- zvariant_utils 3.3.0

### Apache-2.0

- ab_glyph 0.2.32
- ab_glyph_rasterizer 0.1.10
- approx 0.4.0
- cgmath 0.18.0
- gethostname 0.2–1.1
- lzma-rust2 0.15.7
- openssl 0.10.75
- owned_ttf_parser 0.25.1
- vulkanalia 0.26.0
- vulkanalia-sys 0.26.0
- winit 0.29.15

### Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT

- linux-raw-sys 0.4–0.12
- rustix 0.38–1.1
- wasip2 1.0.2
- wasip3 0.4.0
- wasm-encoder 0.244.0
- wasm-metadata 0.244.0
- wasmparser 0.244.0
- wit-bindgen 0.51.0
- wit-bindgen-core 0.51.0
- wit-bindgen-rust 0.51.0
- wit-bindgen-rust-macro 0.51.0
- wit-component 0.244.0
- wit-parser 0.244.0

### MIT OR Apache-2.0 OR Zlib

- bytemuck 1.25.0
- chlorine 1.0.13
- cursor-icon 1.2.0
- dispatch2 0.3.0
- miniz_oxide 0.8.9
- objc2-app-kit 0.3.2
- objc2-core-foundation 0.3.2
- raw-window-handle 0.6.2
- xkeysym 0.2.1
- zune-core 0.5.1
- zune-jpeg 0.5.12

### Unlicense OR MIT

- aho-corasick 1.1.4
- byteorder 1.5.0
- byteorder-lite 0.1.0
- memchr 2.8.0
- same-file 1.0.6
- termcolor 1.4.1
- walkdir 2.5.0
- winapi-util 0.1.11

### Unicode-3.0

- icu_collections 2.1.1
- icu_locale_core 2.1.1
- icu_normalizer 2.1.1
- icu_normalizer_data 2.1.1
- icu_properties 2.1.2
- icu_properties_data 2.1.2
- icu_provider 2.1.1
- litemap 0.8.1
- potential_utf 0.1.4
- tinystr 0.8.2
- writeable 0.6.2
- yoke 0.8.1
- yoke-derive 0.8.1
- zerofrom 0.1.6
- zerofrom-derive 0.1.6
- zerotrie 0.2.3
- zerovec 0.11.5
- zerovec-derive 0.11.2

### (MIT OR Apache-2.0) AND Unicode-3.0

- unicode-ident 1.0.24

### BSD-2-Clause OR Apache-2.0 OR MIT

- zerocopy 0.8.39
- zerocopy-derive 0.8.39

### BSD-2-Clause

- arrayref 0.3.9

### BSD-3-Clause

- tiny-skia 0.11.4
- tiny-skia-path 0.11.4

### BSD-3-Clause OR MIT OR Apache-2.0

- num_enum 0.7.5
- num_enum_derive 0.7.5

### BSD-3-Clause OR Apache-2.0

- moxcms 0.7.11
- pxfm 0.1.27

### 0BSD OR MIT OR Apache-2.0

- adler2 2.0.1

### Zlib

- adler32 1.2.0
- foldhash 0.1.5

### ISC

- hmac-sha256 1.1.14
- libloading 0.8–0.9

### BSL-1.0 (Boost Software License)

- lazy-bytes-cast 5.0.1

### MIT OR Apache-2.0 OR LGPL-2.1-or-later

- r-efi 5.3.0

### MIT OR PDDL-1.0

- ufbx 0.10.1

### CDLA-Permissive-2.0

- webpki-root-certs 1.0.6

---

## 6. Runtime Dependencies

These are not bundled but required at runtime.

### Vulkan SDK / Vulkan Loader

- **License:** Apache-2.0
- **Copyright:** The Khronos Group Inc. / LunarG, Inc.
- **Usage:** Graphics API for rendering
- **Source:** https://www.vulkan.org/

### ONNX Runtime

- **License:** MIT
- **Copyright:** Microsoft Corporation
- **Usage:** ML inference runtime, dynamically linked via the `ort` crate
- **Source:** https://github.com/microsoft/onnxruntime

---

## License References

| SPDX Identifier | Full Name |
|------------------|-----------|
| MIT | MIT License |
| Apache-2.0 | Apache License 2.0 |
| BSD-2-Clause | BSD 2-Clause "Simplified" License |
| BSD-3-Clause | BSD 3-Clause "New" or "Revised" License |
| ISC | ISC License |
| Zlib | zlib License |
| BSL-1.0 | Boost Software License 1.0 |
| 0BSD | BSD Zero Clause License |
| Unlicense | The Unlicense |
| Unicode-3.0 | Unicode License v3 |
| OFL-1.1 | SIL Open Font License 1.1 |
| PDDL-1.0 | Open Data Commons Public Domain Dedication and License 1.0 |
| CDLA-Permissive-2.0 | Community Data License Agreement - Permissive 2.0 |
| LGPL-2.1-or-later | GNU Lesser General Public License v2.1 or later |

Full license texts for each identifier can be found at https://spdx.org/licenses/.
