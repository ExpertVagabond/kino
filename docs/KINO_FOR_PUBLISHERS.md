# KINO FOR PUBLISHERS

**Own your video monetization stack.**
Rust-native vertical video player with IAB-compliant ad insertion, formally verified, built for podcast networks moving to video.

> Prepared for **[NETWORK NAME]** — Spanish-language podcast network migrating to vertical video
> Prepared by **Purple Squirrel Media LLC** · 2026
> Status: Discussion document — not a contract

---

## Resumen Ejecutivo (ES)

Su red está migrando a video, pero lo monetiza como audio — vendiendo a través de SiriusXM y dependiendo de YouTube AdSense. Con 10-12 millones de vistas por video, cada centavo que paga a JWPlayer o Brightcove se compone perpetuamente, y los CTAs envían su audiencia al algoritmo de YouTube, no al suyo.

**Kino** es un reproductor de video nativo en Rust, formalmente verificado, listo para reemplazar JWPlayer/Brightcove.

**Lo que existe hoy (probado):** parsing HLS/DASH, ABR adaptativo (BOLA + throughput), DRM (Widevine/FairPlay/PlayReady), huellas digitales de audio en tiempo real (100× tiempo real vía FFT en Rust), reproducción WASM en edge (compatible con Cloudflare Workers), 8 especificaciones TLA+ verificadas matemáticamente, 4ms de tiempo de inicio, cero fugas de memoria.

**Lo que construimos para usted en 12-16 semanas:** capa de monetización IAB-compliant — VAST 4.x, VMAP, OMID 1.4, SCTE-35, SSAI, UX vertical 9:16, overlays CTA contextuales, integración Prebid.js / Google Ad Manager.

**Diferenciador único:** nuestra huella digital de audio puede clasificar segmentos de podcast (entrevista / monólogo / lectura de patrocinador / cama musical) en tiempo real y dirigir anuncios contextualmente. Esto es estructuralmente imposible en JWPlayer o Brightcove. Para una red en español, desbloquea targeting de idioma-de-contenido sin depender de metadata.

**Economía:** tarifa fija (~$60-90K USD para Fase 1, 6 semanas), código fuente suyo, cero tarifas perpetuas por stream. Punto de equilibrio en menos de 2 meses de tarifas de JWPlayer al volumen actual.

---

## 1. The Problem You're Solving

You're publishing video at YouTube scale (10-12M views per video) but monetizing it through audio-era channels — SiriusXM sells your inventory like audio, YouTube AdSense takes the rev share, and your CTAs end up routing your own audience into YouTube's recommendation algorithm, not back to you.

The structural issues:

- **JWPlayer / Brightcove are perpetual taxes.** Both are per-stream / per-MAU pricing models. At your volume, the run-rate compounds into seven figures annually without giving you any IP back.
- **No control over the ad decisioning layer.** Direct-sold inventory has to round-trip through their ad servers; programmatic demand is gated behind their pre-bid stack.
- **Your creator whitelist is just a contact list.** Without a player you own, you can't enforce cross-promotion, route traffic between creators, or amplify reach inside the network.
- **Vertical video is a different monetization shape.** Mobile-first, swipe-feed, with shorter dwell time and higher ad density tolerance — and JWP/Brightcove are still optimized for the landscape-first OTT use case.

## 2. The Unlock

Stop renting infrastructure. Own the player, the ad pipeline, and the audience routing.

- **One player, your IP, no per-stream fees.** You stop paying JWP/Brightcove forever. The build cost is paid once.
- **CTAs route into your owned video, not YouTube next-up.** Every play becomes an audience-retention event instead of a leak to YouTube.
- **Creator whitelist becomes a real amplification graph.** Cross-promotional CTAs are enforced at the player level, not the editorial level. Reach compounds across the network.
- **IAB-compliant ad pipeline you control.** Direct-sold + programmatic + dynamic insertions in one stack, with full OMID viewability and verification for Tier-1 advertiser procurement.
- **Audio-fingerprint contextual targeting.** Match advertiser category to podcast segment audio — a structural moat JWP/BC can't reproduce.

## 3. What Kino Brings Today (Verified)

Kino is a 7-crate Rust workspace, production-grade, MIT/Apache-2.0 dual licensed, with a verified test suite. Components that exist today:

| Capability | Detail |
|---|---|
| **Streaming core** | HLS / DASH manifest parsing, BOLA + throughput-based ABR, buffer management, segment validation |
| **DRM** | Widevine, FairPlay, PlayReady, ClearKey — full handshake validation in CI |
| **Captions** | WebVTT, SRT — important for Spanish-language accessibility compliance |
| **Player surfaces** | WASM (browser via MSE), React component, embeddable JS, Tauri desktop app, native GStreamer |
| **Audio intelligence** | FFT spectral analysis (rustfft), SHA-256 fingerprinting via spectral peak constellation hashing, ML auto-tagging for genre / mood / BPM / speech-vs-music |
| **Reliability** | 8 TLA+ formal specifications, 25+ verified invariants checked in CI (player state, ABR, buffer, DRM, concurrent streaming, captions, playlist, full-system composition) |
| **Performance** | 4ms cold start, 100× realtime audio processing, zero-copy hot paths, zero memory leaks |
| **Deploy targets** | Native binary (Linux/macOS/Windows), WASM (Cloudflare Workers compatible), Python (PyO3), Docker |
| **AI ops** | MCP server with 8 tools — Claude/GPT/custom agents can drive analysis, QC, and content tagging without writing code |

