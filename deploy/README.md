# Hosting several apps under azmuth.dev

One rule drives every decision: **an app that touches hardware has to run on the
machine the hardware is attached to.** Rig controls a keyboard over USB, so it
runs on the Mac. Everything else can live on a VPS or wherever you like.

## Use a subdomain per app

```
azmuth.dev            landing page (static)
rig.azmuth.dev        → tunnel → this Mac, 127.0.0.1:7311
notes.azmuth.dev      → tunnel → this Mac, 127.0.0.1:7400
static-thing.azmuth.dev → A record → your VPS
```

Subdomains rather than paths (`azmuth.dev/rig`) for three reasons:

1. **Rig assumes it owns the root.** Its assets are `/app.css`, `/app.js` and its
   API is `/api/*`. Serving it under a prefix would need base-path support in the
   HTML, the CSS and every fetch — doable, but it buys nothing here.
2. **Origins stay separate.** Cookies, `localStorage` and any session are scoped
   per host, so one app cannot read another's state. With paths they share an
   origin and everything collides.
3. **Access policies are per hostname.** You can put `rig.azmuth.dev` behind a
   stricter rule than the public landing page.

## Put Cloudflare in front

```
DNS (Cloudflare)  →  Tunnel  →  cloudflared on the Mac  →  127.0.0.1:7311
                              →  cloudflared on the VPS  →  127.0.0.1:7400
```

- Run `cloudflared` as a service on each machine (`brew services start cloudflared`).
- Add a rule per app in `cloudflared.example.yml`, then
  `cloudflared tunnel route dns <tunnel> rig.azmuth.dev` to create the CNAME.
- Keep every app bound to `127.0.0.1`; the tunnel is the only way in, so the
  ports are not exposed to your LAN.
- Cloudflare terminates TLS at the edge, so the origin needs no certificate.

## Authentication: do it at the edge

Rig's own token is a single shared secret with no accounts. If Cloudflare Access
sits in front of `rig.azmuth.dev`, the browser is already authenticated before it
reaches the app, and the token becomes a second, redundant gate — you can drop it
(`RIG_TOKEN` unset, bind loopback) and let Access do the work:

- Access → Applications → Self-hosted → `rig.azmuth.dev`
- Policy: allow your email, one-time PIN or Google/GitHub identity
- Session duration: a day or two

For the public landing page, either no Access policy or a bypass policy.

## Isolation worth knowing about

Rig's page runs on the same origin as its API. Anything loaded into that origin —
a third-party script, an injected ad, a compromised dependency — can call the
API with the user's cookie and drive the keyboard. Access limits *who* gets in;
it does not limit what runs in the page. That is the argument for keeping the
control surface free of third-party scripts, and for keeping the analytics tag
on the landing page rather than on `rig.` (the tag in this repo is skipped on
localhost but does load on any other host — worth overriding if you publish Rig
itself).

## Adding the next app

1. Run it on the machine that has what it needs, bound to loopback.
2. Add an ingress rule for `<app>.azmuth.dev`.
3. `cloudflared tunnel route dns <tunnel> <app>.azmuth.dev`.
4. Restart `cloudflared` and add an Access policy if it is not public.

## If you would rather not use Cloudflare

Caddy on a VPS gets you the same shape with automatic certificates:

```
rig.azmuth.dev {
    reverse_proxy 100.x.y.z:7311   # the Mac over Tailscale
}
```

The Mac's port stays bound to loopback and Tailscale provides the private link.
This costs one more moving part (Tailscale) but removes the tunnel dependency.
