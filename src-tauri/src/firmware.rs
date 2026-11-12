//! Firmware güncelleme denetimi: AULA'nın sitesindeki resmi güncelleyiciyi indirir, içindeki imajı çözüp sürümünü okur,
//! klavyedekinden yeniyse bildirir. Yazmayı resmi araç yapar (kabloyla).
//! Önyükleyici yazma sayısını sayıyor (~8 hak, re-tools/aula-f75/PROTOCOL.md §7) → yalnız gerçekten yeni sürümde başlatılır.

use crate::config_dir;
use serde::Serialize;
use std::path::PathBuf;
use std::time::Duration;
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

const SITE: &str = "https://www.aulastar.com";
const KB: (u16, u16) = (0x258A, 0x010C);

#[derive(Serialize, Clone)]
pub struct FwInfo {
  /// klavyenin sürümü (bcdDevice); kabloyla en az bir kez bağlanmadıysa bilinmez
  current: Option<u16>,
  latest: u16,
  newer: bool,
}

fn tea_dec(mut v0: u32, mut v1: u32, k: [u32; 4]) -> (u32, u32) {
  // standart TEA'dan farklı delta (Update Tool FUN_004038e0)
  let mut s: u32 = 0xC6ED3720;
  for _ in 0..32 {
    v1 = v1.wrapping_sub((v0 << 4).wrapping_add(k[2]) ^ v0.wrapping_add(s) ^ (v0 >> 5).wrapping_add(k[3]));
    v0 = v0.wrapping_sub((v1 << 4).wrapping_add(k[0]) ^ v1.wrapping_add(s) ^ (v1 >> 5).wrapping_add(k[1]));
    s = s.wrapping_sub(0x9E3769B9);
  }
  (v0, v1)
}

/// Güncelleyici exe'sinden çözülmüş imaj: 'Z' A5 "SINO", BE vid, pid, boy, şifreli imaj, 4 B anahtar.
pub fn image(exe: &[u8]) -> Option<((u16, u16), Vec<u8>)> {
  let i = exe.windows(6).position(|w| w == b"Z\xa5SINO")?;
  let be = |o: usize| u16::from_be_bytes([exe[i + o], exe[i + o + 1]]);
  let (vid, pid, n) = (be(6), be(8), be(10) as usize);
  let data = exe.get(i + 12..i + 12 + n)?;
  let k = exe.get(i + 12 + n..i + 16 + n)?;
  let k = [k[0], k[1], k[2], k[3]].map(u32::from);
  let mut out = data.to_vec();
  for c in out.chunks_exact_mut(8) {
    let w = |o: usize| u32::from_be_bytes([c[o], c[o + 1], c[o + 2], c[o + 3]]);
    let (a, b) = tea_dec(w(0), w(4), k);
    c[..4].copy_from_slice(&a.to_be_bytes());
    c[4..].copy_from_slice(&b.to_be_bytes());
  }
  Some(((vid, pid), out))
}

/// İmajdaki USB cihaz tanımlayıcısının bcdDevice'ı (12 01 … vid pid bcd).
pub fn version(img: &[u8], (vid, pid): (u16, u16)) -> Option<u16> {
  let ids = [vid.to_le_bytes(), pid.to_le_bytes()].concat();
  img.windows(14).find(|w| w[0] == 0x12 && w[1] == 0x01 && w[8..12] == ids[..]).map(|w| u16::from_le_bytes([w[12], w[13]]))
}

fn curl(url: &str, out: Option<&PathBuf>) -> Result<Vec<u8>, String> {
  // ponytail: sistem curl'ü (Windows 10+ ve Linux/macOS'ta hazır); HTTP crate'i gerekirse ureq
  let mut c = std::process::Command::new("curl");
  c.args(["-sSfL", "--max-time", "120", url]);
  if let Some(p) = out {
    c.arg("-o").arg(p);
  }
  #[cfg(windows)]
  std::os::windows::process::CommandExt::creation_flags(&mut c, 0x0800_0000); // CREATE_NO_WINDOW
  let o = c.output().map_err(|e| l!("could not run curl: {e}", "curl çalıştırılamadı: {e}"))?;
  if !o.status.success() {
    return Err(l!("download failed: {}", "indirme başarısız: {}", String::from_utf8_lossy(&o.stderr).trim()));
  }
  Ok(o.stdout)
}

/// Sürücü sayfasında F75 kartının "Firmware" bağlantısı.
fn link(html: &str) -> Option<String> {
  let card = &html[html.find(">F75</h3>")?..];
  let card = &card[..card.find("</div>")?];
  let a = card.find(">Firmware<")?;
  let href = &card[..a];
  let start = href.rfind("href=\"")? + 6;
  let path = &href[start..start + href[start..].find('"')?];
  Some(format!("{SITE}{}", path.replace("&amp;", "&")))
}

fn exe_path() -> PathBuf {
  let d = config_dir().join("firmware");
  let _ = std::fs::create_dir_all(&d);
  d.join("F75 firmware.exe")
}

/// Klavyenin sürümü: kablo takılıysa cihazdan, değilse en son kabloda görülen (alıcının sürümü klavyeninki değil).
fn current() -> Option<u16> {
  let saved = config_dir().join("kb_fw");
  let live = hidapi::HidApi::new().ok().and_then(|api| {
    api.device_list().find(|i| (i.vendor_id(), i.product_id()) == KB).map(|i| i.release_number())
  });
  if let Some(v) = live {
    let _ = std::fs::write(&saved, format!("{v:04X}"));
    return Some(v);
  }
  std::fs::read_to_string(saved).ok().and_then(|s| u16::from_str_radix(s.trim(), 16).ok())
}

