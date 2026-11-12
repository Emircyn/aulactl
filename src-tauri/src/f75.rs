//! AULA F75 HID protokolü. Kaynak: re-tools/aula-f75/PROTOCOL.md (OemDrv.exe'den çözüldü, cihazda doğrulandı).
//! Kablolu (258A:010C): 520 baytlık feature raporu, rid 6, checksum yok.
//! 2.4G alıcı (3554:FA09): 20 baytlık output/input raporu, rid 0x13, son bayt toplam, paket başına ACK.

use hidapi::{HidApi, HidDevice};
use serde::Serialize;
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub const MATRIX_LEN: usize = 128;
pub const LED_LEN: usize = 128;
pub const EFFECTS: usize = 18;
pub const PALETTE_LEN: usize = (EFFECTS + 1) * 21; // boş ilk satır + mod başına 7 renk x 3 bayt
pub const PLANE: usize = 126; // tuş başına renk: R, G, B düzlemleri

const WIRED: (u16, u16) = (0x258A, 0x010C);
const DONGLE: (u16, u16) = (0x3554, 0xFA09);
const WIRED_RID: u8 = 6;
const DONGLE_RID: u8 = 0x13; // OemDrv 0x15 yazıyor ama FA09 alıcısı yalnız 0x13'ü kabul ediyor

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
  Wired,
  Dongle,
  Bluetooth,
}

/// Klavye Bluetooth'ta doğrudan bilgisayara bağlı mı? OemDrv bu modda ayar yapmıyor; biz yalnız gösteriyoruz.
/// ponytail: BT'deki VID/PID cihazda görülmedi — Windows'un BT HID yolu ("bth") + AULA/F75 adıyla tahmin; gerçek
/// BT bağlantısında `hidapi` listesine bakıp kesinleştir.
pub fn bluetooth_present(api: &HidApi) -> bool {
  api.device_list().any(|i| {
    let ids = (i.vendor_id(), i.product_id());
    let name = format!("{} {}", i.manufacturer_string().unwrap_or(""), i.product_string().unwrap_or("")).to_lowercase();
    let path = i.path().to_string_lossy().to_lowercase();
    ids != WIRED && ids != DONGLE && (name.contains("aula") || name.contains("f75")) && (path.contains("bth") || path.contains("bluetooth"))
  })
}

#[derive(Clone, Copy, Debug)]
pub enum Op {
  Matrix,
  Led,
  Macro,
  PerKey,
  Palette,
}

impl Op {
  fn write_cmd(self, k: Kind) -> u8 {
    match (k, self) {
      (Kind::Bluetooth, _) => 0, // Device hiçbir zaman Bluetooth türüyle açılmaz
      (Kind::Wired, Op::Matrix) => 0x03,
      (Kind::Wired, Op::Led) => 0x04,
      (Kind::Wired, Op::Macro) => 0x05,
      (Kind::Wired, Op::PerKey) => 0x06,
      (Kind::Wired, Op::Palette) => 0x0A,
      (Kind::Dongle, Op::Matrix) => 0x01,
      (Kind::Dongle, Op::PerKey) => 0x02,
      (Kind::Dongle, Op::Macro) => 0x03,
      (Kind::Dongle, Op::Led) => 0x04,
      (Kind::Dongle, Op::Palette) => 0x09,
    }
  }
  fn read_cmd(self, k: Kind) -> u8 {
    self.write_cmd(k) | if k == Kind::Wired { 0x80 } else { 0x40 }
  }
}

#[derive(Clone, Copy, Serialize, Debug)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Event {
  Battery { level: u8, charging: bool },
  Online { online: bool },
  Other { code: u8, a: u8, b: u8 },
}

/// Alıcının kendiliğinden gönderdiği olay raporu: 13 0A 01 00 04 <olay> <a> <b> … <sum>
pub fn parse_event(p: &[u8]) -> Option<Event> {
  if p.len() < 8 || p[1] != 0x0A {
    return None;
  }
  Some(match p[5] {
    0x05 => Event::Battery { level: p[6], charging: p[7] & 0x10 != 0 },
    0x02 => Event::Online { online: p[6] != 0 },
    c => Event::Other { code: c, a: p[6], b: p[7] },
  })
}

