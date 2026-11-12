# aulactl

A lightweight, open-source replacement for the official **AULA F75** keyboard software (`OemDrv.exe`).
Built with Tauri 2 (Rust + `hidapi`) and Nuxt UI. Works over USB cable and the 2.4 GHz receiver.

> **Unofficial.** Not affiliated with or endorsed by AULA / Epomaker. "AULA" and "F75" are trademarks of their owners.
> The protocol was reverse engineered from the official driver for interoperability ([docs/PROTOCOL.md](docs/PROTOCOL.md)).
> Use at your own risk: every write is preceded by an automatic backup, but this is still a third-party tool writing to your keyboard's flash.

## Status

The author no longer uses an F75, so the project is **unmaintained** — issues may go unanswered, but pull requests and forks are welcome.

## Features

- Lighting effects, brightness, speed, sleep timer, debounce
- Key remapping (base + FN layer), per-key RGB, macros
- Profiles, backup / restore (`%APPDATA%\aulactl\backups`)
- Battery level + low-battery notifications (the official app does not show this)
- Caps Lock indicator, notification lights, music-reactive lighting (Windows, WASAPI loopback)
- Firmware version check against the vendor site (it only opens the official updater; it never flashes anything itself)
- Tray icon, start on login

Hardware: wired `258A:010C`, 2.4G receiver `3554:FA09`. Tested on firmware `0x1100`, mostly over the receiver.
The UI is in English and Turkish (Settings → Language). To add a language, follow `app/utils/i18n.ts` and `app/locales/en.ts`; `pnpm test` flags missing strings.

Not tested: palette writes, the "Off" lighting mode, writing over the cable from the app, Linux / macOS builds on real hardware.

## Install

Grab a build from [Releases](../../releases):

- **Windows:** `aulactl_<version>_x64-setup.exe` (no admin rights) or `.msi`.
- **Linux:** `.deb` / `.rpm` install the udev rule (`70-aula-f75.rules`); replug the keyboard afterwards. For the AppImage add it manually:

  ```bash
  sudo cp src-tauri/linux/70-aula-f75.rules /usr/lib/udev/rules.d/ && sudo udevadm control --reload && sudo udevadm trigger
  ```
- **macOS:** `.dmg` (untested).

Close the official software first: two clients on the same HID interface mix up replies.

## Build

```bash
pnpm install
pnpm tauri dev          # run
pnpm tauri build        # installers for the current OS
pnpm test               # keycode tests
```

Device test CLI (not bundled): `cargo run --manifest-path src-tauri/Cargo.toml --features cli --bin aulactl-cli -- read`

Cross-compiling for Windows from Linux/WSL (needs llvm-mingw and `makensis` on `PATH`):

```bash
pnpm tauri build --target x86_64-pc-windows-gnullvm --bundles nsis --config src-tauri/tauri.gnullvm.conf.json
```

Releases: bump the version in `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml`, push a `v*` tag; CI builds Windows, Linux and macOS and creates a draft release.

## Device quirks

See [docs/PROTOCOL.md](docs/PROTOCOL.md) §5b. Highlights:

- The receiver uses output report `0x13`, not `0x15` as the official driver suggests.
- Writing a key layer without its trailing `00 00 5A A5` signature makes the keyboard stop answering reads for that layer. Always write signed.
- The first packet of each 256-byte page takes ~2.4 s to ACK (flash erase).
- The keyboard sleeps after 10 minutes and ignores writes until a key is pressed.
- The firmware bootloader appears to allow only ~8 flashes. Do not reflash needlessly.

## Türkçe

AULA F75 için resmi `OemDrv.exe` yerine geçen hafif, açık kaynak ayar uygulaması. Kablolu ve 2.4G alıcıyla çalışır:
ışık efektleri, tuş atama, tuş başına renk, makrolar, uyku/debounce, profiller, pil bildirimi, Caps Lock ve müzik ışıkları, firmware denetimi.
Resmi değildir, AULA ile bağlantısı yoktur. Bakımı yapılmıyor; PR ve fork'lar memnuniyetle. Protokol notları: [docs/PROTOCOL.md](docs/PROTOCOL.md).

## License

[MIT](LICENSE)
