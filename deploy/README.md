# The azmuth.dev setup

One subdomain per project, free, with its own deployment and release cycle. The
plan works for every app that is *just software* — and Rig is the one that is
not.

## Two kinds of app

| Project | What it needs | Where it runs | How it is reached |
| --- | --- | --- | --- |
| `azmuth.dev` | nothing | Workers static assets | Cloudflare edge |
| `dimension.azmuth.dev` | nothing | Workers static assets | Cloudflare edge |
| `hld.azmuth.dev` | nothing | Workers static assets | Cloudflare edge |
| `rig.azmuth.dev` | **USB keyboard** | **your Mac** | Cloudflare Tunnel |

## Two builds, two homes

Rig now has two shapes, because the browser can do part of the job:

| Build | What it does | Where it belongs |
| --- | --- | --- |
| `./target/release/rig` (native) | lighting, presets, remaps, macros, hotkeys, idle, remote access | runs on the Mac, reachable through the tunnel |
| `deploy/build-webhid.sh` (static) | lighting only, straight from the browser over WebHID | Workers at `rig.azmuth.dev` |

The static build has no server at all: the page talks to the keyboard's vendor
interface directly with WebHID. `deploy/build-webhid.sh` assembles
`dist-webhid/` with the HTML, the JS, the stylesheet, the fonts and a
`wrangler.jsonc` already pointing at `rig.azmuth.dev`.

```sh
./deploy/build-webhid.sh
python3 -m http.server 8080 --directory dist-webhid   # needs http://localhost
cd dist-webhid && npx wrangler deploy
```

Two limits of the static build, both inherent:

- Animations only run while the tab is visible. A still colour persists after the
  tab closes, because one report is enough; a moving effect does not.
- It cannot remap keys or run macros. Those need a system-wide event tap, which
  only a native process can install.

## Why the native build cannot be a Worker

A Worker runs on Cloudflare's machines, which have no USB bus and no keyboard
attached. The *native* build therefore cannot be deployed: it is the service that
opens the device, renders frames at 30 fps and installs the event tap for
remapping and macros.

What can be deployed is the static WebHID build above, because in that shape the
browser is the client *and* the owner of the device — the page runs on the
visitor's machine, next to their own keyboard.

## Adding a static app

The domain lives in config, so a new project means a name and a hostname.

1. `deploy/workflows/deploy-static.yml` in your org workflow repo
   (`azmuth/.github`).
2. In the project, a three-line caller:

```yaml
name: deploy
on: { push: { branches: [main] } }
jobs:
  ship:
    uses: azmuth/.github/.github/workflows/deploy-static.yml@main
    with:
      project: hld
      domain: hld.azmuth.dev
    secrets: inherit
```

3. Done. The workflow writes the `wrangler.jsonc` with the custom domain and
   deploys; Cloudflare creates the DNS record and certificate.

Keep hostnames at the first level (`hld.azmuth.dev`, not `beta.hld.azmuth.dev`):
free Universal SSL covers the root and first-level subdomains only.

## Adding Rig

```sh
# on the Mac, bound to loopback
RIG_TOKEN=$(openssl rand -hex 16) ./rig --no-open

# on the Mac, exposing only that port
cloudflared tunnel --url http://127.0.0.1:7311
```

Then point `rig.azmuth.dev` at the tunnel (`cloudflared tunnel route dns`), and
put Cloudflare Access in front of it. With Access authenticating the browser,
`RIG_TOKEN` becomes redundant — drop it and let Access be the gate.

Releases for Rig are a tag away: pushing `v*` runs
[`.github/workflows/release.yml`](../.github/workflows/release.yml), which tests,
builds, bundles `Rig.app` and publishes both to GitHub Releases.

## The registry

The landing page reads a static manifest — no database, no build step:

```js
export const projects = [
  { name: "One More Dimension", slug: "dimension", domain: "dimension.azmuth.dev", category: "Mathematics", status: "live" },
  { name: "Rig",                slug: "rig",       domain: "rig.azmuth.dev",       category: "Developer Tools", status: "live" },
  { name: "Azmuth HLD",         slug: "hld",       domain: "hld.azmuth.dev",       category: "System Design", status: "live" },
];
```

Adding a project to the workbench is adding one object. The `status` field is
worth having from day one: it lets you publish a project before it is finished
without pretending otherwise.

## Analytics

One GA4 property can collect every subdomain — the same `G-` tag with the
hostname as a dimension. Keep it off `rig.`: that page shares an origin with an
API that drives hardware, so any script loaded there can call it. Put the tag on
the landing page and the static apps instead.