pub type Sink = Arc<dyn Fn(Event) + Send + Sync>;

/// Klavyenin yüzdesi voltajdan tahmin: kablo takılınca anında ~+20 sıçrıyor, çıkınca birkaç dakikada oturuyor
/// (cihazda görüldü: 45→65, 62→72→57). Takma anındaki sıçramayı düşer; pildeyken değer yükselmez.
/// ponytail: basit sezgisel; gerçek kapasite eğrisi bilinmiyor.
#[derive(Default)]
pub struct BatteryFilter {
  shown: Option<u8>,
  charging: bool,
  offset: u8,
}

impl BatteryFilter {
  pub fn update(&mut self, raw: u8, charging: bool) -> u8 {
    let raw = raw.min(100);
    let shown = match self.shown {
      None => raw,
      Some(s) if charging => {
        if !self.charging {
          self.offset = raw.saturating_sub(s);
        }
        s.max(raw.saturating_sub(self.offset))
      }
      Some(s) => s.min(raw),
    };
    (self.shown, self.charging) = (Some(shown), charging);
    shown
  }
}

pub struct Device {
  pub kind: Kind,
  pub fw: u16,
  dev: HidDevice,
  rx: Option<Receiver<[u8; 20]>>, // alıcı: olay olmayan yanıt paketleri
  pub last_battery: Arc<Mutex<Option<(u8, bool)>>>,
}

fn err<E: std::fmt::Display>(e: E) -> String {
  e.to_string()
}

impl Device {
  /// Önce kablolu, yoksa alıcı. Kablolu: 0xFF00 koleksiyonlarından 520 B feature'ı GetLED imzasıyla seçer.
  pub fn open(sink: Sink) -> Result<Device, String> {
    let api = HidApi::new().map_err(err)?;
    for info in api.device_list() {
      if (info.vendor_id(), info.product_id()) != WIRED || info.usage_page() != 0xFF00 {
        continue;
      }
      let Ok(dev) = info.open_device(&api) else { continue };
      let d = Device { kind: Kind::Wired, fw: info.release_number(), dev, rx: None, last_battery: Default::default() };
      if d.read(Op::Led, 0, LED_LEN).is_ok_and(|b| b[LED_LEN - 2..] == [0x5A, 0xA5]) {
        return Ok(d);
      }
    }
    // ponytail: usage_page'e güveniyor; linux-native arka ucu doldurmazsa interface_number==1'e geç
    let info = api
      .device_list()
      .find(|i| (i.vendor_id(), i.product_id()) == DONGLE && i.usage_page() == 0xFF02)
      .ok_or_else(|| l!("AULA F75 not found (neither the cable nor the 2.4G receiver is plugged in)", "AULA F75 bulunamadı (kablolu ya da 2.4G alıcı takılı değil)"))?;
    let dev = info.open_device(&api).map_err(err)?;
    let reader = api.open_path(info.path()).map_err(err)?; // senkron G/Ç: okuma ve yazma ayrı tanıtıcı
    let (tx, rx) = channel();
    let last_battery: Arc<Mutex<Option<(u8, bool)>>> = Default::default();
    let lb = last_battery.clone();
    thread::spawn(move || {
      let mut buf = [0u8; 20];
      let mut filter = BatteryFilter::default();
      loop {
        match reader.read_timeout(&mut buf, 500) {
          Ok(20) => match parse_event(&buf) {
            Some(Event::Battery { level, charging }) => {
              let level = filter.update(level, charging);
              *lb.lock().unwrap() = Some((level, charging));
              sink(Event::Battery { level, charging })
            }
            Some(e) => sink(e),
            None => {
              if tx.send(buf).is_err() {
                return; // Device kapandı
              }
            }
          },
          Ok(_) => {}
          // alıcı takılıyken de kablosuz bağlantı kopunca birkaç sn os error 433/1167 veriyor (cihazda görüldü);
          // burada çıkılırsa Device her okumada "yanıt vermedi" der → bekle, sürdür. Gerçek çıkarmada watch()
          // Device'ı bırakır: cmd 0 olan boş paket (hiçbir yanıtla eşleşmez) gönderilemeyince çıkılır.
          Err(_) => {
            if tx.send([0; 20]).is_err() {
              return;
            }
            thread::sleep(Duration::from_millis(200));
          }
        }
      }
    });
    Ok(Device { kind: Kind::Dongle, fw: info.release_number(), dev, rx: Some(rx), last_battery })
  }

