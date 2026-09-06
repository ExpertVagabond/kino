# KINO FOR PUBLISHERS — v2

**Own the ad stack. Stop selling video like audio.**
Rust-native vertical video player + IAB-grade ad pipeline. Built for podcast networks moving to video without ad tech of their own.

> Prepared for **[NETWORK NAME]** — Spanish-language podcast network migrating to vertical video
> Prepared by **Purple Squirrel Media LLC** · 2026
> Status: Discussion document — not a contract

---

## Resumen Ejecutivo (ES)

Su red está migrando a video pero sigue monetizando como audio. SiriusXM vende su inventario como podcast audio. YouTube AdSense le paga lo que decide pagarle. El problema real no es JWPlayer o Brightcove — es que la red **no tiene tecnología publicitaria de video propia**. Está cautiva de quien la vende.

**Kino** es un reproductor de video nativo en Rust, formalmente verificado, con la fundación de streaming (HLS, ABR, captions, edge-WASM) ya construida y probada. Lo que falta — la capa de monetización IAB-compliant (VAST 4.x, VMAP, OMID, SSAI, UX vertical 9:16, overlays CTA contextuales) — se construye en **8-10 semanas con un ingeniero**.

**Diferenciador estructural:** la huella digital de audio de Kino (FFT espectral, 100× tiempo real) clasifica segmentos de podcast — entrevista, monólogo, lectura de patrocinador, cama musical — en tiempo real, y dirige anuncios contextualmente al segmento. Para contenido en español donde la calidad de transcripción varía, esta es la única forma confiable de hacer targeting por tipo-de-contenido sin depender de metadata.

**Dos modelos de precio:**
- **Tarifa fija** ~$80-120K USD por el MVP de 8-10 semanas, código fuente al cliente
- **Retainer + rev-share** $15K USD/mes + porcentaje del fill incremental sobre el piso actual de Sirius/YouTube — alineado con el upside, sin necesidad de capital inicial

**Conditional clave a resolver con [NETWORK NAME] antes de cotizar:** ¿la red controla los archivos master de los creadores, o los creadores suben MP4s pre-renderizados con anuncios ya integrados? La respuesta determina si SSAI (server-side ad insertion, anti-adblock) está en el alcance o si el producto es overlays client-side. Ambos productos tienen valor; el alcance y precio cambian.

---

## 1. The Real Problem (Not What It Looks Like)

The surface ask is "replace JWPlayer or Brightcove." The actual situation is sharper:

The network doesn't have a JWP or Brightcove contract because **they don't have video ad tech at all.** SiriusXM sells their inventory like audio because that's the only ad stack they have access to. YouTube takes whatever cut it wants because the network has no alternative monetization surface.

This isn't a player-replacement project. It's a greenfield ad-tech buildout disguised as a player project. That changes the pitch:

- The competitive comparison isn't "Kino vs JWPlayer." It's "owning the ad stack vs letting Sirius sell audio inventory while YouTube monetizes the video."
- There's no incumbent to displace at the player layer. The network has nothing.
- The upside isn't "save on JWP per-stream fees." It's the entire video ad CPM stack that currently flows to YouTube and nobody.

At 10-12M views per video, the inventory the network is leaving on the table is structurally larger than any per-stream license fee.

## 2. The Unlock

Own the player, the ad decisioning layer, and the audience routing.

- **Own player, own IP, no per-stream tax.** Build cost is paid once.
- **CTAs route into your owned video, not YouTube next-up.** Every play becomes audience retention instead of leakage.
- **Creator whitelist becomes an amplification graph.** Cross-promotion enforced at the player level — not editorial. Reach compounds across the network.
- **IAB-compliant ad pipeline you control.** OMID viewability, VAST 4.x creatives, VMAP scheduling. Direct demand from advertisers + programmatic via SSPs (Magnite / PubMatic / FreeWheel) + your own dynamic insertions, in one stack.
- **Audio-fingerprint contextual targeting.** Match advertiser category to podcast segment audio — a structural moat JWP and Brightcove cannot reproduce, and one that survives Spanish-language transcript quality issues that text-based targeting depends on.

## 3. What Kino Brings Today (Verified, in Repo)

Kino is a 7-crate Rust workspace, MIT/Apache-2.0 dual licensed, with a verified test suite. The streaming foundation that took JWPlayer 15 years to build is already done:

| Capability | Detail |
|---|---|
| **Streaming core** | HLS / DASH manifest parsing, BOLA + throughput-based ABR, buffer management, segment validation |
| **Captions** | WebVTT, SRT — required for Spanish-language accessibility |
| **Player surfaces** | WASM (browser via MSE), React component, embeddable JS, Tauri desktop, native GStreamer |
| **Audio intelligence** | FFT spectral analysis (rustfft), SHA-256 fingerprinting via spectral peak constellation, ML auto-tagging (genre / mood / BPM / speech-vs-music) |
| **Reliability** | 8 TLA+ formal specifications, 25+ verified invariants checked in CI |
| **Performance** | 4ms cold start, 100× realtime audio processing, zero-copy hot paths, zero memory leaks |
| **Deploy targets** | Native binary (Linux/macOS/Windows), WASM (Cloudflare Workers compatible), Python (PyO3), Docker |
| **AI ops** | MCP server with 8 tools — Claude/GPT can drive analysis, QC, and content tagging |

