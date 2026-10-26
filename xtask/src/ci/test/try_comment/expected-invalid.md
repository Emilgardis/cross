<!--try-conclusion-comment-->
## [Try](https://github.com/cross-rs/cross/actions/runs/2?pr=1337) run for [comment](https://github.com/cross-rs/cross/pull/1337#issuecomment-1) - ❌

[Matrix](https://github.com/cross-rs/cross/actions/runs/2/job/101?pr=1337#step:4:1)

#### Invalid Command

```
no targets matched `i686-linux`
```

<details>
<summary>Usage</summary>

```
Usage: /ci try [OPTIONS]

Options:
  -t, --target [<TARGET>...]    Filter targets to try. Checks exact name, name.sub, glob (`*-musl*`), and special images (`zig`, `cross`)
      --std <STD>               [possible values: true, false]
      --cpp <CPP>               [possible values: true, false]
      --dylib <DYLIB>           [possible values: true, false]
      --run <RUN>               [possible values: true, false]
  -r, --runners [<RUNNERS>...]  Runners to filter targets by, match against the `runners` field in targets.toml. Does not limit which runners the tests actually use
      --none                    try no targets, only `--tests` jobs
  -v, --verbose                 pass `-v` to `xtask build-docker-image`
  -c, --cross-debug             sets `CROSS_DEBUG=true`
      --tests [<TESTS>...]      extra test jobs to run [default: all] [possible values: remote, bisect, foreign, docker-in-docker, podman, none, all]
  -h, --help                    Print help
```

</details>

#### Failed Jobs (1)

- **Generate matrix** (1): [generate-matrix](https://github.com/cross-rs/cross/actions/runs/2/job/101?pr=1337)


