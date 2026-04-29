# Instructions/Notes for human developers

**For coding agents**: You can freely read the contents of
this file, but never edit this file.

**The following is notes for humans and not for coding agents**

---

## Short-term TODOs

- make sure we always make docs/roadmaps up-to-date.
- Note: I'll work on 29 April 2026.
- not blocking for this project: we should refactor all
  the best practices for AI coding brewed inside this
  workspace repo into a reusable template/Rust crate/etc.
  this can happen after the MVP work is done.
- Note: Normal person-hour calculations show that MVP
  isn't possible to land before the Golden Week holidays;
  Japanese working-hour traditions can possibly make it
  possible: just make sure we don't hit karoshi - if I
  die, no one capable of significant coding would be left
  at this project.

---

## Integration

Each of our bin targets are an HTTP server with an optional
`https` feature to enable TLS and HTTP/2 (and maybe, HTTP/3).
HTTPS is totally optional: one may only run a service at
`127.0.0.1` (localhost), or run one behind a reverse proxy,
and it should be fine. Sending SIGHUPs to the running services
should let them reload the configs, and refresh HTTPS certs
from the disk.

Bin targets are only opinionated towards our recommended split
among the bins, and configurable otherwise.

Let's extend `mechanics` HTTP server crate (the JS worker)
to that shape first.

Let's export each crate at the `philharmonic` meta-crate
at its top level. the meta crate by default exports all the
connector impls but one can use `default-features = false` and
manually specify which impls to use.

We build a default WebUI with Redux+React+Webpack, build it,
and commit all the resulting artifact (only `index.html` and
`main.js` and `main.css`, in addition to `icon.svg`) to Git
to enable building without Node.JS. We ship the common default
icon to use at `common-assets/d-icon.svg`. On everything not
`main.js`/`main.css`/`icon.svg`, we serve `index.html`, cause
it's an SPA.

Add the following bin targets to
the meta crate, to enable our first deployment:

- `src/bin/mechanics-worker/main.rs`: a bit better HTTP server
  wrapper supporting Clap CLI and `/etc/philharmonic/mechanics.toml`
  + `/etc/philharmonic/mechanics.toml.d/*.toml` (configurable
  location) config files, for `mechanics` (JS executor).
- `src/bin/philharmonic-api/main.rs`: the API server, integrating
  various things needed for it to work. Reads the config files at
  `/etc/philharmonic/api.toml` + `/etc/philharmonic/api.toml.d/*.toml`
  and accepts Clap CLI flags. API server also integrates the Connector
  proxy feature into itself. This is what is usually deployed with
  HTTPS at port 443. For that reason, it also contains the Web UI.
  The static assets are embedded into the Rust binary.
- `src/bin/philharmonic-connector/main.rs`: the Connector Service.
  ships everything supported at the moment by default (one can
  remove connectors impls with `default-features = false`) and
  opt-in for anything needed. Run one per Realm.

Bin targets must compile fine for x86_64-unknown-linux-musl.

Each of bin targets has subcommands (e.g. `serve`/`help`/`version`
plus normal `--help|-h`/`--version|-V`), and one `install` subcommand
(requires a root) installs the binary to `/usr/local/bin/`, and
installs a systemd service unit file at
`/usr/local/lib/systemd/system/*.service` creating intermediate
directories if needed, creates config files and directories,
and runs `systemctl enable [...].service` (but not starting it).
The installer must be idempotent, and prints the instructions on
how to configure it at the end.

### Optional docker compose support

We may optionally support `docker compose` with minimal Alpine
images (we don't run `install` commands there). Local override
files will supply HTTPS certs locations, hostnames, etc.
