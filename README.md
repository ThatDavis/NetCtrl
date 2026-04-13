# NET CONTROL — HAM Radio Net Check-in Logger

```
 ███╗   ██╗███████╗████████╗     ██████╗████████╗██████╗ ██╗
 ████╗  ██║██╔════╝╚══██╔══╝    ██╔════╝╚══██╔══╝██╔══██╗██║
 ██╔██╗ ██║█████╗     ██║       ██║        ██║   ██████╔╝██║
 ██║╚██╗██║██╔══╝     ██║       ██║        ██║   ██╔══██╗██║
 ██║ ╚████║███████╗   ██║       ╚██████╗   ██║   ██║  ██║███████╗
 ╚═╝  ╚═══╝╚══════╝   ╚═╝        ╚═════╝   ╚═╝   ╚═╝  ╚═╝╚══════╝
```

A terminal TUI for logging amateur radio net check-ins. Built in Rust with [Ratatui](https://ratatui.rs/) and the [Catppuccin Mocha](https://catppuccin.com/) colour theme.

---

## Features

- **Multiple saved nets** — store frequency, offset, PL tone, date, time, and club/association per net
- **Digital net support** — flag a net as digital and select from 20 common modes (FT8, FT4, JS8Call, Winlink, DMR, D-STAR, and more), with a free-text notes field for dial frequency or gateway info
- **Check-in logging** — log callsign, operator name, and remarks with automatic UTC timestamps
- **Operator profile** — set your callsign and name on first launch; shown in the header and included in exports
- **Export to text** — export any net log to a formatted `.txt` file with a custom filename, saved to your home directory
- **Resizable panels** — drag the nets/log split with `Ctrl+←` / `Ctrl+→`
- **Persistent storage** — all data saved automatically to `~/.netcontrol_data.json`

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

### Keybindings

| Key | Action |
|-----|--------|
| `Tab` | Switch focus between Nets panel and Log panel |
| `↑` / `↓` | Navigate lists |
| `Enter` | Select net / confirm dialog |
| `n` | Add a new net |
| `e` | Edit the selected net |
| `d` | Delete selected net or check-in |
| `c` | Add a check-in to the active net |
| `x` | Export the active net log to a text file |
| `p` | Edit operator profile |
| `Ctrl+←` | Shrink the left panel |
| `Ctrl+→` | Grow the left panel |
| `q` / `Esc` | Quit / close dialog |

### Net dialog fields

| Field | Description |
|-------|-------------|
| Net Name | Short identifier, stored in uppercase |
| Club / Association | Optional club name shown in the net list and exports |
| Frequency (MHz) | Repeater or simplex frequency |
| Offset | Repeater offset e.g. `+0.600` or `SIMPLEX` |
| PL Tone (Hz) | CTCSS tone or `NONE` |
| Date | `YYYY-MM-DD` format |
| Time | Local time `HH:MM` |
| Digital Net | Toggle with `Space`; expands to show mode picker and notes field |

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

## Data file

All nets and check-ins are stored in `~/.netcontrol_data.json`. This file is human-readable and can be backed up or version-controlled. The schema is forward-compatible — new fields added in future versions use `#[serde(default)]` so existing data files will always load cleanly.

---

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `ratatui` | 0.26 | Terminal UI layout and widgets |
| `crossterm` | 0.27 | Cross-platform terminal input and raw mode |
| `serde` + `serde_json` | 1.x | JSON serialisation of nets and check-ins |
| `chrono` | 0.4 | UTC clock, timestamps, date defaults |

---

## License

MIT — see [LICENSE](LICENSE).

---

73 
