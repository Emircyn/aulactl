/// Kullanıcıya görünen metin: l!("English {x}", "Türkçe {x}") → prefs.lang'e göre biçimlenmiş String.
macro_rules! l {
  ($en:literal, $tr:literal $(, $a:expr)* $(,)?) => {
    if crate::extras::is_tr() { format!($tr $(, $a)*) } else { format!($en $(, $a)*) }
  };
}

pub mod extras;
pub mod f75;
pub mod firmware;
pub mod music;

use f75::{Device, Event, Kind, LED_LEN, MATRIX_LEN, Op, PALETTE_LEN, PLANE};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};

// ---------- app/types/device.ts aynası ----------
pub(crate) type Rgb = [u8; 3];

#[derive(Serialize)]
pub struct DeviceStatus {
  connected: bool,
  transport: Option<Kind>,
  online: bool,
  fw: Option<String>,
  battery: Option<u8>,
  charging: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct LedMode {
  brightness: u8,
  speed: u8,
  color: u8,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Led {
  mode: u8,
  custom: bool,
  sleep: u32,
  debounce: u8,
  modes: Vec<LedMode>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MacroStep {
  pub action: String, // "down" | "up"
  pub key: u8,
  pub delay: u32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Macro {
  pub name: String,
  pub repeat: u8,
  pub count: u8,
  pub steps: Vec<MacroStep>,
}

#[derive(Serialize)]
pub struct DeviceConfig {
  layers: Vec<Vec<String>>,
  led: Led,
  palette: Vec<Vec<Rgb>>,
  perkey: Vec<Vec<Rgb>>,
  macros: Vec<Macro>,
}

#[derive(Serialize, Clone)]
struct DeviceEvent {
  kind: &'static str,
  a: u8,
  b: u8,
}

// ---------- LED bloğu (128 B) ----------
// 0x09 custom, 0x0A etkin efekt hw kimliği, 0x18 uyku (x30 sn), 0x3A+2i parlaklık, 0x3B+2i hız<<4|renk, 0x7E 5A A5
const EFFECTS: usize = f75::EFFECTS;

pub fn led_from(b: &[u8]) -> Led {
  Led {
    mode: b[0x0A],
    custom: b[0x09] != 0,
    sleep: b[0x18] as u32 * 30,
    debounce: b[0x03],
    modes: (0..EFFECTS)
      .map(|i| LedMode { brightness: b[0x3A + 2 * i], speed: b[0x3B + 2 * i] >> 4, color: b[0x3B + 2 * i] & 0xF })
      .collect(),
  }
}

pub fn led_into(led: &Led, b: &mut [u8]) {
  b[0x0A] = led.mode;
  b[0x09] = led.custom as u8;
  b[0x18] = (led.sleep / 30).min(255) as u8;
  if led.debounce <= 4 {
    b[0x03] = led.debounce; // debounce 0-4 (resmi arayüzde +1 "Stage")
  }
  for (i, m) in led.modes.iter().take(EFFECTS).enumerate() {
    b[0x3A + 2 * i] = m.brightness.min(9);
    b[0x3B + 2 * i] = (m.speed.min(4) << 4) | m.color.min(7);
  }
}

fn palette_from(b: &[u8]) -> Vec<Vec<Rgb>> {
  b[21..].chunks(21).map(|row| row.chunks(3).map(|c| [c[0], c[1], c[2]]).collect()).collect()
}

fn palette_bytes(p: &[Vec<Rgb>]) -> Vec<u8> {
  let mut out = vec![0u8; 21];
  for row in p.iter().take(EFFECTS) {
    for i in 0..7 {
      out.extend_from_slice(&row.get(i).copied().unwrap_or([0; 3]));
    }
  }
  out.resize(PALETTE_LEN, 0);
  out
}

fn perkey_from(b: &[u8]) -> Vec<Rgb> {
  (0..PLANE).map(|i| [b[i], b[PLANE + i], b[2 * PLANE + i]]).collect()
}

fn perkey_bytes(c: &[Rgb]) -> Vec<u8> {
  let mut out = vec![0u8; MATRIX_LEN * 3]; // OemDrv 3x126 düzlem + 6 B dolgu gönderiyor (MatrixLen*3)
  for (i, rgb) in c.iter().take(PLANE).enumerate() {
    (out[i], out[PLANE + i], out[2 * PLANE + i]) = (rgb[0], rgb[1], rgb[2]);
  }
  out
}

fn codes_from(b: &[u8]) -> Vec<String> {
  b.chunks(4).map(|c| format!("{:02X}{:02X}{:02X}{:02X}", c[0], c[1], c[2], c[3])).collect()
}

fn codes_bytes(codes: &[String]) -> Result<Vec<u8>, String> {
  let mut out = vec![0u8; MATRIX_LEN * 4];
  for (i, c) in codes.iter().take(MATRIX_LEN).enumerate() {
    let v = u32::from_str_radix(c, 16).map_err(|_| l!("invalid key code: {c}", "geçersiz tuş kodu: {c}"))?;
    out[i * 4..i * 4 + 4].copy_from_slice(&v.to_be_bytes());
  }
  out[MATRIX_LEN * 4 - 4..].copy_from_slice(&[0, 0, 0x5A, 0xA5]); // katman imzası (cihazda doğrulandı)
  Ok(out)
}

/// Makro tamponu (StMacro_To_HdMacro @0x48cd70 + FillMatrix @0x48d140):
/// [adres u16 LE, boyut u16 LE] x n, sonra makro başına [isim bayt sayısı][isim UTF-16LE][olay x 4 B]
/// olay = [tip<<4 | gecikme>>16 & 0xF | 0x80 bırakma][gecikme>>8][gecikme][HID]; tip 0 = tuş, 1 = modifier (E0-E7)
pub fn macro_buffer(macros: &[Macro]) -> Vec<u8> {
  let blobs: Vec<Vec<u8>> = macros
    .iter()
    .map(|m| {
      let name: Vec<u8> = m.name.encode_utf16().take(30).flat_map(u16::to_le_bytes).collect();
      let mut b = vec![name.len() as u8];
      b.extend(name);
      for s in &m.steps {
        let d = s.delay.min(0xFFFFF);
        let up = if s.action == "up" { 0x80 } else { 0 };
        let typ = if (0xE0..=0xE7).contains(&s.key) { 0x10 } else { 0 }; // cihazda: tip 1 verilen "a" hiç yazılmadı
        b.extend([typ | up | ((d >> 16) as u8 & 0xF), (d >> 8) as u8, d as u8, s.key]);
      }
      b
    })
    .collect();
  let mut addr = blobs.len() * 4;
  let mut out = Vec::new();
  for b in &blobs {
    out.extend((addr as u16).to_le_bytes());
    out.extend((b.len() as u16).to_le_bytes());
    addr += b.len();
  }
  blobs.into_iter().for_each(|b| out.extend(b));
  out
}

// ---------- durum ----------
pub struct App {
  dev: Mutex<Option<Device>>,
  prefs: Mutex<extras::Prefs>,
  /// bildirim ışıkları için: klavye tuş başına modda mı + kayıtlı tuş renkleri
  look: Mutex<Look>,
}

#[derive(Clone, Default)]
pub struct Look {
  pub custom: bool,
  pub perkey: Vec<Rgb>,
}

fn look_from_value(v: &serde_json::Value) -> Look {
  let led = v["led"].as_str().map(unhex).unwrap_or_default();
  let pk = v["perkey"].as_str().map(unhex).unwrap_or_default();
  Look { custom: led.get(9).is_some_and(|&b| b != 0), perkey: if pk.len() >= PLANE * 3 { perkey_from(&pk) } else { vec![] } }
}

pub(crate) fn config_dir() -> PathBuf {
  let base = std::env::var_os("APPDATA")
    .map(PathBuf::from)
    .or_else(|| std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from))
    .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config"));
  let d = base.join("aulactl");
  let _ = std::fs::create_dir_all(d.join("backups"));
  d
}

fn sink_for(app: &AppHandle) -> f75::Sink {
  let app = app.clone();
  Arc::new(move |e: Event| {
    let ev = match e {
      Event::Battery { level, charging } => {
        extras::on_battery(&app, level, charging);
        DeviceEvent { kind: "battery", a: level, b: if charging { 0x10 } else { 0 } }
      }
      Event::Online { online } => DeviceEvent { kind: "link", a: online as u8, b: 0 },
      Event::Other { code, a, b } => DeviceEvent { kind: "other", a: code, b: a ^ b },
    };
    let _ = app.emit("device-event", ev);
  })
}

/// Önbellekteki cihazla çalışır; hata olursa bağlantıyı bırakır (sonraki çağrı yeniden açar).
pub(crate) fn with_dev<T>(app: &AppHandle, st: &App, f: impl FnOnce(&Device) -> Result<T, String>) -> Result<T, String> {
  let mut g = st.dev.lock().unwrap();
  if g.is_none() {
    *g = Some(Device::open(sink_for(app))?);
  }
  let r = f(g.as_ref().unwrap());
  if r.is_err() {
    *g = None;
  }
  r
}

pub fn read_raw(d: &Device) -> Result<serde_json::Value, String> {
  let hex = |b: Vec<u8>| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
  let mut layers = Vec::new();
  for l in 0..d.layers() {
    layers.push(hex(d.read(Op::Matrix, l, MATRIX_LEN * 4)?));
  }
  Ok(serde_json::json!({
    "transport": d.kind,
    "layers": layers,
    "led": hex(d.read(Op::Led, 0, LED_LEN)?),
    // alıcı paleti okumaya yanıt vermiyor; yedekte yok sayılır
    "palette": if d.kind == Kind::Wired { hex(d.read(Op::Palette, 0, PALETTE_LEN)?) } else { String::new() },
    "perkey": hex(d.read(Op::PerKey, 0, MATRIX_LEN * 3)?),
  }))
}

fn unhex(s: &str) -> Vec<u8> {
  (0..s.len() / 2).filter_map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()).collect()
}

/// Her yazmadan önce cihazın tam görüntüsünü saklar.
fn auto_backup(d: &Device) -> Result<PathBuf, String> {
  let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
  let path = config_dir().join("backups").join(format!("{ts}.json"));
  std::fs::write(&path, serde_json::to_vec_pretty(&read_raw(d)?).unwrap()).map_err(|e| e.to_string())?;
  Ok(path)
}

fn macros_path() -> PathBuf {
  config_dir().join("macros.json")
}

// ---------- komutlar ----------
#[tauri::command(async)]
fn device_status(app: AppHandle, st: State<App>) -> DeviceStatus {
  with_dev(&app, &st, |d| {
    // klavye pil olayını her zaman göndermiyor → her durum yenilemesinde sorgula (düşük pil uyarısı da buradan).
    // Sorgu kilit dışında: okuyucu iş parçacığı pil olayında aynı kilidi alıyor, yanıtı o taşıyor.
    let bat = d.battery().or(*d.last_battery.lock().unwrap());
    if let Some((level, charging)) = bat {
      *d.last_battery.lock().unwrap() = bat;
      extras::on_battery(&app, level, charging);
    }
    Ok(DeviceStatus {
      connected: true,
      transport: Some(d.kind),
      online: d.online(),
      // resmi uygulama gibi: 0x102 → V1.0.2
      fw: Some(format!("V{}", format!("{:X}", d.fw).chars().map(String::from).collect::<Vec<_>>().join("."))),
      battery: bat.map(|b| b.0),
      charging: if d.kind == Kind::Wired { Some(true) } else { bat.map(|b| b.1) },
    })
  })
  .unwrap_or_else(|_| {
    let bt = hidapi::HidApi::new().is_ok_and(|api| f75::bluetooth_present(&api));
    DeviceStatus { connected: bt, transport: bt.then_some(Kind::Bluetooth), online: bt, fw: None, battery: None, charging: None }
  })
}

#[tauri::command(async)]
fn read_config(app: AppHandle, st: State<App>) -> Result<DeviceConfig, String> {
  with_dev(&app, &st, |d| {
    let mut layers = Vec::new();
    for l in 0..d.layers() {
      layers.push(codes_from(&d.read(Op::Matrix, l, MATRIX_LEN * 4)?));
    }
    // alıcı paleti okumaya yanıt vermiyor → varsayılan palet (kırmızı, yeşil, mavi, sarı, mor, camgöbeği, beyaz)
    let default_pal = || {
      let row: Vec<Rgb> = vec![[255, 0, 0], [0, 255, 0], [0, 0, 255], [255, 255, 0], [255, 0, 255], [0, 255, 255], [255, 255, 255]];
      vec![row; EFFECTS]
    };
    let led = led_from(&d.read(Op::Led, 0, LED_LEN)?);
    let perkey = perkey_from(&d.read(Op::PerKey, 0, MATRIX_LEN * 3)?);
    *st.look.lock().unwrap() = Look { custom: led.custom, perkey: perkey.clone() };
    Ok(DeviceConfig {
      layers,
      led,
      palette: if d.kind == Kind::Wired { palette_from(&d.read(Op::Palette, 0, PALETTE_LEN)?) } else { default_pal() },
      perkey: vec![perkey],
      macros: std::fs::read(macros_path()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default(),
    })
  })
}

#[tauri::command(async)]
fn write_keymap(app: AppHandle, st: State<App>, layer: u8, codes: Vec<String>) -> Result<(), String> {
  let data = codes_bytes(&codes)?;
  with_dev(&app, &st, |d| {
    auto_backup(d)?;
    if layer >= d.layers() {
      return Err(l!("layer {layer} is not available on this connection", "katman {layer} bu bağlantıda yok"));
    }
    d.write_verified(Op::Matrix, layer, &data)
  })
}

#[tauri::command(async)]
fn write_led(app: AppHandle, st: State<App>, led: Led) -> Result<(), String> {
  with_dev(&app, &st, |d| {
    auto_backup(d)?;
    let mut b = d.read(Op::Led, 0, LED_LEN)?; // resmi yazılım gibi oku-değiştir-yaz: bilinmeyen alanlar korunur
    if b[LED_LEN - 2..] != [0x5A, 0xA5] {
      return Err(l!("could not read the LED block signature, write cancelled", "LED bloğu imzası okunamadı, yazma iptal"));
    }
    led_into(&led, &mut b);
    d.write_verified(Op::Led, 0, &b)?;
    st.look.lock().unwrap().custom = led.custom;
    Ok(())
  })
}

#[tauri::command(async)]
fn write_palette(app: AppHandle, st: State<App>, palette: Vec<Vec<Rgb>>) -> Result<(), String> {
  let data = palette_bytes(&palette);
  with_dev(&app, &st, |d| {
    auto_backup(d)?;
    d.write_verified(Op::Palette, 0, &data)
  })
}

#[tauri::command(async)]
fn write_perkey(app: AppHandle, st: State<App>, group: u8, colors: Vec<Rgb>) -> Result<(), String> {
  let data = perkey_bytes(&colors);
  with_dev(&app, &st, |d| {
    auto_backup(d)?;
    d.write(Op::PerKey, group, &data)?; // geri okumada 6 B dolgunun ne döneceği bilinmiyor; doğrulama yok
    st.look.lock().unwrap().perkey = colors.clone();
    Ok(())
  })
}

#[tauri::command(async)]
fn write_macros(app: AppHandle, st: State<App>, macros: Vec<Macro>) -> Result<(), String> {
  let data = macro_buffer(&macros);
  std::fs::write(macros_path(), serde_json::to_vec_pretty(&macros).unwrap()).map_err(|e| e.to_string())?;
  if data.is_empty() {
    return Ok(());
  }
  with_dev(&app, &st, |d| {
    auto_backup(d)?;
    d.write(Op::Macro, 0, &data)
  })
}

#[tauri::command(async)]
fn backup(app: AppHandle, st: State<App>) -> Result<String, String> {
  with_dev(&app, &st, |d| auto_backup(d).map(|p| p.display().to_string()))
}

/// Yedek/profil görüntüsünü klavyeye yazar (önce otomatik yedek).
pub(crate) fn restore_value(d: &Device, v: &serde_json::Value) -> Result<(), String> {
  let field = |k: &str| v[k].as_str().map(unhex).ok_or(format!("yedekte {k} yok"));
  auto_backup(d)?;
  for (l, s) in v["layers"].as_array().ok_or("yedekte katman yok")?.iter().enumerate().take(d.layers() as usize) {
    d.write_verified(Op::Matrix, l as u8, &unhex(s.as_str().unwrap_or_default()))?;
  }
  d.write_verified(Op::Led, 0, &field("led")?)?;
  let pal = field("palette")?;
  if !pal.is_empty() && d.kind == Kind::Wired {
    d.write_verified(Op::Palette, 0, &pal)?;
  }
  d.write(Op::PerKey, 0, &field("perkey")?)
}

#[tauri::command(async)]
fn restore(app: AppHandle, st: State<App>, path: String) -> Result<(), String> {
  let v: serde_json::Value =
    serde_json::from_slice(&std::fs::read(&path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
  with_dev(&app, &st, |d| restore_value(d, &v))?;
  *st.look.lock().unwrap() = look_from_value(&v);
  Ok(())
}

#[tauri::command(async)]
fn factory_reset(app: AppHandle, st: State<App>) -> Result<(), String> {
  with_dev(&app, &st, |d| {
    if d.kind != Kind::Wired {
      return Err(l!("Factory reset only works over the cable (the official software does not offer it via the receiver either)", "Fabrika ayarı yalnız kablolu bağlantıda (resmi yazılımda da alıcı üzerinden yok)"));
    }
    auto_backup(d)?;
    d.write_reset()
  })
}

/// Kablolu / alıcı / Bluetooth'tan biri takılıp çıkarılınca "device-changed" yayar ve bağlantıyı yeniden kurdurur
/// (alıcı takılıyken kablo eklenince kabloluya geçmesi için türler ayrı izlenir).
fn watch(app: AppHandle) {
  std::thread::spawn(move || {
    let mut last = None;
    loop {
      let now = hidapi::HidApi::new()
        .map(|api| {
          let has = |id| api.device_list().any(|i| (i.vendor_id(), i.product_id()) == id);
          (has((0x258A, 0x010C)), has((0x3554, 0xFA09)), f75::bluetooth_present(&api))
        })
        .unwrap_or_default();
      if last != Some(now) {
        if last.is_some() {
          *app.state::<App>().dev.lock().unwrap() = None;
          let _ = app.emit("device-changed", now.0 || now.1 || now.2);
        }
        last = Some(now);
      }
      std::thread::sleep(std::time::Duration::from_secs(2));
    }
  });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_notification::init())
    .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--hidden"])))
    .manage(App { dev: Mutex::new(None), prefs: Mutex::new(extras::load_prefs()), look: Default::default() })
    .setup(|app| {
      watch(app.handle().clone());
      extras::tray(app.handle())?;
      extras::notifier(app.handle().clone());
      music::run(app.handle().clone());
      firmware::watch(app.handle().clone());
      extras::autostart_init(app.handle());
      // pencere tauri.conf'ta gizli açılıyor; Windows açılışında (--hidden) tepside kalır
      if !std::env::args().any(|a| a == "--hidden") {
        if let Some(w) = app.get_webview_window("main") {
          let _ = w.show();
        }
      }
      Ok(())
    })
    // pencereyi kapatmak tepsiye küçültür; çıkış tepsi menüsünden (pil uyarıları ve Caps ışığı arkada çalışsın)
    .on_window_event(|w, e| {
      if let tauri::WindowEvent::CloseRequested { api, .. } = e {
        api.prevent_close();
        let _ = w.hide();
      }
    })
    .invoke_handler(tauri::generate_handler![
      device_status,
      read_config,
      write_keymap,
      write_led,
      write_palette,
      write_perkey,
      write_macros,
      backup,
      restore,
      factory_reset,
      extras::get_prefs,
      extras::set_prefs,
      extras::profile_list,
      extras::profile_save,
      extras::profile_apply,
      extras::profile_delete,
      firmware::firmware_check,
      firmware::firmware_update,
      extras::autostart_get,
      extras::autostart_set
    ])
    .run(tauri::generate_context!())
    .expect("error while building tauri application");
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn conversions() {
    // cihazdan okunan LED bloğu mod tablosu: "09 47" = parlaklık 9, hız 4, renk 7 (rastgele)
    let mut b = vec![0u8; LED_LEN];
    b[0x0A] = 13;
    b[0x3A] = 9;
    b[0x3B] = 0x47;
    let led = led_from(&b);
    assert_eq!((led.mode, led.modes[0].brightness, led.modes[0].speed, led.modes[0].color), (13, 9, 4, 7));
    let mut b2 = b.clone();
    led_into(&led, &mut b2);
    assert_eq!(b, b2);

    let codes: Vec<String> = (0..MATRIX_LEN).map(|i| if i == 0 { "00000029".into() } else { "00000000".into() }).collect();
    let raw = codes_bytes(&codes).unwrap();
    assert_eq!(&raw[..4], &[0, 0, 0, 0x29]);
    assert_eq!(&raw[508..], &[0, 0, 0x5A, 0xA5]);
    assert_eq!(codes_from(&raw)[0], "00000029");

    let m = Macro { name: "a".into(), repeat: 1, count: 1, steps: vec![
      MacroStep { action: "down".into(), key: 0x04, delay: 50 },
      MacroStep { action: "up".into(), key: 0x04, delay: 0x12345 },
    ] };
    let buf = macro_buffer(&[m]);
    // başlık: adres 4, boyut 1+2+8=11
    assert_eq!(&buf[..4], &[4, 0, 11, 0]);
    assert_eq!(&buf[4..7], &[2, b'a', 0]);
    assert_eq!(&buf[7..11], &[0x00, 0, 50, 0x04]);
    assert_eq!(&buf[11..15], &[0x81, 0x23, 0x45, 0x04]);

    let pal = vec![vec![[255, 0, 0]; 7]; EFFECTS];
    let pb = palette_bytes(&pal);
    assert_eq!(pb.len(), PALETTE_LEN);
    assert_eq!(palette_from(&pb)[0][0], [255, 0, 0]);
  }
}
