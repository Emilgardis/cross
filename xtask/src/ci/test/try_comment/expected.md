<!--try-conclusion-comment-->
## [Try](https://github.com/cross-rs/cross/actions/runs/1?pr=1337) run for [comment](https://github.com/cross-rs/cross/pull/1337#issuecomment-1) - ❌

[Matrix](https://github.com/cross-rs/cross/actions/runs/1/job/101?pr=1337#step:4:1)

#### Failed Jobs (19)

- <details>
  <summary>❌ <b>Build Docker image</b> (7)</summary>

  - [powerpc-unknown-linux-gnu](https://github.com/cross-rs/cross/actions/runs/1/job/153?pr=1337)
    ```
    + cp 'root-powerpc/boot/vmlinu*' kernel
    cp: cannot stat 'root-powerpc/boot/vmlinu*': No such file or directory
    ERROR: process "/bin/sh -c /linux-image.sh powerpc" did not complete successfully: exit code: 1
    ```
  - [sparc64-unknown-linux-gnu](https://github.com/cross-rs/cross/actions/runs/1/job/154?pr=1337)
    ```
    + cp 'root-sparc64/boot/vmlinu*' kernel
    cp: cannot stat 'root-sparc64/boot/vmlinu*': No such file or directory
    ERROR: process "/bin/sh -c /linux-image.sh sparc64" did not complete successfully: exit code: 1
    ```
  - [x86_64-unknown-freebsd](https://github.com/cross-rs/cross/actions/runs/1/job/112?pr=1337)
    ```
    + curl -4 --retry 3 -sSfL http://ftp.freebsd.org/pub/FreeBSD/releases/amd64/-RELEASE/base.txz -O
    curl: (22) The requested URL returned error: 404
    ERROR: process "/bin/sh -c /freebsd.sh" did not complete successfully: exit code: 22
    ```
  - [aarch64-unknown-freebsd](https://github.com/cross-rs/cross/actions/runs/1/job/152?pr=1337)
    ```
    + curl -4 --retry 3 -sSfL http://ftp.freebsd.org/pub/FreeBSD/releases/arm64/-RELEASE/base.txz -O
    curl: (22) The requested URL returned error: 404
    ERROR: process "/bin/sh -c /freebsd.sh" did not complete successfully: exit code: 22
    ```
  - [i686-unknown-freebsd](https://github.com/cross-rs/cross/actions/runs/1/job/113?pr=1337)
    ```
    + curl -4 --retry 3 -sSfL http://ftp.freebsd.org/pub/FreeBSD/releases/i386/-RELEASE/base.txz -O
    curl: (22) The requested URL returned error: 404
    ERROR: process "/bin/sh -c /freebsd.sh" did not complete successfully: exit code: 22
    ```
  - [x86_64-unknown-netbsd](https://github.com/cross-rs/cross/actions/runs/1/job/114?pr=1337)
    ```
    Unable to download base.tar.xz
    + exit 1
    ERROR: process "/bin/sh -c /netbsd.sh" did not complete successfully: exit code: 1
    ```
  - [zig](https://github.com/cross-rs/cross/actions/runs/1/job/156?pr=1337)
    ```
    [build 2/2] RUN /zig.sh linux/amd64
    /bin/sh: 1: /zig.sh: Permission denied
    ERROR: process "/bin/sh -c /zig.sh $TARGETPLATFORM" did not complete successfully: exit code: 126
    ```

  </details>
- ❌ **Test Image > build bin** (7)
  - [thumbv6m-none-eabi](https://github.com/cross-rs/cross/actions/runs/1/job/110?pr=1337), [thumbv7m-none-eabi](https://github.com/cross-rs/cross/actions/runs/1/job/111?pr=1337), [thumbv7em-none-eabi](https://github.com/cross-rs/cross/actions/runs/1/job/117?pr=1337), [thumbv7em-none-eabihf](https://github.com/cross-rs/cross/actions/runs/1/job/118?pr=1337), [armv7a-none-eabi](https://github.com/cross-rs/cross/actions/runs/1/job/150?pr=1337), [armebv7r-none-eabi](https://github.com/cross-rs/cross/actions/runs/1/job/151?pr=1337)
    ```
    error: linking with `false` failed: exit status: 1
    error: could not compile `hello` (bin "hello") due to 1 previous error
    ```
  - [thumbv7neon-unknown-linux-gnueabihf](https://github.com/cross-rs/cross/actions/runs/1/job/155?pr=1337)
    ```
    error: linker `arm-linux-gnueabihf-gcc` not found
    error: could not compile `hello` (bin "hello") due to 1 previous error
    ```
- ❌ **Test Image > run bin** (2): [thumbv8m.base-none-eabi](https://github.com/cross-rs/cross/actions/runs/1/job/119?pr=1337), [thumbv8m.main-none-eabi](https://github.com/cross-rs/cross/actions/runs/1/job/122?pr=1337)
  ```
  /linux-runner: line 133:    27 Segmentation fault      (core dumped) ${QEMU_CMD} >> "${LOG}" 2>&1
  ```
- ❌ **Test Zig Image** (1): [zig](https://github.com/cross-rs/cross/actions/runs/1/job/115?pr=1337)
- ❌ **cancelled** (1): [s390x-unknown-linux-gnu](https://github.com/cross-rs/cross/actions/runs/1/job/116?pr=1337)
- ❌ **Run clippy** (1): [clippy (ubuntu-latest)](https://github.com/cross-rs/cross/actions/runs/1/job/102?pr=1337)

#### Successful Jobs (11)

<details>
<summary>List</summary>

| | | | |
|-|-|-|-|
| [aarch64-unknown-linux-gnu](https://github.com/cross-rs/cross/actions/runs/1/job/120?pr=1337#step:12:1) | [x86_64-apple-darwin](https://github.com/cross-rs/cross/actions/runs/1/job/121?pr=1337) | [armv7-unknown-linux-gnueabihf](https://github.com/cross-rs/cross/actions/runs/1/job/140?pr=1337#step:12:1) | [i686-unknown-linux-gnu](https://github.com/cross-rs/cross/actions/runs/1/job/141?pr=1337#step:12:1) |
| [mips-unknown-linux-gnu](https://github.com/cross-rs/cross/actions/runs/1/job/142?pr=1337#step:12:1) | [powerpc64le-unknown-linux-gnu](https://github.com/cross-rs/cross/actions/runs/1/job/143?pr=1337#step:12:1) | [riscv64gc-unknown-linux-gnu](https://github.com/cross-rs/cross/actions/runs/1/job/144?pr=1337#step:12:1) | [x86_64-unknown-linux-musl](https://github.com/cross-rs/cross/actions/runs/1/job/145?pr=1337#step:12:1) |
| [aarch64-linux-android](https://github.com/cross-rs/cross/actions/runs/1/job/146?pr=1337#step:12:1) | [x86_64-pc-windows-gnu](https://github.com/cross-rs/cross/actions/runs/1/job/147?pr=1337#step:12:1) | [wasm32-unknown-emscripten](https://github.com/cross-rs/cross/actions/runs/1/job/148?pr=1337#step:12:1) |

</details>
