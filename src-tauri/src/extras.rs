//! Resmi sürücüde olmayanlar: tepsi + pil bildirimi, profiller, bildirim ışıkları (Caps Lock, düşük pil).
//! Bildirim ışıkları anlık komutla (flaşa yazmaz) çalışır; klavye yalnız tuş başına (Self-define) moddayken gösterir
//! ve anlık kare gelirken listede olmayan tuşlar söner (ikisi de cihazda görüldü).

use crate::f75::{Device, Kind, PLANE};
use crate::{App, Rgb, config_dir, read_raw, restore_value, with_dev};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::time::Duration;
use tauri::menu::{Menu, MenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_notification::NotificationExt;

pub const CAPS_IDX: usize = 3; // layout.json: CapsLock matris/LED indeksi
const TRAY_ID: &str = "main";

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)] // eski prefs.json yeni alanlar olmadan da okunsun
pub struct Prefs {
  /// arayüz, tepsi ve bildirim dili: "en" | "tr"
  pub lang: String,
  pub caps_light: bool,
  pub caps_color: Rgb,
  pub battery_notify: bool,
  pub battery_flash: bool,
  pub music: bool,
  pub music_gain: f32,
  pub music_smooth: u8,
  pub music_color: Rgb,
  pub music_rainbow: bool,
}

impl Default for Prefs {
  fn default() -> Self {
    Prefs { lang: "en".into(), caps_light: false, caps_color: [255, 255, 255], battery_notify: true, battery_flash: true, music: false, music_gain: 1.0, music_smooth: 4, music_color: [0, 255, 0], music_rainbow: true }
  }
}

fn prefs_path() -> std::path::PathBuf {
  config_dir().join("prefs.json")
}

static TR: AtomicBool = AtomicBool::new(false);

/// l! makrosu için: Türkçe mi?
pub fn is_tr() -> bool {
  TR.load(Ordering::Relaxed)
}

