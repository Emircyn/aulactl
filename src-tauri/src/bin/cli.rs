//! Cihaz testi için küçük CLI: aulactl-cli read | events <sn> | brightness <efekt-no> <0-9>
use aulactl_lib::f75::{Device, LED_LEN, Op};
use std::sync::Arc;

fn main() -> Result<(), String> {
  let args: Vec<String> = std::env::args().skip(1).collect();
  if args.first().map(String::as_str) == Some("caps") {
    // arka plan iş parçacığından Caps durumu (bildirim ışığı böyle okuyor)
    return std::thread::spawn(|| {
      for _ in 0..12 {
        println!("caps={}", aulactl_lib::extras::caps_on());
        std::thread::sleep(std::time::Duration::from_millis(500));
      }
      Ok(())
    })
    .join()
    .unwrap();
  }
  if args.first().map(String::as_str) == Some("fw") {
    // sitedeki firmware'i indirip sürümünü klavyeninkiyle karşılaştırır (klavyeye yazmaz)
    println!("{}", serde_json::to_string(&aulactl_lib::firmware::firmware_check()?).unwrap());
    return Ok(());
  }
  let d = Device::open(Arc::new(|e| println!("olay: {e:?}")))?;
  println!("connected: {:?}, fw {:X}, online {}", d.kind, d.fw, d.online());
  match args.first().map(String::as_str) {
    Some("read") | None => println!("{}", serde_json::to_string_pretty(&aulactl_lib::read_raw(&d)?).unwrap()),
    Some("events") => std::thread::sleep(std::time::Duration::from_secs(args.get(1).and_then(|s| s.parse().ok()).unwrap_or(30))),
    Some("brightness") => {
      // en zararsız yazma testi: tek bir efektin parlaklığı, oku-değiştir-yaz + geri okuma
      let (i, v): (usize, u8) = (args[1].parse().unwrap(), args[2].parse().unwrap());
      let mut b = d.read(Op::Led, 0, LED_LEN)?;
      println!("before: effect {i} brightness {}", b[0x3A + 2 * i]);
      b[0x3A + 2 * i] = v.min(9);
      d.write_verified(Op::Led, 0, &b)?;
      println!("written and verified by read-back: {}", d.read(Op::Led, 0, LED_LEN)?[0x3A + 2 * i]);
    }
    Some("led") => {
      // led <ofset> <değer> [<ofset> <değer>…] — LED bloğunda alan(lar) değiştir
      let mut b = d.read(Op::Led, 0, LED_LEN)?;
      for kv in args[1..].chunks(2) {
        let (o, v) = (usize::from_str_radix(kv[0].trim_start_matches("0x"), 16).unwrap(), kv[1].parse::<u8>().unwrap());
        println!("LED[0x{o:02X}] {} -> {v}", b[o]);
        b[o] = v;
      }
      d.write_verified(Op::Led, 0, &b)?;
      println!("verified");
    }
    Some("key") => {
      // key <katman> <indeks> <8 hex kod>
      let (l, i): (u8, usize) = (args[1].parse().unwrap(), args[2].parse().unwrap());
      let mut b = d.read(Op::Matrix, l, 512)?;
      println!("katman {l} idx {i}: {:02X?} -> {}", &b[i * 4..i * 4 + 4], args[3]);
      b[i * 4..i * 4 + 4].copy_from_slice(&u32::from_str_radix(&args[3], 16).unwrap().to_be_bytes());
      d.write_verified(Op::Matrix, l, &b)?;
      println!("verified");
    }
    Some("perkey") => {
      // perkey <rrggbb> — tüm tuşlar aynı renk (Self-define modunda görünür)
      let c = u32::from_str_radix(&args[1], 16).unwrap();
      let mut b = vec![0u8; 384];
      for i in 0..126 {
        (b[i], b[126 + i], b[252 + i]) = ((c >> 16) as u8, (c >> 8) as u8, c as u8);
      }
      d.write(Op::PerKey, 0, &b)?;
      let back = d.read(Op::PerKey, 0, 384)?;
      println!("read-back first key: {:02X} {:02X} {:02X}, last 6 B {:02X?}", back[0], back[126], back[252], &back[378..]);
    }
    Some("macro") => {
      // macro <katman> <indeks> <metin> — metni (a-z) yazan makroyu yuva 0'a yazar, tuşa bağlar
      use aulactl_lib::{Macro, MacroStep, macro_buffer};
      let (l, i): (u8, usize) = (args[1].parse().unwrap(), args[2].parse().unwrap());
      let steps = args[3]
        .bytes()
        .flat_map(|c| {
          let hid = c - b'a' + 4;
          [("down", hid, 30), ("up", hid, 30)].map(|(a, k, dl)| MacroStep { action: a.into(), key: k, delay: dl })
        })
        .collect();
      let buf = macro_buffer(&[Macro { name: "test".into(), repeat: 1, count: 1, steps }]);
      println!("makro tamponu {} B: {:02X?}", buf.len(), &buf[..buf.len().min(24)]);
      d.write(Op::Macro, 0, &buf)?;
      let mut b = d.read(Op::Matrix, l, 512)?;
      b[i * 4..i * 4 + 4].copy_from_slice(&[0x03, 0x01, 0x01, 0x00]); // makro, 1 kez, yuva 0
      d.write_verified(Op::Matrix, l, &b)?;
      println!("macro written and bound to key");
    }
    Some("rt") => {
      // rt <rrggbb> [sub] — 5 sn boyunca saniyede ~10 anlık kare
      let c = u32::from_str_radix(&args[1], 16).unwrap();
      let sub: u8 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
      let (r, g, b) = ((c >> 16) as u8, (c >> 8) as u8, c as u8);
      let data = if sub == 2 {
        vec![r, g, b]
      } else {
        let mut v = vec![0u8; 378];
        for i in 0..126 {
          (v[i], v[126 + i], v[252 + i]) = (r, g, b);
        }
        v
      };
      for _ in 0..50 {
        d.realtime(sub, &data)?;
        std::thread::sleep(std::time::Duration::from_millis(100));
      }
      println!("bitti");
    }
    Some("rtk") => {
      // rtk <rrggbb> <idx> [idx…] — anlık tuş listesi (sub 1): [R G B n idx…], 5 sn
      let c = u32::from_str_radix(&args[1], 16).unwrap();
      let idx: Vec<u8> = args[2..].iter().map(|s| s.parse().unwrap()).collect();
      let mut data = vec![(c >> 16) as u8, (c >> 8) as u8, c as u8, idx.len() as u8];
      data.extend(&idx);
      for _ in 0..50 {
        d.realtime(1, &data)?;
        std::thread::sleep(std::time::Duration::from_millis(100));
      }
      println!("bitti");
    }
    Some("readmacro") => {
      let b = d.read(Op::Macro, 0, 512)?;
      let used = b.iter().rposition(|&x| x != 0).map_or(0, |p| p + 1);
      println!("makro tamponu ({used} B dolu): {:02X?}", &b[..used.max(8)]);
    }
    Some("restore") => {
      let v: serde_json::Value = serde_json::from_slice(&std::fs::read(&args[1]).map_err(|e| e.to_string())?).unwrap();
      let unhex = |s: &str| (0..s.len() / 2).map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap()).collect::<Vec<u8>>();
      for (l, s) in v["layers"].as_array().unwrap().iter().enumerate() {
        d.write_verified(Op::Matrix, l as u8, &unhex(s.as_str().unwrap()))?;
      }
      d.write_verified(Op::Led, 0, &unhex(v["led"].as_str().unwrap()))?;
      d.write(Op::PerKey, 0, &unhex(v["perkey"].as_str().unwrap()))?;
      println!("restored");
    }
    Some(x) => return Err(format!("bilinmeyen komut: {x}")),
  }
  Ok(())
}