DRM (Widevine/FairPlay/PlayReady/ClearKey) also exists in `kino-core` but is **out of scope for the MVP** — organic content doesn't need it. It's available later for premium content licensing if the network signs syndication deals.

## 4. MVP Scope — 8-10 Weeks, One Engineer

Tight scope. Ship the smallest thing that monetizes, then iterate.

| Week | Deliverable |
|---|---|
| **1-2** | VAST 4.x parser + VMAP 1.0 scheduler in `kino-core`. No solid open-source Rust VAST parser exists — this is real IP leverage. |
| **3-4** | Ad insertion runtime. SSAI (HLS, SCTE-35 cues) **if** the network controls master files; CSAI + dynamic overlay framework if not. Decision gated by Section 6 below. |
| **5-6** | Web player — WASM core + thin React shell over MSE, vertical 9:16 layout, CTA overlay framework with cue-point triggers, OMID 1.4 viewability via IAB reference SDK. |
| **7-8** | iOS + Android wrappers around the same Rust core (Section 5 architecture). |
| **9-10** | Creator whitelist gating at manifest level, ad ops dashboard, fill-rate / eCPM / completion-rate telemetry. |

### Deliberately out of MVP

- **DASH parsing** — HLS-only covers iOS natively and works in MSE-supporting browsers. DASH transcode happens at ingest if needed; not a player concern at v1.
- **DRM** — organic creator content doesn't need it. The crate has it; we ship it when premium syndication signs.
- **VPAID 2.0 / SIMID** — defer to v1.1. Interactive ad units are a small fraction of demand at MVP; most VAST creatives are linear.
- **Header bidding (Prebid.js)** — start with a VAST proxy to whichever SSP the network brings (Magnite, PubMatic, FreeWheel — all speak VAST). Add Prebid wrapper after first buyer signs and demand competition matters.
- **GAM bridge** — same logic. Add when direct-sold inventory needs to coexist with programmatic in one waterfall.

## 5. Architecture

**Rust core (the moat).** One crate owns manifest parsing/rewriting, VAST/VMAP resolution, ad decisioning, SSAI stitching, beacon firing, and audio fingerprinting. Compile targets:

- **wasm32** → web (thin JS/React shell over MSE + hls.js)
- **iOS** via `cargo-lipo` → Swift wrapper around AVPlayer (Apple handles decode, Rust handles ad logic + manifest + beacons)
- **Android** via JNI → Kotlin wrapper around ExoPlayer with Rust ad logic
- **Server** → same crate compiled native, runs the SSAI stitcher behind CF/CDN

The shape: **Rust owns ad logic and manifests; platform-native players handle decode.** A Rust-native decoder via WGPU or ffmpeg-rs is a 12-month detour that adds nothing for monetization. Don't fight the platforms on decode.

**SSAI service.** Rust + Axum, segments on R2/S3, per-session manifests with ad pods spliced at SCTE-35 cues, impression beacons fired server-side. Cloudflare Workers at the edge for sub-100ms manifest TTFB.

**Edge performance moat:** Workers + wasmtime gives **<10ms manifest rewrites**. AWS MediaTailor (the SSAI incumbent) sits at **50-200ms**. For LATAM audiences hitting US edges, that latency differential compounds across every ad break.

**Ad decisioning.** Starts as a VAST proxy to whatever SSP the network brings. Contextual targeting layers on top later — feed audio-fingerprint segment classification + creator metadata to a ranker, pick the highest-eCPM creative compatible with the segment type.

## 6. The Critical Diagnostic to Resolve

**Does the network control creator master files, or do creators upload pre-baked MP4s with ads burned in?**

This is the single most important question, and it determines whether SSAI is even a product.

- **If master files are network-controlled:** full SSAI stitcher at edge, ads spliced into the manifest per session, un-blockable, no separate client-side ad request. This is the strong product.
- **If creators upload pre-baked MP4s:** SSAI is off the table. The product becomes CSAI runtime + dynamic CTA/overlay framework. Still valuable — overlay deep-linking to network-owned destinations, contextual creative swap, viewability tracking — but smaller scope.

**Both products have a real business.** Scope and price change. Frankie needs to confirm this with the network before we quote.

Secondary diagnostics that change architecture:

- **Distribution surface.** In-app only, embedded web on creator/publisher sites, or both? Web embed is highest-revenue but requires IAB Tech Lab certification, GPP/TCF consent strings, and CMP integration. Adds ~2 weeks.
- **Demand side.** Network bringing buyers, or do we broker SSP integration? Sourcing programmatic demand is a separate workstream and not in the MVP scope.
- **Spanish-language fill.** Confirm Magnite or PubMatic has Spanish inventory at the network's scale before architecting around a specific exchange.

## 7. Economics & Two Pricing Models

### Model A — Fixed fee

