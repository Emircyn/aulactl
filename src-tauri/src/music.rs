//! Müziğe tepki: sistem sesi (WASAPI loopback) → frekans bantları → tuş sütunlarında spektrum çubukları.
//! Resmi sürücünün müzik modu da aynı anlık ışık komutunu (0x08/0x88) kullanıyor; klavye tuş başına modda olmalı.

use crate::extras::{caps_on, realtime_frame, send};
use crate::{App, Rgb};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Manager};

pub const BANDS: usize = 16;
const N: usize = 1024; // analiz penceresi (~21 ms @48 kHz)
pub static ACTIVE: AtomicBool = AtomicBool::new(false); // akış sürerken Caps göstergesi kendi karesini göndermesin

/// layout.json'dan her tuş: (LED indeksi, bant, alttan satır 0-5)
fn keys() -> Vec<(usize, usize, usize)> {
  let v: serde_json::Value = serde_json::from_str(include_str!("../../layout.json")).unwrap();
  let k: Vec<(usize, f64, i64)> = v["keys"]
    .as_array()
    .unwrap()
    .iter()
    .map(|k| {
      let r = &k["rect"];
      (k["index"].as_u64().unwrap() as usize, (r[0].as_f64().unwrap() + r[2].as_f64().unwrap()) / 2.0, r[1].as_i64().unwrap())
    })
    .collect();
  // satır üst kenarları birkaç piksel oynuyor (80/81, 234-236): 10 px'ten yakın olanlar aynı satır
  let mut ys: Vec<i64> = k.iter().map(|x| x.2).collect();
  ys.sort();
  ys.dedup_by(|b, a| *b - *a < 10);
  let row = |y: i64| ys.iter().rposition(|&r| r <= y).unwrap();
  let (x0, x1) = k.iter().fold((f64::MAX, f64::MIN), |(a, b), x| (a.min(x.1), b.max(x.1)));
  k.iter()
    .map(|&(i, x, y)| {
      let band = (((x - x0) / (x1 - x0 + 1.0)) * BANDS as f64) as usize;
      (i, band, ys.len() - 1 - row(y))
    })
    .collect()
}

/// 60 Hz - 12 kHz arası logaritmik bantların genliği.
/// ponytail: bant başına birkaç DFT kutusu üzerinden düz toplama (~200k işlem/kare); yetmezse FFT crate'i
pub fn spectrum(s: &[f32], rate: f32) -> [f32; BANDS] {
  let mut out = [0f32; BANDS];
  let bin_hz = rate / s.len() as f32;
  for (b, o) in out.iter_mut().enumerate() {
    let f = |i: usize| 60.0 * (12000f32 / 60.0).powf(i as f32 / BANDS as f32);
    let (lo, hi) = ((f(b) / bin_hz) as usize, ((f(b + 1) / bin_hz) as usize).max((f(b) / bin_hz) as usize + 1));
    for k in lo.max(1)..hi.min(s.len() / 2) {
      let w = std::f32::consts::TAU * k as f32 / s.len() as f32;
      let (mut re, mut im) = (0f32, 0f32);
      for (n, x) in s.iter().enumerate() {
        let (sn, cs) = (w * n as f32).sin_cos();
        re += x * cs;
        im -= x * sn;
      }
      *o = o.max((re * re + im * im).sqrt());
    }
  }
  out
}

fn hue(h: f32) -> Rgb {
  let f = |n: f32| {
    let k = (n + h * 6.0) % 6.0;
    (255.0 * (1.0 - (k.min(4.0 - k).clamp(0.0, 1.0)))) as u8
  };
  [f(5.0), f(3.0), f(1.0)]
}

/// Bant seviyeleri (0-1) → tuş renkleri. Gökkuşağında renk sütuna göre; alıcıya grup başına bir renk gider.
pub fn colors(keys: &[(usize, usize, usize)], lv: &[f32; BANDS], color: Rgb, rainbow: bool) -> Vec<(usize, Rgb)> {
  keys
    .iter()
    .filter(|&&(_, b, row)| lv[b] * 6.0 > row as f32 + 0.3)
    .map(|&(i, b, _)| (i, if rainbow { hue(b as f32 / BANDS as f32 * 0.83) } else { color }))
    .collect()
}

/// Sistem sesinin kaydedileceği aygıt ve ayarı. Windows (WASAPI) ve macOS 14.2+ (CoreAudio) çıkış aygıtında
/// giriş akışını loopback olarak açar; Linux'ta PulseAudio/PipeWire-pulse varsayılan çıkışın ".monitor" kaynağını verir.
fn loopback() -> Option<(cpal::Device, cpal::SupportedStreamConfig)> {
  use cpal::traits::{DeviceTrait, HostTrait};
  let host = cpal::default_host();
  let out = host.default_output_device()?;
  #[cfg(target_os = "linux")]
  {
    let mon = format!("{}.monitor", out.id().ok()?.id());
    let dev = host.input_devices().ok()?.find(|d| d.id().is_ok_and(|i| i.id() == mon))?;
    let cfg = dev.default_input_config().ok()?;
    Some((dev, cfg))
  }
  #[cfg(not(target_os = "linux"))]
  {
    let cfg = out.default_output_config().ok()?;
    Some((out, cfg))
  }
}