  pub fn read(&self, op: Op, sub: u8, len: usize) -> Result<Vec<u8>, String> {
    match self.kind {
      Kind::Wired | Kind::Bluetooth => self.wired_io(op.read_cmd(self.kind), sub, None, len),
      Kind::Dongle => {
        let mut out = self.dongle_read(op.read_cmd(self.kind), sub, len)?;
        // alıcı katmanı 36 paket = 504 B döndürüyor; son dword'deki imza gelmiyor. İmzasız katman geri yazılınca
        // klavye o katmanı okumaya yanıt vermeyi kesti (cihazda görüldü) → imzayı burada tamamla.
        if matches!(op, Op::Matrix) && len == MATRIX_LEN * 4 {
          out[len - 4..].copy_from_slice(&[0, 0, 0x5A, 0xA5]);
        }
        Ok(out)
      }
    }
  }

  pub fn write(&self, op: Op, sub: u8, data: &[u8]) -> Result<(), String> {
    let cmd = op.write_cmd(self.kind);
    match (self.kind, op) {
      (Kind::Wired | Kind::Bluetooth, _) => self.wired_io(cmd, sub, Some(data), data.len()).map(|_| ()),
      // OemDrv makroyu alıcıya 512'lik bloklar halinde, sub = blok no ile ve aralarında 100 ms beklerek yollar
      (Kind::Dongle, Op::Macro) => data.chunks(512).enumerate().try_for_each(|(i, c)| {
        thread::sleep(Duration::from_millis(100));
        self.dongle_write(cmd, i as u8, c)
      }),
      (Kind::Dongle, _) => self.dongle_write(cmd, sub, data),
    }
  }

  /// Kablolu: [rid, cmd, sub, 0, toplam, no, lenLo, lenHi, veri…] — okumada başlık gönderilip GetFeature ile alınır.
  fn wired_io(&self, cmd: u8, sub: u8, data: Option<&[u8]>, len: usize) -> Result<Vec<u8>, String> {
    let n = len.div_ceil(512).max(1);
    let mut out = Vec::with_capacity(len);
    for i in 0..n {
      let chunk = (len - i * 512).min(512);
      let mut f = [0u8; 520];
      f[..8].copy_from_slice(&[WIRED_RID, cmd, sub, 0, n as u8, i as u8, chunk as u8, (chunk >> 8) as u8]);
      if let Some(d) = data {
        f[8..8 + chunk].copy_from_slice(&d[i * 512..i * 512 + chunk]);
      }
      thread::sleep(Duration::from_millis(20));
      self.dev.send_feature_report(&f).map_err(err)?;
      if data.is_none() {
        thread::sleep(Duration::from_millis(20));
        let mut r = [0u8; 520];
        r[0] = WIRED_RID;
        self.dev.get_feature_report(&mut r).map_err(err)?;
        out.extend_from_slice(&r[8..8 + chunk]);
      }
    }
    Ok(out)
  }

  fn dongle_frame(cmd: u8, total: u8, idx: u8, sub_len: u8, data: &[u8]) -> [u8; 20] {
    let mut f = [0u8; 20];
    f[..5].copy_from_slice(&[DONGLE_RID, cmd, total, idx, sub_len]);
    f[5..5 + data.len()].copy_from_slice(data);
    f[19] = f[..19].iter().fold(0u8, |a, b| a.wrapping_add(*b));
    f
  }

