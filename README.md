# NET CONTROL a HAM Radio Net Check-in Logger

```
 ███╗   ██╗███████╗████████╗     ██████╗████████╗██████╗ ██╗
 ████╗  ██║██╔════╝╚══██╔══╝    ██╔════╝╚══██╔══╝██╔══██╗██║
 ██╔██╗ ██║█████╗     ██║       ██║        ██║   ██████╔╝██║
 ██║╚██╗██║██╔══╝     ██║       ██║        ██║   ██╔══██╗██║
 ██║ ╚████║███████╗   ██║       ╚██████╗   ██║   ██║  ██║███████╗
 ╚═╝  ╚═══╝╚══════╝   ╚═╝        ╚═════╝   ╚═╝   ╚═╝  ╚═╝╚══════╝
```

A terminal TUI for logging amateur radio net check-ins. Built in Rust with [Ratatui](https://ratatui.rs/). Fully themeable via the [Base16](https://github.com/chriskempson/base16) colour framework - 8 themes built in, unlimited custom themes supported.

---

## Features

- **Multiple saved nets** - store frequency, offset, PL tone, and club/association per net
- **Multi-session logging** - each net holds multiple dated sessions; browse and manage them independently
- **Digital net support** - flag a net as digital and select from 20 common modes (FT8, FT4, JS8Call, Winlink, DMR, D-STAR, and more), with a free-text notes field
- **Check-in logging** - log callsign, operator name, and remarks with automatic UTC timestamps
- **Callsign autocomplete** - remembers callsign/name pairs from previous check-ins and suggests matches as you type
- **Callsign lookup** - automatically queries [callook.info](https://callook.info) in the background and fills the operator name when a result is found
- **Operator profile** - set your callsign and name on first launch; shown in the header and included in exports
- **Export to text** - export any session log to a formatted `.txt` file with a custom filename
- **Resizable panels** - drag the nets/log split and session pane height with keyboard shortcuts
- **Base16 theming** - 8 built-in themes switchable at runtime; drop any Base16 TOML into `~/.config/netcontrol/themes/` for custom colours
- **Persistent storage** - all data saved automatically to `~/.netcontrol_data.json`

---

## Installation

### Pre-built binary (Linux x86-64)

Download `netcontrol` from the [Releases](../../releases) page, then:

```bash
chmod +x netcontrol
./netcontrol
```

Requires only standard system libraries (`libc`, `libm`, `libgcc_s`) present on any Linux distro.

### Build from source

Requires Rust 1.75 or later. Install via [rustup](https://rustup.rs/) if needed.

```bash
git clone https://github.com/youruser/netcontrol
cd netcontrol
cargo build --release
./target/release/netcontrol
```

---

## Usage

On first launch you will be prompted for your **operator callsign and name**. This is stored and shown in the header on all subsequent launches. Press `p` at any time to update it.

Press **`?`** at any time to open the full keybinding reference.

### Navigation model

The interface has three focus levels navigated with `Tab`, `Enter`, and `Esc`:

```
NETS  →(Enter)→  SESSIONS  →(Enter)→  LOG
      ←(Esc)←               ←(Esc)←
```

`Tab` cycles forward through all three. `Esc` steps back one level.

### Keybindings

| Key | Action |
|-----|--------|
| `Tab` | Cycle focus: Nets → Sessions → Log |
| `↑` / `↓` | Move selection |
| `Enter` | Open selected item |
| `Esc` | Go back one level |
| `n` | Add new net (Nets focus) / new session for today (Sessions or Log focus) |
| `e` | Edit selected net |
| `d` | Delete selected net, session, or check-in |
| `c` | Add a check-in to the active session |
| `x` | Export active session log to a text file |
| `p` | Edit operator profile |
| `t` | Open theme picker |
| `?` | Show keybinding help |
| `Ctrl+←` / `Ctrl+→` | Resize the left nets panel |
| `Ctrl+↑` / `Ctrl+↓` | Resize the sessions pane height |
| `q` | Quit (with confirmation) |

### Sessions

Each net can have multiple sessions - one per time the net runs. Select a net and press `Enter` to see its session list. From there:

- `n` creates a new session dated today with the current local time
- `Enter` on a session opens its check-in log
- `d` deletes a session (with confirmation)
- `Ctrl+↑` / `Ctrl+↓` adjusts how many rows the session list occupies when the log is also visible

**Existing data** from before sessions were introduced is automatically migrated: a net's flat check-in list becomes a single session using the net's saved date and time.

### Check-in autocomplete and lookup

While typing a callsign in the check-in dialog:

- A dropdown of up to 8 matching callsigns from previous check-ins appears automatically
- `↑` / `↓` cycles through suggestions; `Tab` or `Enter` applies the selected one and fills the Name field
- When you move off the callsign field, a background lookup is sent to [callook.info](https://callook.info); the title bar shows `[Searching…]` while it is pending
- When a result arrives the Name field is filled automatically

Callsign/name pairs are saved to `~/.netcontrol_data.json` and grow richer over time as you log more check-ins.

### Net dialog fields

| Field | Description |
|-------|-------------|
| Net Name | Short identifier, stored in uppercase |
| Club / Association | Optional club name shown in the net list and exports |
| Frequency (MHz) | Repeater or simplex frequency |
| Offset | Repeater offset e.g. `+0.600` or `SIMPLEX` |
| PL Tone (Hz) | CTCSS tone or `NONE` |
| Digital Net | Toggle with `Space`; expands to show mode picker and notes field |

Date and time are set per-session, not per-net. When you create a new session with `n`, it is automatically stamped with today's date and current local time.

### Export format

Exports are plain text files saved to `~/`. Example:

```
============================================================
  NET CONTROL LOG EXPORT
============================================================
  Net Control : W1AW (Hiram Percy Maxim)
  Net    : WEEKLY 2M NET
  Club   : ARRL
  Freq   : 146.520 MHz   Offset: +0.600   PL: 100.0
  Date   : 2026-04-12   Time: 19:00
  Type   : Voice
  Total  : 3 check-ins
============================================================
  #  TIME    CALLSIGN     NAME                   REMARKS
----------------------------------------------------------------------
  1  19:02z  W1AW         Hiram                  NCS
  2  19:03z  KD9XYZ       Jane                   /QRP
  3  19:04z  N0CALL       Bob
----------------------------------------------------------------------
Exported: 2026-04-12 19:05:00 UTC
```

---

## Theming

NET CONTROL uses the [Base16](https://github.com/chriskempson/base16) colour framework. Every colour in the UI maps to one of 16 named slots, making it straightforward to create or adapt any existing Base16 theme.

### Built-in themes

Press `t` to open the theme picker. The selected theme is saved and restored on next launch.

| Theme | Style |
|-------|-------|
| Catppuccin Mocha | Soft pastel purples (default) |
| Gruvbox Dark | Warm retro browns and oranges |
| Nord | Cool arctic blues |
| Solarized Dark | Classic teal and gold |
| Dracula | High-contrast purple and pink |
| One Dark | Atom editor grey and blue |
| Tokyo Night | Deep navy with vivid accents |
| Mellow | Desaturated warm neutrals |

### Custom themes

Drop any Base16 TOML file into `~/.config/netcontrol/themes/`. It will appear in the theme picker automatically on next launch, sorted alphabetically after the built-ins.

**Minimal template** (`~/.config/netcontrol/themes/mytheme.toml`):

```toml
scheme = "My Theme"

# Backgrounds - darkest to lightest
base00 = "1a1b26"   # main background
base01 = "16172e"   # unused (kept for compatibility)
base02 = "2a2b3d"   # selection background
base03 = "565f89"   # dim text / hints

# Foregrounds
base04 = "a9b1d6"   # normal borders
base05 = "c0caf5"   # default foreground / body text
base06 = "cbdbf8"   # unused
base07 = "e5e9fc"   # unused

# Accent colours
base08 = "f7768e"   # red   - danger, delete confirmations
base09 = "ff9e64"   # amber - frequencies, accents
base0a = "e0af68"   # yellow - UTC timestamps
base0b = "9ece6a"   # green  - net names, ok states
base0c = "73daca"   # cyan   - field labels, callsigns
base0d = "7aa2f7"   # blue   - info values
base0e = "bb9af7"   # mauve  - bold text, headings, focused borders
base0f = "b45bcf"   # pink   - club names, operator profile dialog
```

Values can be bare hex (`1a1b26`) or prefixed (`#1a1b26`). The `scheme` key sets the name shown in the picker; if omitted the filename is used.

See the [base16-gallery](https://tinted-theming.github.io/base16-gallery/) for hundreds of community themes. Most ship as YAML - convert by replacing `:` with `=` and removing the `---` header.

---

## Data file

All nets, sessions, check-ins, known callsigns, and the active theme name are stored in `~/.netcontrol_data.json`. This file is human-readable, can be backed up or version-controlled, and is forward-compatible - new fields use `#[serde(default)]` so existing data always loads cleanly.

---

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `ratatui` | 0.26 | Terminal UI layout and widgets |
| `crossterm` | 0.27 | Cross-platform terminal input and raw mode |
| `serde` + `serde_json` | 1.x | JSON serialisation |
| `chrono` | 0.4 | UTC clock, timestamps, date defaults |
| `minreq` | 2.11 | HTTPS callsign lookup via callook.info |

---

## License

MIT - see [LICENSE](LICENSE).

---

73
