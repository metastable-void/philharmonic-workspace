# `archives/` — workspace HEAD snapshot bundles

Output directory for [`scripts/archive-all.sh`](../scripts/archive-all.sh).
Each invocation writes one self-contained tarball capturing the
parent workspace plus every submodule at their respective `HEAD`s,
named after the parent's full commit SHA.

## What lives here

- `philharmonic-workspace-<full-HEAD-sha>.tar.zst` — generated
  bundles. **Not tracked** ([../.gitignore](../.gitignore) →
  `/archives/*.tar.zst`, `/archives/*.tar.gz`).
- `README.md` — this file. Tracked.

## Generating a bundle

```sh
./scripts/archive-all.sh
```

The script runs `git archive HEAD` over the parent and each
submodule (`git submodule foreach --recursive`), prefixes every
entry with `philharmonic-workspace-<sha>/<displaypath>/`, and
concatenates the per-tree uncompressed tarballs into a single
zstd-compressed output via the `tar-concatenate` xtask bin
([`xtask/src/bin/tar-concatenate.rs`](../xtask/src/bin/tar-concatenate.rs)).
All intermediate tempfiles come from
[`scripts/mktemp.sh`](../scripts/mktemp.sh) and are removed on
exit.

The script aborts early if any submodule is uninitialized — a
partial bundle would be silently incomplete. Run
[`scripts/setup.sh`](../scripts/setup.sh) (or `git submodule
update --init --recursive`) first.

## Internal layout

Extracting the bundle yields a single top-level directory whose
name encodes the parent's HEAD:

```
philharmonic-workspace-<full-HEAD-sha>/
    CLAUDE.md
    CONTRIBUTING.md
    Cargo.toml
    README.md
    ...                                  # parent workspace files
    inline-blob/                         # submodule by displaypath
    mechanics-core/
    philharmonic-types/
    xtask/
    ...
```

The parent's gitlinks pin every submodule's commit at the named
parent SHA, so the bundle filename uniquely identifies the whole
snapshot — no per-submodule SHA is recorded in the filename.

## Caveats

- **HEAD-only.** Staged or unstaged working-tree changes are not
  captured. Commit (or stash) before archiving if you need them
  included.
- **Submodule `HEAD` ≠ parent gitlink, in principle.** `git
  archive HEAD` inside a submodule reads that submodule's own
  `HEAD`. Right after `pull-all.sh` / `setup.sh` it matches the
  parent's pinned commit; if you've manually checked out a
  different commit inside a submodule, the bundle captures the
  submodule's actual `HEAD`, not the parent's pin. Rerun
  `pull-all.sh` first if you want the parent-pinned snapshot.
- **Generated artefacts; not committed.** The git-ignore rule
  covers `*.tar.gz` and `*.tar.zst` rooted at `/archives/`.
  Anything else dropped here (e.g. a future hand-curated
  manifest) would be tracked unless extended.

See the script's header comment for the full pipeline and the
[`tar-concatenate`](../xtask/src/bin/tar-concatenate.rs) bin for
the concatenation primitive.