#[tauri::command(async)]
pub fn firmware_check() -> Result<FwInfo, String> {
  let html = String::from_utf8_lossy(&curl(&format!("{SITE}/drive/"), None)?).into_owned();
  let url = link(&html).ok_or_else(|| l!("AULA does not publish F75 firmware on its driver page (as of 2026-11 only the driver is listed)", "AULA sürücü sayfasında F75 için firmware yayınlanmıyor (2026-11: kartta yalnız Driver var)"))?;
  let path = exe_path();
  curl(&url, Some(&path))?;
  let exe = std::fs::read(&path).map_err(|e| e.to_string())?;
  let (ids, img) = image(&exe).ok_or_else(|| l!("no firmware image in the downloaded file", "indirilen dosyada firmware imajı yok"))?;
  if ids != KB {
    return Err(l!("the downloaded firmware is for another device ({:04X}:{:04X})", "indirilen firmware başka bir cihaz için ({:04X}:{:04X})", ids.0, ids.1));
  }
  let latest = version(&img, ids).ok_or_else(|| l!("no version info in the image", "imajda sürüm bilgisi bulunamadı"))?;
  let current = current();
  Ok(FwInfo { current, latest, newer: current.is_some_and(|c| latest > c) })
}

/// Resmi güncelleyiciyi açar; yalnız sitedeki sürüm klavyedekinden yeniyse (yazma hakkı sınırlı).
#[tauri::command(async)]
pub fn firmware_update() -> Result<(), String> {
  let info = firmware_check()?;
  if !info.newer {
    return Err(l!("The keyboard is already on the latest firmware; flashing uses up limited write cycles, so the updater was not started", "Klavye zaten en son sürümde; güncelleme yazma hakkı harcar, başlatılmadı"));
  }
  #[cfg(windows)]
  {
    std::process::Command::new(exe_path()).spawn().map_err(|e| l!("could not open the updater: {e}", "güncelleyici açılamadı: {e}"))?;
    Ok(())
  }
  #[cfg(not(windows))]
  Err(l!("The official updater only runs on Windows", "Resmi güncelleyici yalnız Windows'ta çalışıyor"))
}

/// Açılışta ve günde bir: yeni sürüm varsa bildirim (aynı sürüm için bir kez).
pub fn watch(app: AppHandle) {
  std::thread::spawn(move || {
    std::thread::sleep(Duration::from_secs(60));
    loop {
      if let Ok(i) = firmware_check() {
        let seen = config_dir().join("fw_notified");
        if i.newer && std::fs::read_to_string(&seen).ok() != Some(format!("{:04X}", i.latest)) {
          let _ = app
            .notification()
            .builder()
            .title(l!("New firmware for the AULA F75", "AULA F75 için yeni firmware"))
            .body(l!("Version {}. Update it from aulactl → Device (over the cable).", "Sürüm {}. aulactl → Cihaz sayfasından güncelleyebilirsin (kabloyla).", fmt(i.latest)))
            .show();
          let _ = std::fs::write(seen, format!("{:04X}", i.latest));
        }
      }
      std::thread::sleep(Duration::from_secs(24 * 3600));
    }
  });
}

fn fmt(v: u16) -> String {
  format!("V{}", format!("{v:X}").chars().map(String::from).collect::<Vec<_>>().join("."))
}

#[cfg(test)]
mod tests {
  use super::*;

  fn tea_enc(mut v0: u32, mut v1: u32, k: [u32; 4]) -> (u32, u32) {
    let mut s: u32 = 0;
    for _ in 0..32 {
      s = s.wrapping_add(0x9E3769B9);
      v0 = v0.wrapping_add((v1 << 4).wrapping_add(k[0]) ^ v1.wrapping_add(s) ^ (v1 >> 5).wrapping_add(k[1]));
      v1 = v1.wrapping_add((v0 << 4).wrapping_add(k[2]) ^ v0.wrapping_add(s) ^ (v0 >> 5).wrapping_add(k[3]));
    }
    (v0, v1)
  }

  #[test]
  fn parse_updater() {
    // gerçek dosyanın baş bloğu: anahtar 85 2D A0 F5 ile çözülünce 8051 LJMP (02 9C 74 …)
    let k = [0x85, 0x2D, 0xA0, 0xF5];
    assert_eq!(tea_dec(0x46d0c1eb, 0x273b401c, k.map(u32::from)).0 >> 8, 0x029C74);

    let mut img = vec![0u8; 64];
    img[16..30].copy_from_slice(&[0x12, 1, 0, 2, 0, 0, 0, 8, 0x8a, 0x25, 0x0c, 0x01, 0x00, 0x11]);
    let mut enc = img.clone();
    for c in enc.chunks_exact_mut(8) {
      let w = |o: usize| u32::from_be_bytes([c[o], c[o + 1], c[o + 2], c[o + 3]]);
      let (a, b) = tea_enc(w(0), w(4), k.map(u32::from));
      c[..4].copy_from_slice(&a.to_be_bytes());
      c[4..].copy_from_slice(&b.to_be_bytes());
    }
    let mut exe = b"MZ....Z\xa5SINO\x25\x8a\x01\x0c\x00\x40".to_vec();
    exe.extend(enc);
    exe.extend(k);
    let (ids, out) = image(&exe).unwrap();
    assert_eq!((ids, &out), (KB, &img));
    assert_eq!(version(&out, ids), Some(0x1100));

    let html = r#"<h3 class="cover_tit">F75HE</h3><div><a href="/x">Firmware</a></div><h3 class="cover_tit">F75</h3>
      <div class="n-ab"><a class="d" href="/index.php?a=1&amp;aid=548">Driver</a><a class="d" href="/index.php?a=2&amp;aid=548">Firmware</a></div>"#;
    assert_eq!(link(html).as_deref(), Some("https://www.aulastar.com/index.php?a=2&aid=548"));
  }
}