pub fn load_prefs() -> Prefs {
  let p: Prefs = std::fs::read(prefs_path()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
  TR.store(p.lang == "tr", Ordering::Relaxed);
  p
}

#[tauri::command]
pub fn get_prefs(st: State<App>) -> Prefs {
  st.prefs.lock().unwrap().clone()
}

#[tauri::command]
pub fn set_prefs(app: AppHandle, st: State<App>, prefs: Prefs) -> Result<(), String> {
  std::fs::write(prefs_path(), serde_json::to_vec_pretty(&prefs).unwrap()).map_err(|e| e.to_string())?;
  let lang_changed = TR.swap(prefs.lang == "tr", Ordering::Relaxed) != (prefs.lang == "tr");
  *st.prefs.lock().unwrap() = prefs;
  if lang_changed {
    refresh_tray(&app);
  }
  Ok(())
}

// ---------- profiller: klavyenin tam görüntüsü + makrolar, %APPDATA%/aulactl/profiles/<ad>.json ----------
fn profile_path(name: &str) -> Result<std::path::PathBuf, String> {
  let name = name.trim();
  if name.is_empty() || name.contains(['/', '\\', ':', '.']) {
    return Err(l!("invalid profile name", "geçersiz profil adı"));
  }
  let dir = config_dir().join("profiles");
  let _ = std::fs::create_dir_all(&dir);
  Ok(dir.join(format!("{name}.json")))
}

#[tauri::command]
pub fn profile_list() -> Vec<String> {
  let mut v: Vec<String> = std::fs::read_dir(config_dir().join("profiles"))
    .map(|d| d.flatten().filter_map(|e| e.path().file_stem().map(|s| s.to_string_lossy().into_owned())).collect())
    .unwrap_or_default();
  v.sort();
  v
}

#[tauri::command(async)]
pub fn profile_save(app: AppHandle, st: State<App>, name: String) -> Result<(), String> {
  let path = profile_path(&name)?;
  let mut v = with_dev(&app, &st, read_raw)?;
  v["macros"] = serde_json::from_slice(&std::fs::read(config_dir().join("macros.json")).unwrap_or_default())
    .unwrap_or(serde_json::json!([]));
  std::fs::write(path, serde_json::to_vec_pretty(&v).unwrap()).map_err(|e| e.to_string())?;
  refresh_tray(&app);
  Ok(())
}

#[tauri::command(async)]
pub fn profile_apply(app: AppHandle, st: State<App>, name: String) -> Result<(), String> {
  let bytes = std::fs::read(profile_path(&name)?).map_err(|e| e.to_string())?;
  let v: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
  with_dev(&app, &st, |d| restore_value(d, &v))?;
  if let Ok(macros) = serde_json::from_value::<Vec<crate::Macro>>(v["macros"].clone()) {
    if !macros.is_empty() {
      std::fs::write(config_dir().join("macros.json"), serde_json::to_vec_pretty(&macros).unwrap()).ok();
      let buf = crate::macro_buffer(&macros);
      with_dev(&app, &st, |d| d.write(crate::f75::Op::Macro, 0, &buf))?;
    }
  }
  Ok(())
}

#[tauri::command]
pub fn profile_delete(app: AppHandle, name: String) -> Result<(), String> {
  std::fs::remove_file(profile_path(&name)?).map_err(|e| e.to_string())?;
  refresh_tray(&app);
  Ok(())
}

// ---------- anlık kareler ----------
/// Alıcı (sub 1): [R G B n idx…] grupları, en fazla 200 B. Kablolu (sub 0): 3x126 düzlem.
pub fn realtime_frame(kind: Kind, base: &[Rgb], over: &[(usize, Rgb)]) -> (u8, Vec<u8>) {
  let mut colors: Vec<Rgb> = base.iter().copied().chain(std::iter::repeat([0; 3])).take(PLANE).collect();
  for &(i, c) in over {
    if i < PLANE {
      colors[i] = c;
    }
  }
  if kind == Kind::Wired {
    let mut v = vec![0u8; PLANE * 3];
    for (i, c) in colors.iter().enumerate() {
      (v[i], v[PLANE + i], v[2 * PLANE + i]) = (c[0], c[1], c[2]);
    }
    return (0, v);
  }
  let mut groups: Vec<(Rgb, Vec<u8>)> = Vec::new();
  for (i, c) in colors.iter().enumerate().filter(|(_, c)| **c != [0; 3]) {
    match groups.iter_mut().find(|g| g.0 == *c) {
      Some(g) => g.1.push(i as u8),
      None => groups.push((*c, vec![i as u8])),
    }
  }
  // vurgulanan tuşlar önce: 200 B'a sığmazsa en son eklenen (taban) renk grupları düşer
  // ponytail: çok renkli tuş başına düzende bazı tuşlar sönük kalır; gerekirse renkleri niceleyerek grup sayısını düşür
  groups.sort_by_key(|g| !over.iter().any(|o| o.1 == g.0));
  let mut out = Vec::new();
  for (c, idx) in groups {
    if out.len() + 4 + idx.len() > 200 {
      continue;
    }
    out.extend([c[0], c[1], c[2], idx.len() as u8]);
    out.extend(idx);
  }
  (1, out)
}

#[cfg(windows)]
mod caps_win {
  use std::sync::atomic::{AtomicBool, Ordering};
  use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
  use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
  use windows_sys::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
  use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetKeyState;
  use windows_sys::Win32::UI::WindowsAndMessaging::*;

  static CAPS: AtomicBool = AtomicBool::new(false);
  static DOWN: AtomicBool = AtomicBool::new(false);
  static HOOK: std::sync::Once = std::sync::Once::new();

  unsafe extern "system" fn hook(code: i32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
      if code >= 0 && (*(l as *const KBDLLHOOKSTRUCT)).vkCode == 0x14 {
        let down = w as u32 == WM_KEYDOWN || w as u32 == WM_SYSKEYDOWN;
        if down && !DOWN.swap(true, Ordering::Relaxed) {
          CAPS.fetch_xor(true, Ordering::Relaxed); // tekrar eden basılı tutma olaylarını sayma
        } else if !down {
          DOWN.store(false, Ordering::Relaxed);
        }
      }
      CallNextHookEx(std::ptr::null_mut(), code, w, l)
    }
  }

  /// Öndeki pencerenin iş parçacığına bağlanıp gerçek durumu okur; bağlanılamazsa None
  /// (yönetici/UWP pencerelerde AttachThreadInput reddediliyor, cihaz testinde görüldü).
  fn read_attached() -> Option<bool> {
    unsafe {
      let (me, fg) = (GetCurrentThreadId(), GetWindowThreadProcessId(GetForegroundWindow(), std::ptr::null_mut()));
      if fg == 0 || fg == me || AttachThreadInput(me, fg, 1) == 0 {
        return None;
      }
      let on = GetKeyState(0x14) & 1 != 0;
      AttachThreadInput(me, fg, 0);
      Some(on)
    }
  }

  /// Düşük seviyeli klavye kancası Caps basışlarını sayar; bağlanılabilen anlarda gerçek durumla düzeltilir.
  pub fn caps_on() -> bool {
    HOOK.call_once(|| {
      std::thread::spawn(|| unsafe {
        SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook), GetModuleHandleW(std::ptr::null()), 0);
        let mut msg = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {}
      });
    });
    if let Some(on) = read_attached() {
      CAPS.store(on, Ordering::Relaxed);
    }
    CAPS.load(Ordering::Relaxed)
  }
}