  fn next_reply(&self, cmd: u8, timeout: Duration) -> Option<[u8; 20]> {
    let rx = self.rx.as_ref()?;
    let end = Instant::now() + timeout;
    loop {
      match rx.recv_timeout(end.saturating_duration_since(Instant::now())) {
        Ok(p) if p[1] == cmd => return Some(p),
        Ok(_) => continue, // başka komutun geç kalan yanıtı
        Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => return None,
      }
    }
  }

  fn drain(&self) {
    if let Some(rx) = &self.rx {
      while rx.try_recv().is_ok() {}
    }
  }

  /// Yarıda kalan aktarım klavyeye uygulanmıyor (cihazda görüldü) → hata olursa baştan bir kez daha.
  fn dongle_write(&self, cmd: u8, sub: u8, data: &[u8]) -> Result<(), String> {
    self.dongle_write_once(cmd, sub, data).or_else(|_| {
      thread::sleep(Duration::from_millis(300));
      self.dongle_write_once(cmd, sub, data)
    })
  }

  /// Alıcı yazma (SendData @0x48a0a0): 14 baytlık paketler, her biri ACK bekler; ACK [2]&0x80 = hata.
  fn dongle_write_once(&self, cmd: u8, sub: u8, data: &[u8]) -> Result<(), String> {
    self.drain();
    let total = data.len().div_ceil(14).max(1);
    for (i, c) in data.chunks(14).enumerate() {
      let f = Self::dongle_frame(cmd, total as u8, i as u8, (sub << 4) | c.len() as u8, c);
      let mut tries = 0;
      loop {
        self.dev.write(&f).map_err(err)?;
        if cmd & 0x80 != 0 {
          break; // anlık komutlar ACK beklemez
        }
        // 256 B'lık sayfa başlarında (paket 0, 18) ACK ~2.4 sn gecikiyor (cihazda ölçüldü; flaş silme) — normal ACK hızlı
        match self.next_reply(cmd, Duration::from_millis(5000)) {
          Some(a) if a[2] & 0x80 != 0 => return Err(format!("klavye paketi reddetti (cmd 0x{cmd:02X}, paket {i})")),
          Some(_) => break,
          None if tries < 4 => tries += 1,
          None => return Err(l!("receiver did not respond (cmd 0x{cmd:02X}, packet {i}); the keyboard may be asleep", "alıcı yanıt vermedi (cmd 0x{cmd:02X}, paket {i}) — klavye uykuda olabilir")),
        }
      }
    }
    Ok(())
  }

  /// Alıcı okuma (ReadData @0x48a340): [rid, cmd, 1, 0, sub<<4] → paketler [rid, cmd, toplam, no, len, veri…].
  fn dongle_read(&self, cmd: u8, sub: u8, len: usize) -> Result<Vec<u8>, String> {
    for _ in 0..3 {
      self.drain();
      self.dev.write(&Self::dongle_frame(cmd, 1, 0, sub << 4, &[])).map_err(err)?;
      // paketler kendi numarasının yerine yazılır: kablosuzda paket düşebiliyor (cihazda görüldü), eksik varsa baştan oku
      let mut out = vec![0u8; len.div_ceil(14) * 14 + 14];
      let mut got = vec![false; 256];
      // ilk paket kablosuz gidiş-dönüşte ~1.3 sn sürebiliyor (cihazda ölçüldü)
      let mut wait = Duration::from_millis(3000);
      while let Some(p) = self.next_reply(cmd, wait) {
        wait = Duration::from_millis(1000);
        let (idx, total) = (p[3] as usize, (p[2] & 0x7F) as usize);
        let n = (p[4] as usize & 0xF).min(14);
        if idx * 14 + n <= out.len() {
          out[idx * 14..idx * 14 + n].copy_from_slice(&p[5..5 + n]);
          got[idx] = true;
        }
        if idx + 1 >= total {
          if got[..total].iter().all(|&g| g) {
            out.truncate(len);
            out.resize(len, 0);
            return Ok(out);
          }
          break; // eksik paket → tekrar dene
        }
      }
    }
    Err(l!("receiver did not answer the read (cmd 0x{cmd:02X}); the keyboard may be asleep", "alıcı okumaya yanıt vermedi (cmd 0x{cmd:02X}) — klavye uykuda olabilir"))
  }

