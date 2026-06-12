# WideScope marketing site

A self-contained, zero-build landing page for [WideScope](https://widescope.soumendrak.com) — matching the product's local-first ethos: one HTML file, one SVG, no framework, no bundler.

## Preview

```bash
python3 -m http.server 8000 --directory site
# → http://localhost:8000
```

## Deploy

Point any static host (Cloudflare Pages, Netlify, GitHub Pages) at the `site/` directory, or add it as a second Cloudflare Pages project. Everything is relative-path; no build step required.

## Notes

- Fonts: Bricolage Grotesque + Spline Sans Mono via Google Fonts (the only external requests).
- All animations are CSS-first, with small vanilla-JS helpers (scroll reveals, counters, tab switching, the A/B diff cycler). `prefers-reduced-motion` is respected throughout.
- The trace data shown in the hero and diff sections is illustrative, hand-tuned to tell a realistic RAG-agent story.