pub fn caps_on() -> bool {
  // Tuş durumu iş parçacığına özgü: kuyruğu olmayan arka plan iş parçacığı Caps'i hep kapalı görüyor (cihazda görüldü).
  #[cfg(windows)]
  {
    caps_win::caps_on()
  }
  #[cfg(target_os = "linux")]
  {
    std::fs::read_dir("/sys/class/leds")
      .map(|d| {
        d.flatten().any(|e| {
          e.file_name().to_string_lossy().ends_with("capslock")
            && std::fs::read_to_string(e.path().join("brightness")).is_ok_and(|s| s.trim() != "0")
        })
      })
      .unwrap_or(false)
  }
  #[cfg(not(any(windows, target_os = "linux")))]
  {
    false
  }
}

/// Bildirim ışığı tanı kaydı: %APPDATA%/aulactl/notifier.log
pub(crate) fn log(line: &str) {
  use std::io::Write;
  if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(config_dir().join("notifier.log")) {
    let _ = writeln!(f, "{} {line}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
  }
}

pub(crate) fn send(app: &AppHandle, build: impl FnOnce(&Device) -> (u8, Vec<u8>)) -> &'static str {
  let st = app.state::<App>();
  let Ok(g) = st.dev.try_lock() else { return "busy" }; // başka bir yazma sürüyorsa bu kareyi atla
  let Some(d) = g.as_ref() else { return "device not open" };
  let (sub, data) = build(d);
  match d.realtime(sub, &data) {
    Ok(()) => "sent",
    Err(e) => {
      log(&format!("send error: {e}"));
      "error"
    }
  }
}

/// Caps Lock göstergesi: Caps açık ve klavye tuş başına moddayken ~5 Hz kare akışı.
pub fn notifier(app: AppHandle) {
  std::thread::spawn(move || {
    let mut last = None;
    loop {
      std::thread::sleep(Duration::from_millis(200));
      let st = app.state::<App>();
      let prefs = st.prefs.lock().unwrap().clone();
      let look = st.look.lock().unwrap().clone();
      let caps = caps_on();
      let mut r = "-";
      if prefs.caps_light && look.custom && caps && !crate::music::ACTIVE.load(Ordering::Relaxed) {
        r = send(&app, |d| realtime_frame(d.kind, &look.perkey, &[(CAPS_IDX, prefs.caps_color)]));
      }
      let state = (caps, look.custom, prefs.caps_light, r);
      if last != Some(state) {
        log(&format!("caps={caps} perkey={} caps_light={} colors={} result={r}", look.custom, prefs.caps_light, look.perkey.len()));
        last = Some(state);
      }
    }
  });
}

static LAST_ALERT: AtomicU8 = AtomicU8::new(100);

/// Her pil olayında: tepsi ipucu, %15 ve %5'te bildirim + (tuş başına moddaysa) kırmızı yanıp sönme.
pub fn on_battery(app: &AppHandle, level: u8, charging: bool) {
  if let Some(t) = app.tray_by_id(TRAY_ID) {
    let _ = t.set_tooltip(Some(if charging { l!("AULA F75 · {level}% · charging", "AULA F75 · %{level} · şarjda") } else { l!("AULA F75 · {level}%", "AULA F75 · %{level}") }));
  }
  if charging {
    LAST_ALERT.store(100, Ordering::Relaxed);
    return;
  }
  let Some(th) = [5u8, 15].into_iter().find(|&t| level <= t) else { return };
  if LAST_ALERT.swap(th, Ordering::Relaxed) <= th {
    return; // bu eşik için zaten uyarıldı
  }
  let st = app.state::<App>();
  let prefs = st.prefs.lock().unwrap().clone();
  if prefs.battery_notify {
    let _ = app.notification().builder().title(l!("AULA F75 battery low", "AULA F75 pili azaldı")).body(l!("Battery at {level}%. Time to plug it in.", "Pil %{level}. Şarja takmanız iyi olur.")).show();
  }
  if prefs.battery_flash && st.look.lock().unwrap().custom {
    let app = app.clone();
    std::thread::spawn(move || {
      for _ in 0..3 {
        for _ in 0..4 {
          send(&app, |d| if d.kind == Kind::Wired { realtime_frame(d.kind, &[[255, 0, 0]; PLANE], &[]) } else { (2, vec![255, 0, 0]) });
          std::thread::sleep(Duration::from_millis(100));
        }
        std::thread::sleep(Duration::from_millis(600)); // akış durunca klavye kendi renklerine döner
      }
    });
  }
}