  /// Anlık ışık (müzik modu @0x48138b): flaşa yazılmaz, onay beklenmez.
  /// sub 0 = 3x126 R/G/B düzlem (tuş başına), 2 = tek renk RGB. Kablolu cmd 0x08, alıcı 0x88.
  pub fn realtime(&self, sub: u8, data: &[u8]) -> Result<(), String> {
    match self.kind {
      Kind::Dongle => self.dongle_write_once(0x88, sub, data),
      _ => self.wired_io(0x08, sub, Some(data), data.len()).map(|_| ()),
    }
  }

  /// ResetDevice @0x48c320: kablolu cmd 0x11, 1 bayt.
  pub fn write_reset(&self) -> Result<(), String> {
    self.wired_io(0x11, 0, Some(&[0]), 1).map(|_| ())
  }

  /// Alıcı yalnız 2 katmanı (normal, FN) yanıtlıyor; 2-3 sessiz kalıyor (cihazda denendi).
  pub fn layers(&self) -> u8 {
    if self.kind == Kind::Wired { 4 } else { 2 }
  }

  /// Alıcı 0x4A = [yüzde, bayrak] (olaydaki gibi bit4 şarj; cihazda `58 01` = %88 pilde). Klavye pil olayını
  /// her zaman göndermiyor (2026-11-09'da 90 sn'de hiç gelmedi) → olay yoksa buradan sorgulanır.
  pub fn battery(&self) -> Option<(u8, bool)> {
    if self.kind != Kind::Dongle {
      return None;
    }
    let d = self.dongle_read(0x4A, 0, 2).ok()?;
    (d[0] != 0).then(|| (d[0].min(100), d[1] & 0x10 != 0))
  }

  pub fn online(&self) -> bool {
    self.kind == Kind::Wired || self.dongle_read(0x07, 0, 1).is_ok_and(|d| d[0] != 0)
  }

  /// Okuyup yazar, sonra geri okuyup karşılaştırır.
  pub fn write_verified(&self, op: Op, sub: u8, data: &[u8]) -> Result<(), String> {
    self.write(op, sub, data)?;
    thread::sleep(Duration::from_millis(60));
    let back = self.read(op, sub, data.len())?;
    if back != data {
      let at = back.iter().zip(data).position(|(a, b)| a != b).unwrap_or(0);
      return Err(l!("written data does not match read-back ({op:?}, first difference at byte {at})", "yazılan veri geri okunanla uyuşmuyor ({op:?}, ilk fark bayt {at})"));
    }
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn frames_and_events() {
    // gerçek cihazdan yakalanan paketler (f75listen.py / diag)
    assert_eq!(Device::dongle_frame(0x07, 1, 0, 0, &[])[19], 0x1B);
    assert_eq!(Device::dongle_frame(0x44, 1, 0, 0, &[])[19], 0x58);
    let bat = hex("130a010004053e01000000000000000000000066");
    assert!(matches!(parse_event(&bat), Some(Event::Battery { level: 0x3E, charging: false })));
    let chg = hex("130a010004053d11000000000000000000000075");
    assert!(matches!(parse_event(&chg), Some(Event::Battery { level: 0x3D, charging: true })));
    assert!(matches!(parse_event(&hex("130a010004020100000000000000000000000025")), Some(Event::Online { online: true })));
    assert_eq!(Op::Palette.read_cmd(Kind::Dongle), 0x49);
    assert_eq!(Op::Matrix.read_cmd(Kind::Wired), 0x83);
  }

  #[test]
  fn battery_filter() {
    let mut f = BatteryFilter::default();
    assert_eq!(f.update(45, false), 45);
    assert_eq!(f.update(65, true), 45); // takınca sıçrama yutulur
    assert_eq!(f.update(70, true), 50); // şarjda artış izlenir
    assert_eq!(f.update(72, false), 50); // çıkınca yükselmez
    assert_eq!(f.update(48, false), 48); // oturunca düşer
  }

  fn hex(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
  }
}
