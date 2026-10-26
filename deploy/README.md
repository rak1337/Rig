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

## Why Rig cannot be a Worker

A Worker runs on Cloudflare's machines with no USB bus, no IOKit, no HID
permissions and no persistent process. Rig is not a web page that talks to a
service — it *is* the service: it opens the keyboard, renders 132-cell frames at
30 fps and writes them over USB. Deploying `dist/` for Rig would publish a UI
whose every request fails, because there is no backend anywhere near a keyboard.

So `rig.azmuth.dev` is a tunnel to the Mac, and that is the only exception. If
you want Rig's UI to be publicly viewable without hardware, that is a separate
demo build with the API stubbed out — not a deployment of the real thing.

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

Releases for Rig are a tag away: `deploy/workflows/rig-release.yml` builds and
attaches a macOS binary on `v*`.

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
