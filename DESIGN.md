---
name: Cadence native desktop
description: Native Rust listening workspace in the established Apple Music Windows visual world.
colors:
  primary: "#fa3b56"
  dark-content: "#1d1c1e"
  dark-navigation: "#18171a"
  dark-transport: "#232124"
  dark-text: "#f2f0f2"
  dark-muted: "#aaa6ae"
  light-content: "#fdfcfc"
  light-navigation: "#f2f0f2"
  light-transport: "#f7f5f7"
  light-text: "#232124"
  light-muted: "#635f66"
  white: "#ffffff"
typography:
  headline:
    fontFamily: "egui proportional default"
    fontSize: "30px"
  title:
    fontFamily: "egui proportional default"
    fontSize: "21px"
  section:
    fontFamily: "egui proportional default"
    fontSize: "18px"
  body:
    fontFamily: "egui proportional default"
    fontSize: "14px"
  label:
    fontFamily: "egui proportional default"
    fontSize: "13px"
rounded:
  row: "4px"
  artwork-icon: "5px"
  navigation: "6px"
spacing:
  item-x: "8px"
  item-y: "7px"
  button-x: "10px"
  button-y: "6px"
  panel: "12px"
  compact-content: "16px"
  content: "28px"
components:
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.white}"
    typography: "{typography.label}"
    padding: "6px 10px"
    height: "34px"
    width: "116px"
  icon-button:
    rounded: "{rounded.artwork-icon}"
    size: "32px"
  navigation:
    rounded: "{rounded.navigation}"
    typography: "{typography.body}"
    height: "36px"
  track-row:
    rounded: "{rounded.row}"
    padding: "6px"
---

# Design System: Cadence native desktop

## Overview

**Creative North Star: "Apple Music Windows listening workspace"**

Mode: Operate. This is a code-led Rust rebuild under the user's pinned Apple Music reference and approved rebuild direction. The incumbent visual world applies: quiet neutral surfaces, restrained red selection, collapsible left navigation, persistent top transport, and readable music-library rows. A new-world seed and exploratory comp selection are inapplicable to this reference-led implementation; neither an invented seed nor an approved comp is claimed.

This refresh derives from `src/ui/components.rs`, `src/ui/mod.rs`, `src/ui/pages.rs`, and `src/ui/mini.rs`. Inspected native PNGs in `.impeccable/review/` include desktop Home, the final MiniPlayer, and revised 150%/200% app-zoom Home surfaces. The zoom captures show the transport and both Home actions within the window; at 200%, supporting copy continues below the viewport in the scrollable content region. These are scoped visual observations, not certification of complete feature parity, protected playback, account connection, or every interaction.

**Key Characteristics:**

- Persistent native transport and collapsible navigation.
- Flat neutral panels with a single restrained red accent.
- Real catalog artwork when available and an honest drawn music-note placeholder.
- Explicit local-file and catalog-preview states.

## Colors

Primary red signals selected navigation, the active song, selection, and the principal action. Neutral panels carry both appearances; album imagery supplies additional color.

### Primary

**Music red** (`primary`) fills primary actions and colors selected navigation and the active track title. Preserve source `ACCENT.gamma_multiply` operations: navigation (0.15), selected icon field (0.18), text selection (0.25), active track (0.08), and error field (0.10). These egui operations are not CSS alpha values.

### Neutral

**Content / navigation / transport** separate the canvas, sidebar, and player by tone in both appearances. **Text / muted text** separate names and actions from artist metadata, subtitles, and idle controls. **White** supplies the primary button label.

**The Selection Rule.** Reserve red for meaningful action and state; do not tint every surface or replace real cover art with brand-colored decoration.

## Typography

**Body and control font:** the shipped egui proportional default. No custom Apple/SF font is loaded or claimed. The YAML family identifies a native renderer role, not a browser font stack.

The hierarchy is functional: page headings exceed section titles, while song metadata and controls stay close to reading size. Strong emphasis is applied at individual call sites. Font weights and line-height metrics remain renderer-owned rather than fabricated numeric tokens.

- **Headline:** page heading.
- **Title:** sidebar identity and major library sections, often strong.
- **Section:** settings categories and album headings, often strong.
- **Body:** reading text and navigation labels.
- **Label:** buttons; row title and artist actions inherit this control role. The historical fixed 15px row-title claim is not current.

## Layout

Source dimensions are egui logical points, represented as px in the portable frontmatter. They are not physical screenshot pixels at every app zoom.

**FORM (verified from source):** expanded desktop navigation (220px), compact navigation (60px); desktop top transport (94px), compact top transport (116px); flexible content canvas. The main window defaults to 1240 × 820 with an 860 × 600 minimum. This replaces historical 176px navigation / always-116px player / 940 × 640 minimum.

Navigation expands only when manually enabled and root available width exceeds 950px, or 1120px while the queue is open. It scrolls vertically. After navigation consumes its width, transport height switches below 624px remaining width; its inner arrangement switches below 600px available width. Both panels have 12px inner margins.