// ---------- Windows açılışında başlama (tepside, pencere gizli: --hidden) ----------
#[tauri::command]
pub fn autostart_get(app: AppHandle) -> bool {
  app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
pub fn autostart_set(app: AppHandle, on: bool) -> Result<(), String> {
  let a = app.autolaunch();
  if on { a.enable() } else { a.disable() }.map_err(|e| e.to_string())
}

/// İlk açılışta açılışta başlatmayı açar; açıksa her açılışta yeniler (kurulum yeri değişirse kayıt yeni exe'yi göstersin).
pub fn autostart_init(app: &AppHandle) {
  let mark = config_dir().join("autostart_init");
  let a = app.autolaunch();
  if !mark.exists() || a.is_enabled().unwrap_or(false) {
    let _ = a.enable();
    let _ = std::fs::write(mark, "");
  }
}

// ---------- tepsi ----------
fn tray_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
  let profiles = profile_list();
  let sub = Submenu::with_id(app, "profiles", l!("Load profile", "Profil yükle"), !profiles.is_empty())?;
  for p in &profiles {
    sub.append(&MenuItem::with_id(app, format!("profile:{p}"), p, true, None::<&str>)?)?;
  }
  Menu::with_items(
    app,
    &[
      &MenuItem::with_id(app, "show", l!("Open aulactl", "aulactl'yi aç"), true, None::<&str>)?,
      &sub,
      &MenuItem::with_id(app, "quit", l!("Quit", "Çıkış"), true, None::<&str>)?,
    ],
  )
}

fn refresh_tray(app: &AppHandle) {
  if let (Some(t), Ok(m)) = (app.tray_by_id(TRAY_ID), tray_menu(app)) {
    let _ = t.set_menu(Some(m));
  }
}

fn show(app: &AppHandle) {
  if let Some(w) = app.get_webview_window("main") {
    let _ = w.show();
    let _ = w.unminimize();
    let _ = w.set_focus();
  }
}

pub fn tray(app: &AppHandle) -> tauri::Result<()> {
  let mut b = TrayIconBuilder::with_id(TRAY_ID).tooltip("AULA F75").menu(&tray_menu(app)?).show_menu_on_left_click(false);
  if let Some(icon) = app.default_window_icon() {
    b = b.icon(icon.clone());
  }
  b.on_menu_event(|app, e| match e.id.as_ref() {
    "show" => show(app),
    "quit" => app.exit(0),
    id => {
      if let Some(name) = id.strip_prefix("profile:") {
        let (app, name) = (app.clone(), name.to_string());
        std::thread::spawn(move || {
          let r = profile_apply(app.clone(), app.state::<App>(), name.clone());
          let body = r.err().unwrap_or_else(|| l!("\"{name}\" loaded to the keyboard", "\"{name}\" klavyeye yüklendi"));
          let _ = app.notification().builder().title("aulactl").body(body).show();
        });
      }
    }
  })
  .on_tray_icon_event(|t, e| {
    if let tauri::tray::TrayIconEvent::Click { button: tauri::tray::MouseButton::Left, .. } = e {
      show(t.app_handle());
    }
  })
  .build(app)?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn frames() {
    // alıcı: Caps (3) kırmızı tek başına → cihazda "sadece Caps yandı" ile aynı bayt dizisi
    assert_eq!(realtime_frame(Kind::Dongle, &[], &[(3, [255, 0, 0])]), (1, vec![255, 0, 0, 1, 3]));
    // taban beyaz iki tuş + Caps kırmızı: vurgulanan grup önce
    let base = vec![[255, 255, 255]; 2];
    assert_eq!(realtime_frame(Kind::Dongle, &base, &[(3, [255, 0, 0])]).1, vec![255, 0, 0, 1, 3, 255, 255, 255, 2, 0, 1]);
    // 200 B sınırı aşılmaz
    let many: Vec<Rgb> = (0..126).map(|i| [i as u8, 1, 1]).collect();
    assert!(realtime_frame(Kind::Dongle, &many, &[]).1.len() <= 200);
    // kablolu: düzlemler
    let (sub, v) = realtime_frame(Kind::Wired, &[], &[(3, [1, 2, 3])]);
    assert_eq!((sub, v.len(), v[3], v[PLANE + 3], v[2 * PLANE + 3]), (0, 378, 1, 2, 3));
  }
}