pub fn run(app: AppHandle) {
  use cpal::traits::{DeviceTrait, StreamTrait};
  use std::sync::{Arc, Mutex};
  use std::time::Duration;

  std::thread::spawn(move || {
    let keys = keys();
    loop {
      std::thread::sleep(Duration::from_millis(300));
      let wanted = || {
        let st = app.state::<App>();
        st.prefs.lock().unwrap().music && st.look.lock().unwrap().custom
      };
      if !wanted() {
        continue;
      }
      let Some((dev, cfg)) = loopback() else {
        std::thread::sleep(Duration::from_secs(3));
        continue;
      };
      let (ch, rate) = (cfg.channels() as usize, cfg.sample_rate() as f32);
      let buf = Arc::new(Mutex::new(Vec::<f32>::with_capacity(N * 2)));
      let failed = Arc::new(AtomicBool::new(false));
      let (b2, f2) = (buf.clone(), failed.clone());
      let stream = dev.build_input_stream(
        cfg.config(),
        move |d: &[f32], _: &cpal::InputCallbackInfo| {
          let mut b = b2.lock().unwrap();
          b.extend(d.chunks(ch).map(|f| f.iter().sum::<f32>() / ch as f32));
          let extra = b.len().saturating_sub(N);
          b.drain(..extra);
        },
        move |e| {
          // Xrun sessizlikte sürekli geliyor ve zararsız (cihazda görüldü); gerisi aygıt değişti/gitti demek
          if e.kind() != cpal::ErrorKind::Xrun {
            crate::extras::log(&format!("audio stream error: {e}"));
            f2.store(true, Ordering::Relaxed);
          }
        },
        None,
      );
      let Ok(stream) = stream.inspect_err(|e| crate::extras::log(&format!("audio capture failed: {e}"))) else {
        std::thread::sleep(Duration::from_secs(3));
        continue;
      };
      if stream.play().is_err() {
        continue;
      }
      ACTIVE.store(true, Ordering::Relaxed);
      let (mut lv, mut peak) = ([0f32; BANDS], 1e-3f32);
      // aygıt değişince (kulaklık takıldı vb.) akış hata verir → dış döngü yeniden açar
      while wanted() && !failed.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(66)); // ~15 kare/sn; alıcıda kare başına ~10 paket
        let s: Vec<f32> = buf.lock().unwrap().clone();
        let p = app.state::<App>().prefs.lock().unwrap().clone();
        let sp = if s.len() == N { spectrum(&s, rate) } else { [0.0; BANDS] };
        // otomatik seviye: tepe yavaşça söner, ses düzeyinden bağımsız çubuklar
        peak = sp.iter().fold(peak * 0.995, |a, &b| a.max(b)).max(1e-3);
        let decay = 0.4 + p.music_smooth as f32 * 0.06; // 0 → hızlı düşüş, 9 → yumuşak
        for (l, v) in lv.iter_mut().zip(sp) {
          *l = (v / peak * p.music_gain).min(1.0).max(*l * decay);
        }
        let mut over = colors(&keys, &lv, p.music_color, p.music_rainbow);
        if p.caps_light && caps_on() {
          over.push((crate::extras::CAPS_IDX, p.caps_color));
        }
        send(&app, |d| realtime_frame(d.kind, &[], &over));
      }
      ACTIVE.store(false, Ordering::Relaxed);
      drop(stream);
    }
  });
}


#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn spectrum_and_bars() {
    // 1 kHz sinüs en güçlü olarak 1 kHz'i içeren bantta görünmeli
    let rate = 48000.0;
    let s: Vec<f32> = (0..N).map(|n| (std::f32::consts::TAU * 1000.0 * n as f32 / rate).sin()).collect();
    let sp = spectrum(&s, rate);
    let top = (0..BANDS).max_by(|&a, &b| sp[a].total_cmp(&sp[b])).unwrap();
    let edge = |i: usize| 60.0 * (12000f32 / 60.0).powf(i as f32 / BANDS as f32);
    assert!(edge(top) <= 1000.0 && 1000.0 < edge(top + 1), "bant {top}");

    let k = keys();
    assert_eq!(k.len(), 80);
    assert_eq!(k.iter().map(|x| x.2).max(), Some(5)); // 6 satır
    assert!(k.iter().all(|x| x.1 < BANDS));
    // sessizlikte hiç tuş yanmaz, tam seviyede hepsi
    assert!(colors(&k, &[0.0; BANDS], [0, 255, 0], false).is_empty());
    assert_eq!(colors(&k, &[1.0; BANDS], [0, 255, 0], true).len(), 80);
  }
}