**What this means for the network:** the streaming foundation that JWPlayer and Brightcove spent 15 years building is already done, formally verified, and edge-deployable.

## 4. What We Build for You (12-16 Weeks, Phased)

The ad-monetization layer is the missing third. Phased so you can de-risk after each milestone.

### Phase 1 — Core ad pipeline (5-6 weeks, fixed fee)

- VAST 4.x parser + ad pod scheduler in `kino-core`
- VMAP 1.0 cue timeline (pre/mid/post-roll)
- CSAI runtime (client-side ad insertion)
- OMID 1.4 integration via IAB reference SDK (the IAB-compliance piece — viewability, verification, fraud signals)
- Demo: full pre/mid/post-roll playback on one of your creator streams

**Deliverable:** running player, IAB-compliant ad delivery, demonstrable on a single test channel.

### Phase 2 — Vertical UX + monetization stack (4-5 weeks, milestone-based)

- 9:16 vertical swipe-feed React/WASM player UI
- Cue-point overlay framework — contextual CTAs that route to owned video, not external
- VPAID 2.0 / SIMID iframe shim for interactive ad units
- Prebid.js header bidding integration (programmatic demand stack)
- Google Ad Manager bridge (direct-sold + programmatic unified)

**Deliverable:** player that monetizes at parity with JWPlayer for direct-sold and programmatic inventory.

### Phase 3 — Differentiators (3-5 weeks, milestone-based)

- **Audio-fingerprint-driven contextual ad targeting** — segment-level classification (interview / monologue / sponsor read / music bed) feeds the ad decision in real time. This is the unique moat.
- SCTE-35 ad marker parsing + server-side ad insertion (SSAI) stitcher — defeats ad blockers, eliminates client-side stitching artifacts
- Creator whitelist amplification — enforce cross-promotional CTAs at the player level, with reach analytics per creator pair
- MCP-driven ad ops dashboard — agentic creative drafting, A/B scheduling, and performance analysis without hiring an ad ops team

**Deliverable:** a player no competitor can match on Spanish-language contextual targeting, with creator-graph amplification built in.

## 5. The Economics

At 10-12M views per video, the per-stream pricing of JWPlayer and Brightcove compounds fast.

| Provider | Pricing shape | Annual cost at 10M MAU |
|---|---|---|
| JWPlayer | Per-MAU + per-1k-plays + per-feature add-ons | ~$600K - $1.8M perpetual |
| Brightcove | Enterprise SaaS, multi-year contract | ~$500K - $1.5M perpetual |
| **Kino** | **Fixed-fee build, then $0** | **~$60-90K Phase 1, ~$150-250K total v1** |

**Break-even on Phase 1 alone:** less than 2 months of JWPlayer fees at current network volume.
**5-year TCO advantage:** $3M-$8M depending on growth assumptions.
**Plus:** every line of code is owned by the network. Acquisition optionality, exit story, no platform risk.

## 6. The Differentiation (Why This vs JWP/BC)

- **Spanish-language contextual ad targeting via audio.** Kino's FFT fingerprinting classifies podcast segments by content type and mood. Match advertiser category to audio content in real time. JWP and BC target on session/cookie data — they have no equivalent to this. For a Spanish-speaking audience, this is also the cleanest way to enforce language-of-content targeting without trusting metadata.
- **Formally verified player = procurement story.** When the network pitches Tier-1 advertisers (or gets acquired), a TLA+-verified player with 25+ mathematically proven invariants is a real moat. JWP and BC have no formal verification.
- **MCP-native = AI-driven ad ops.** Agents can draft creative variants in Spanish, schedule placements, run A/B tests, and analyze performance — without building an ops team to match SiriusXM's. The network competes on automation, not headcount.
- **Edge-deploy via Cloudflare Workers (WASM).** Ad-decisioning runs at the CDN edge, sub-100ms globally. JWP and BC run ad decisions through their own US-hosted ad servers — slower for LATAM audiences.
- **Rust + WASM = mobile battery advantage.** Vertical video is 90%+ mobile-consumed. Lighter player → longer sessions → more ad inventory per user. Compounds at scale.

## 7. The Ask

This is structured to de-risk for the network:

1. **Pilot Phase 1 (fixed fee, 6 weeks)** — VAST + VMAP + OMID running on one creator's catalog as proof
2. **Decision gate after Phase 1** — continue to Phase 2/3 on milestone-based terms, or fork the code and walk away with full source
3. **Source code transfers to the network** for Spanish-language podcast vertical use
4. **PSM retains rights** to license the ad-monetization layer (non-Spanish, non-podcast verticals) to other clients — this is what makes the fixed-fee build economics work for us

**Timeline:** if the network commits in [June], Phase 1 demo is live by [August]; full v1 player (Phases 1-3) in production for the network by [Q4].

---

## Technical Appendix

- **Repository:** github.com/ExpertVagabond/kino (private, source available under NDA)
- **License:** MIT OR Apache-2.0 (existing code) — custom license for ad-monetization layer per terms
- **Stack:** Rust 2021 edition, Tokio async runtime, Symphonia audio decode, Ring crypto/DRM, Nom parser combinators
- **Hardware acceleration:** VA-API, VideoToolbox, NVDEC, D3D11VA auto-detection
- **CI/CD:** GitHub Actions running fmt, clippy, build, test across {ubuntu, macos, windows} × {stable, beta}, plus TLC model checking for all 8 TLA+ specs on every push

---

*Kino is built by [Purple Squirrel Media LLC](https://purplesquirrelmedia.io).*
*This document is a discussion artifact, not a contractual offer. Final terms negotiated in MSA.*