Compact transport stacks three rows: previous/play/next plus queue/MiniPlayer; truncated track title plus volume; seek and elapsed/duration. This is implemented narrow/zoom behavior, not a separate mobile app.

Content padding is 28px, reducing to 16px below 600px available width. The queue normally uses a right panel, default 270px with a 220–350px range. Below 620px remaining width, an open queue occupies central content.

Home content scrolls vertically. Its empty state places wrapped import/search actions before supporting copy when content width is below 600px or available height below 320px. Roomier sizes use the spacious text-led desktop arrangement. Revised 150% and 200% native captures show both actions above supporting copy and the compact transport within the window. This confirms those captured layouts; it does not establish all screen or interaction behavior.

Library lists use virtualized 62px row slots, 6px inner margins, and 42px artwork. Above 680px row width, the album column takes 24% of width; otherwise it is omitted. Title and artist actions truncate. Shared item spacing follows the frontmatter; section gaps vary with content rather than a fabricated universal grid.

**The Persistent Player Rule.** Keep transport above changing page content and preserve reachable playback controls as width decreases.

## Elevation & Depth

The shell uses flat tonal layering and separators rather than decorative card shadows. Navigation selection and active rows receive low-intensity red fills. Menus, buttons, sliders, and focus treatment otherwise inherit egui theme defaults unless explicitly drawn. Their defaults are not extracted as a new shadow or motion system.

There is no continuous ornamental animation. Bounded repaint scheduling during idle/playback is not an animation-duration token.

## Shapes

Small corners support dense controls: track frames use the row radius, artwork and icon fields use the artwork/icon radius, selected navigation uses the navigation radius. Artwork stays square in proportion. Custom icons are painted geometry, with a 1.6px stroke where outlined; do not substitute emoji or font glyphs.

## Components

### Buttons

The principal action uses red with white text and a minimum 116 × 34px footprint; frontmatter width/height describe this minimum, not a forced fixed size. Shared padding is 10px horizontally and 6px vertically. Secondary buttons inherit egui styling. No custom primary hover color or corner token is claimed.

### Icon controls

A 32px square click region contains drawn geometry inset by 4px. Selected, hovered, or focused controls receive a rounded field; selection also changes the icon to red. Disabled transport draws muted and provides hover information without click actions. Widget labels and hover text name the controls.

### Inputs / Fields

Native single-line fields include the sidebar search (30px high), catalog search (36px high), and library filter (180 × 28px). Theme defaults own their background, corners, and focus stroke; no custom input palette is claimed.

### Navigation

Rows fill sidebar width and are 36px high. Expanded rows show a drawn icon and 14px label; compact rows retain the icon and hover label. Selected text/icon use red over a softly rounded selection field. Hover and keyboard focus use the theme's hovered fill.

### Track rows / artwork

Rows show actual artwork when fetched, otherwise the drawn music-note placeholder on a neutral square. Title and artist are actionable, with optional album column, visible source kind, play/pause, and a More menu for queue/favorites/playlists. Active rows use subtle red fill and red title. Source type stays readable: catalog previews are not full subscription streams.

### Persistent transport

Desktop transport groups previous/play/next at left, artwork/title/metadata and seek centrally, and queue/MiniPlayer/favorite/volume at right. The compact stack preserves previous/play/next, queue, MiniPlayer, title, volume, and seek; it does not duplicate all desktop metadata.

### MiniPlayer

`src/ui/mini.rs` owns a separate deferred native viewport, default 470 × 200px with a 390 × 180px minimum and an 18px inner margin. A truncated strong 17px title and truncated artist/source or idle/loading description precede a row of previous/play/next, queue, and volume controls, followed by elapsed time, seek, and duration. The inspected `mini.png` shows the idle state at the default geometry; it does not prove live audio or every minimum-size state.

The viewport renders a snapshot and sends typed actions to the single app controller. `App::logic` owns polling, command handling, and snapshot updates, allowing the architecture to keep controller work outside the main visible UI. Queue requests focus and restore the main viewport. This source-grounded separation is not independent playback state or a second audio engine; hidden-window runtime behavior still needs its own interaction evidence.

## Do's and Don'ts

### Do:

- **Do** keep dark and light panel relationships coherent with the extracted palette.
- **Do** use real cover art when available and the drawn music placeholder otherwise.
- **Do** preserve wrapped Home actions, scrollable navigation, and compact transport at increased zoom.
- **Do** distinguish local audio, previews, unavailable provider capabilities, and actual playback state.

### Don't:

- **Don't** claim an Apple account is connected or protected playback works from visual resemblance.
- **Don't** call this checkpoint complete Apple Music feature parity.
- **Don't** carry historical panel dimensions or palette values over current code.
- **Don't** canonize clipping, inaccessible actions, or stale captures as intended density.