- **MVP:** $80-120K USD, 8-10 weeks
- Source code transfers to the network for Spanish-language podcast vertical use
- PSM retains rights to license the ad-monetization layer (non-Spanish, non-podcast verticals) to other clients
- v1.1 additions (VPAID, Prebid, GAM, DRM) priced per milestone

### Model B — Retainer + rev-share (recommended for this customer)

- **$15K USD / month retainer**, ongoing
- **Plus rev-share on incremental fill** above the current Sirius + YouTube baseline (terms TBD; meaningful upside cap)
- Network commits zero capital outlay
- PSM upside scales with actual viewership and fill — fully aligned

**Why Model B is the better deal for both sides:**
- The network "doesn't understand how to live outside Google AdSense" (Frankie's words). They have no cash to spend on a fixed-fee build, but the upside is enormous if execution lands.
- Rev-share take-rate scales with viewership. At 10-12M views per video, the SSAI take-rate compounds. Better than any fixed fee.
- Aligns our incentives with theirs. We don't get paid for shipping a player; we get paid for shipping a player that monetizes.

### Cost reference (the JWP/BC counterfactual)

At 10M MAU, JWPlayer's per-MAU + per-1k-play pricing lands around **$600K-$1.8M/year perpetual**. Brightcove's enterprise SaaS sits similar with multi-year lock-in. Kino's MVP build cost is recovered in **<2 months** at that volume — but the bigger story is the incremental video ad CPM that currently flows to YouTube and SiriusXM's audio-only stack.

## 8. Differentiation (Why Kino, Specifically)

**Three real reasons Rust matters here — not Rust as a fashion choice:**

1. **Manifest manipulation at edge scale.** Workers + wasmtime give single-digit-ms ad-pod rewrites at the CDN edge. Node-based SSAI stacks (including AWS MediaTailor) sit at 50-200ms. Latency differential compounds across every ad break, every viewer, every day.
2. **One core, all platforms.** Same Rust crate runs in browser, iOS, Android, and edge. JS-based players force parallel Swift/Kotlin reimplementations of ad logic — that's where bugs and revenue leakage live in production. One source of truth for the decisioning code is a structural advantage.
3. **Memory safety on untrusted XML.** VAST creatives in the wild ship malformed payloads constantly. Rust parsers don't crash the player. That's directly measurable as completed-view rate — fewer crashes equal more completed impressions equal more revenue.

**Kino-specific moats on top of Rust:**

- **Audio-fingerprint contextual targeting.** FFT spectral classification of podcast segments (interview / monologue / sponsor read / music bed) in real time. Match advertiser category to audio content. Structurally impossible on JWP/BC. For Spanish-language content where transcript accuracy varies, audio-feature targeting is the only reliable approach.
- **Formal verification.** 8 TLA+ specifications, 25+ mathematically proven invariants. When the network pitches Tier-1 advertisers or considers an exit, "TLA+-verified player" is a real procurement story. No competitor has this.
- **MCP-native ad ops.** AI agents draft creative variants in Spanish, schedule placements, run A/B tests, and analyze performance — without growing an ad ops team to match SiriusXM's. Compete on automation, not headcount.

## 9. The Ask

Structured to de-risk for the network and align with their reality (no cash, slow internal sale, education-heavy):

1. **Discovery call** with [NETWORK NAME] to resolve the master-files diagnostic (Section 6) and confirm SSP/demand setup. ~1 hour.
2. **Pilot MVP** on one creator's catalog as proof. 8-10 weeks. Pricing per Model A or Model B above — Model B recommended given Frankie's read on the customer.
3. **Decision gate** after MVP — continue to v1.1 (VPAID, Prebid, GAM, contextual targeting tier, SSAI hardening) on milestone-based terms, or fork the code and walk away with full source.
4. **Network keeps source code** for the Spanish-language podcast vertical. PSM retains rights to license the ad-monetization layer (non-Spanish, non-podcast verticals) to other clients — this is what funds the next phase of Kino across all customer types.

**Timeline:** if the network commits in [June], MVP demo is live by [August]; full v1 player with v1.1 differentiators by [Q4].

---

## Technical Appendix

- **Repository:** github.com/ExpertVagabond/kino (private, source available under NDA)
- **License:** MIT OR Apache-2.0 (existing code) — custom license for ad-monetization layer per terms
- **Stack:** Rust 2021 edition, Tokio async runtime, Symphonia audio decode, Ring crypto/DRM, Nom parser combinators, Axum (server)
- **Hardware acceleration:** VA-API, VideoToolbox, NVDEC, D3D11VA auto-detection
- **CI/CD:** GitHub Actions running fmt, clippy, build, test across {ubuntu, macos, windows} × {stable, beta}, plus TLC model checking for all 8 TLA+ specs on every push
- **Edge deployment:** Cloudflare Workers (WASM) for ad decisioning + manifest rewrites; R2/S3 for segment origin; native binary for the SSAI stitcher service

---

*Kino is built by [Purple Squirrel Media LLC](https://purplesquirrelmedia.io).*
*This document is a discussion artifact, not a contractual offer. Final terms negotiated in MSA.*
