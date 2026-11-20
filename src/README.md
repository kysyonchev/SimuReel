# Film Lab

A small Rust RAW photo editor built around film simulations.

- Opens RAW (DNG, NEF, ARW, CR2, ORF, RAF, RW2, PEF, SRW … – **not CR3**), JPEG, PNG, TIFF (or drag & drop)
- 130+ film stocks across Kodak, Fujifilm, Ilford, Kentmere, Harman, Agfa, CineStill, Lomography,
  Rollei, Foma, Adox, Orwo, Konica, Svema, Polaroid and more
- Each film is shown as a drawn canister (or instant-film pack); filter by brand, type, or search
- Sliders: Film strength, Exposure, Contrast, Highlights, Shadows, Temperature, Tint, Saturation,
  Grain, Glow (halation), Vignette – double-click a slider to reset it
- Hold **Space** (or tick *Show original*) to compare before/after
- Export renders at full resolution (JPEG q95 or PNG)

## Run

```
cargo run --release
```

Needs a recent stable Rust toolchain. On Linux you may need the usual windowing/GTK/portal packages.

## Adding / tweaking films

Everything lives in `src/films.rs`. One line per film:

```rust
c("Kodak", "Portra 400", 400, ColorNeg, 0xF0C75E /*canister*/, 0xB83227 /*accent*/,
  [contrast, saturation, warmth, tint, fade, grain], shadow_tone_rgb, highlight_tone_rgb)
```

Chain `.hal(x)` for halation, `.vig(x)` for vignette, `.mat([[..];3])` for a colour matrix
(false colour / redscale / LomoChrome), and use `bw(...)` with a channel mixer for black & white.

The looks are hand-tuned approximations of each stock's character, not official profiles.
