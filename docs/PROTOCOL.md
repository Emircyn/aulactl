# AULA F75 – HID protokolü ve veri formatları (OemDrv.exe v0x200'den)

Kaynak: AULA F75 resmi sürücüsü `OemDrv.exe` (Windows), sha256 `7c41a8af…2783`, x86 MFC, Ghidra 12.1.4.
Adresler o binary’ye göre. Bu belge birlikte çalışabilirlik (interoperability) amacıyla, cihazla gözlenerek yazıldı.

## Cihazlar
| Mod | VID:PID | Sınıf |
|---|---|---|
| Kablolu (Sinowealth) | 258A:010C | `CDevG5KB` (cihaz tipi 0x18/0x1A), vtable 0x60ff9c |
| 2.4G alıcı | 3554:FA09 (+FA07 varyantı) | `CDevComboFilm`; ApplySetting aynı G5KB veri hazırlayıcısını çağırır |

KB.ini (`Dev\kb\F75KR`): `MatrixLen=128`, `Fw=24`, `Psd=3,0,0,0,1,91`, `LightHW=0..9`, `SpeedHW=0..4`, `RGBIndex=0,1,2`,
`LedOptN = ui_id, hw_id, speed?, light?, direct?, random?, color?`.

> Düzeltme: ilk sürümde "kablolu 520B, checksum'lı" diye yazdığım `AccessData @0x48f3b0` **fare** sınıfına (`CDevG5MS`) ait. Klavyenin çerçevesi aşağıda.

## 1) Kablolu klavye çerçevesi — `CDevG5KB::AccessData` @0x48b350
Feature report, 520 bayt, **checksum yok**, ACK beklenmez:
```
[0] rid      6 (cihaz tipi 0x18) ya da 9 (diğer) — F75'te 6 (cihazda doğrulandı)
[1] cmd      yazma ID'si; okuma = cmd | 0x80
[2] sub      katman / renk grubu
[3] 0
[4] toplam parça (ceil(len/512))
[5] parça indeksi
[6..7] parça uzunluğu (LE, ≤512)
[8..519] veri
```
Okuma: başlığı SetFeature ile gönder, ~20 ms sonra GetFeature(buf[0]=rid) → veri `[8..]`. Yazmada 3 deneme, 70 ms arayla.

| cmd | Yön | Anlam | Boyut | Fonksiyon |
|---|---|---|---|---|
| 0x03 / 0x83 | W/R | tuş matrisi, sub = katman 0‥3 | 128×4 = 512 | 0x48b590 / 0x48b5d0 |
| 0x04 / 0x84 | W/R | LED ayar bloğu | 128 | 0x48ba30 / 0x48ba80 |
| 0x05 | W | makro tamponu | değişken | 0x48b7e0 |
| 0x06 | W | per-key renk matrisi, sub = renk grubu (log: "SetGame") | 384 | 0x48bc80 |
| 0x0A / 0x8A | W/R | mod renk paleti (RgbTab) | (modlar+1)×21 | 0x48c0d0 / 0x48c120 |
| 0x11 | W | ResetDevice (fabrika ayarı) | 1 | 0x48c320 |

Kablosuz alıcıda aynı veriler 20 baytlık output raporlarla gider (aşağıda §5), komut no'ları farklı: 1 matris, 2 per-key renk ("SetGame"), 3 makro, 4 LED, 9 RgbTab, 0x0A onboard.

## 2) Tuş matrisi (cmd 3, katman başına 512 bayt)
- 4 katman (0 = normal, 1 = FN, 2‥3 boş); ini'deki katman maskesi kullanılmayanları atlar.
- Konum = **matris indeksi × 4**. İndeks KB.ini `[KEY]` satırlarının son sayısı (Esc=0, \`=1, Tab=2, Caps=3, LShift=4 … sütun-sıralı). Geçerli aralık < 0x90.
- Katmanın son dword'ü (idx 127) `00 00 5A A5` imzası (F75'te her 4 katmanda var, cihazda doğrulandı).
- Her tuş 4 bayt `[tip][b1][b2][b3]` (`keyinfo_to_hardware_code` @0x48c690):

| tip | b1 | b2 | b3 | Örnek |
|---|---|---|---|---|
| 0x00 klavye | modifier bitleri (HID sırası: LCtrl=1, LShift=2, LAlt=4, LWin=8, R… <<4) | 2. HID kodu | **ana HID kodu** | Esc `00 00 00 29`, LShift `00 02 00 00`, LCtrl `00 01 00 00`, Alt+Tab `00 04 00 2B` |
| 0x01 fare | düğme | … | | sol `01 01 01 00` |
| 0x02 tüketici | 0 | usage hi | usage lo | Play/Pause `02 00 00 CD`, Vol+ `02 00 00 E9` |
| 0x03 makro | mod (1=sayı, 2, 4) | tekrar | tampon no−1 | |
| 0x07 özel | 0 | 0 | fonksiyon | FN+Esc `07 00 00 04`, FN+LWin `07 00 00 01` |
| 0x08 ışık | alt-komut | arg | 0 | FN+↑ `08 03 01 00` (parlaklık+), FN+→ `08 04 01 00` (hız+) |
| 0x0D Fn | | | | Fn tuşu `0D 00 00 00` |

KB.ini `[FN1]` satırlarındaki `0x09,0x01,0xTTxxxxxx` değerleri bu 4 baytın big-endian yazımıdır (kod bswap ediyor @0x463150).
VK→HID tablosu 0x607280'de (`f75profile.py` içinde gömülü).

## 3) LED ayar bloğu (cmd 4 / 0x84, 128 bayt) — doldurma @0x48d680
Uygulama önce GetLED ile bloğu okur, `[0x7E..0x7F] == 5A A5` değilse 5 kez yeniden dener, sadece aşağıdaki alanları değiştirip geri yazar (geri kalanı firmware'in):

| ofset | alan |
|---|---|
| 0x03 | (ini bayrağı açıksa) ek parametre |
| 0x09 | 1 = aktif mod "custom/per-key" modu |
| 0x0A | aktif mod hw kimliği (LedOpt'un 2. sütunu) |
| 0x0B | taban + profil değeri (yön/alt-mod, kesin değil) |
| 0x12‥0x15 | yan LED: mod, renk, parlaklık, hız (ini'de yan LED varsa) |
| 0x16 | ek seçenek (kapalıysa 0) |
| 0x17 | ters mantıklı bayrak (ini'ye bağlı) |
| 0x18 | uyku süresi hw değeri (`SleepHW`), 0 = kapalı |
| 0x1A | tekerlek modu |
| 0x38, 0x39 | `FF FF` |
| 0x3A + 2i | mod i parlaklığı (`LightHW[idx]`) |
| 0x3B + 2i | mod i: `(SpeedHW[idx] << 4) \| renk_idx`; renk_idx 7 = rastgele/gökkuşağı |

Cihazdan okunan örnek: mod tablosu `09 47 09 47 …` = parlaklık 9, hız 4, renk 7 (rastgele) — profildeki (4, 9, rastgele) ile aynı.
Bir mod (tablodaki 12.) `02 00` döndü; aktif mod hw kimliği 13 — profilde kayıtlı moddan (11) farklı, yani FN tuşlarıyla cihazda değiştirilmiş olabilir.
| 0x7E, 0x7F | `5A A5` imza |

### Ayarlar paneli (kaydetme @0x45e6b0, LED bloğuna doldurma @0x48d680)
| Kontrol | profil alanı | LED bloğu | not |
|---|---|---|---|
| Uyku aç/kapa + süre | 0x333C / 0x333E | 0x18 = SleepHW (sn/30), kapalı = 0 | |
| Debounce kademesi | 0x3340 | 0x03 | `ShowDebounce`=0x04000101 → cihaz değeri **0-4**, resmi arayüz +1 ("1-5 Stage"; değer 1 = "2 Stage", ekranda doğrulandı); cihazda 1→2→1 yazılıp doğrulandı |
| **Tap** aç/kapa + "Sensitivity" kaydırıcısı | 0x3334 / 0x3338 | 0x16 (kapalıysa 0) | profilde kapalı, hassasiyet 0x7F; aralığı ve işlevi bilinmiyor |
| Otomatik algılama | 0x3342 | 0x17 | yalnız `ShowAutoDetect=1`; F75'te yok |
Uygulama tarafı ayarları (dil, başlangıçta aç, tepside kal, güncelleme kontrolü) cihaza yazılmaz.
`SendSysParam` (0x34BC) ekranlı modellere saat/CPU/RAM gönderir; F75'te kapalı.

## 4) Renkler
- **RgbTab (cmd 0x0A):** 21 baytlık boş ilk satır + mod başına 7 renk × 3 bayt (bayt sırası `RGBIndex`). Bayrak `0x2ff9` açıksa 512'ye doldurulur ve `[0x1FA..0x1FB] = 5A A5`. Varsayılan palet: kırmızı, yeşil, mavi, sarı, mor, camgöbeği, beyaz.
- **Per-key matris (cmd 6, sub = grup):** 3 düzlem × 126 bayt (R, G, B sırası ini'den), düzlem içinde konum = tuşun LED indeksi; gönderilen uzunluk MatrixLen×3 = 384. Sadece kirli (değişmiş) gruplar gönderilir.

## 5) Kablosuz alıcı: Output Report 0x15, 20 bayt — `CDevComboFilm::SendData` @0x48a0a0
Koleksiyon usage page 0xFF02, in/out 20 bayt (cihazda doğrulandı). 0xFF04'te 8 baytlık feature da var (kullanımı bilinmiyor).
```
[0] 0x15  [1] cmd (+0x10 alıcının 2. yuvası)  [2] toplam parça  [3] parça no
[4] (sub<<4)|len (len ≤ 14)  [5..18] veri  [19] sum(bytes[0..18]) & 0xFF
```
ACK: 20 baytlık input raporu, `[1]==cmd`, `[2]&0x80` = hata, `[0]==0xFF` = kopma. bit7'li cmd'ler ACK beklemez.
Input `[1]==0x0A` = olay: `[3]` 0=klavye/1=mouse, `[5]==2` ise `[6]` = online.

## 5b) Alıcı — cihazda doğrulanan düzeltmeler (2026-10-30)
- Output rapor ID'si **0x13** (OemDrv'nin yazdığı 0x15 → hata 87). Okuma ve yazma için **ayrı tanıtıcı** (senkron G/Ç kilitleniyor).
- Okuma: `[13][cmd][1][0][sub<<4][0…][sum]`, yanıt paketleri `[13][cmd][toplam][no][(sub<<4)|len][14 B]`. Paketler düşebiliyor → **sıra numarasına göre yerleştir**, eksikse tekrar oku.
- Komutlar: oku 0x41 katman (yalnız 0 ve 1; 2-3 cevapsız), 0x42 tuş başına renk (378 B), 0x43 makro (512 B), 0x44 LED, 0x4A pil (2 B: yüzde, bayrak bit0 pilde/bit4 şarj; `58 01` = %88 pilde — 2026-11-09 cihazda; klavye pil olayını her zaman göndermiyor), 0x07 online; **0x49 palet cevapsız**. Yaz: 1 katman, 2 renk, 3 makro, 4 LED, 9 palet. Klavye 1. yuvada (+0x10'lu komutlar cevapsız).
- Katman okuması **504 B** (36 paket) döner, son dword'deki `00 00 5A A5` imzası gelmez. **İmzasız katman geri yazılınca klavye o katmanı okumaya cevap vermeyi bıraktı** → okurken imzayı tamamla, her zaman imzalı yaz.
- Yazma ACK'i 256 B'lık sayfa başlarında (paket 0, 18) ~2.4 sn gecikiyor (flaş silme). Yarıda kalan aktarım uygulanmaz.
- Normal katman ilk okumada ~1.3 sn gecikebilir.
- **Makro olayında tip 0 = normal tuş, 1 = modifier (E0-E7)**, 2 fare, 3/4 X/Y, 5 tekerlek. Tuşa bağlama kodu `[03][mod][sayı][yuva]`. Cihazda "ab" makrosu çalıştı.
- Cihazda doğrulanan yazmalar: LED (efekt, parlaklık, Self-define 0x09=1), tuş başına renk, tuş atama (Caps→Q), makro.

## 5c) Anlık ışık (müzik modu @0x48138b) — cihazda doğrulandı
- Komut: kablolu `0x08` (feature), alıcı `0x88` (ACK'siz). Flaşa yazmaz. **Yalnız klavye tuş başına (LED 0x09=1) moddayken görünür.**
- sub 0: 3×126 R/G/B düzlem — resmi kod yalnız kabloda kullanıyor; alıcıda görünmedi.
- sub 1: tuş listesi, ≤200 B, gruplar `[R][G][B][n][idx×n]` (idx = matris/LED indeksi). Listede olmayan tuşlar söner. Alıcıda doğrulandı (Caps=3 tek başına kırmızı).
- sub 2: tek renk `[R][G][B]`; alıcıda doğrulandı.
- Akış durunca klavye kendi kayıtlı renklerine döner. Firmware Caps açıkken Caps tuşunu kendisi beyaz yakıyor (yerleşik gösterge).
- Windows'ta Caps durumu: arka plan iş parçacığı `GetKeyState` hep 0 görür → öndeki pencereye `AttachThreadInput` (bazı pencerelerde reddedilir) + `WH_KEYBOARD_LL` kancası.

## 6) profile.dct (uygulamanın yerel kaydı, `%LOCALAPPDATA%\BYCOMBO4\AULA F75`)
- Dosya ofseti 4'ten itibaren profil struct'ı; ad UTF-16 (`Profile1`) başta.
- Tuş kayıtları: struct+0x38, 16 bayt × 0x90 tuş × 4 katman. `dword0 = Major<<24 | Minor`, `[4]=modifier`, `[5]=VK1`, `[6]=VK2`, `dword2 = parametre`. Major 2 = normal tuş, 9 = ham donanım kodu.
- Aktif LED modu: struct+0x243C. Mod kayıtları: struct+0x2F88, 0x24 adım: `[hız idx, parlaklık idx, rastgele, renk idx]` + 7 renk dword'ü.

## Pil seviyesi
Resmi yazılım F75'in pilini **hiç okumuyor**:
- `SyncCfg` (0x489d60) pili yalnız bağlantı tipi 2 ise vtable+0x20 üzerinden okur; kablodayken değeri sabit 100 yazar.
- Hem `CDevG5KB` hem `CDevComboFilm` vtable'ında +0x20 = 0x485cc0 → `return 0` (boş). KB.ini'de de `ShowPower=0`.
- Kablosuz okuma fonksiyonu `ReadData` @0x48a340'ın 7 çağıranı arasında pil komutu yok (GetOnline = cmd 7, 2 bayt: yuva1, yuva2).
- Fare sınıfındaki `GetPower` (cmd 0x09 sub 1, checksum'lı çerçeve) klavyede geçerli değil; kablosuzda 0x09 = SetLedRgbTab (yazma) olduğundan denenmemeli.
**Ama alıcı pili kendiliğinden bildiriyor** (2026-10-30 pasif dinlemeyle cihazda doğrulandı, `f75listen.py`).
Alıcının 0xFF02 koleksiyonunda 20 baytlık input raporları:
```
13 0A 01 00 04 <olay> <a> <b> 00 … 00 <sum>      sum = bytes[0..18] toplamı & 0xFF
```
| olay | a | b | gözlem |
|---|---|---|---|
| 05 pil | yüzde (0x3E = %62) | bit4 = şarj oluyor, bit0 = pilden çalışıyor | ~25 sn'de bir; kablo takınca `3d 11`→`3d 10`, çıkınca `3e 01` |
| 02 bağlantı | 0 = koptu, 1 = bağlandı | 0 | klavye kapatıp açınca `02 00` → `02 01` |
| 07 | 0 | 0 | anlamı belirsiz (FN ışık değişikliği?) |

Kablo çıkarıldıktan sonra yüzde önce %72'ye fırlayıp birkaç dakikada %57'ye indi: değer voltajdan tahmin ediliyor ve şarj sonrası oturuyor.
Kablolu modda (258A) pil bildirimi görülmedi (0xFF00 koleksiyonları dinlenmedi; kablo bağlıyken zaten şarjda).

## Dikkat
- Firmware güncelleme `http://120.79.152.79/update/update_version.xml` → `Update.exe`, **düz HTTP, imza kontrolü görülmedi**. Profil bulutu `http://%s/queryprofile.php`, `submitprofile.php`.
- Firmware imajı pakette yok (ayrı `Update.exe`).

## Cihazda doğrulandı (2026-10-30, kablolu, salt-okuma)
- Çerçeve, rid 6, GetLED (0x84) `5A A5` imzası, GetMatrix (0x83) 4 katman ve GetLedRgbTab (0x8A) okundu.
- `f75compare.py`: profile.dct'den üretilen 4 katman (512 B) ve RgbTab (399 B) cihazdakiyle **bayt bayt aynı**.
- 520 baytlık feature koleksiyonu: usage page 0xFF00, usage 0x01 (MI_01). Ayrıca 6 baytlık feature koleksiyonu var.
- Hiçbir yazma komutu gönderilmedi.

## Doğrulanmamış
- LED bloğunda 0x0B/0x16/0x17 alanlarının anlamı ini anahtarlarına tam eşlenmedi; makro tampon formatı ve fare (tip 1) kodlarının ayrıntısı çıkarılmadı.
- Per-key matriste düzlem adımı (126) ile gönderilen uzunluk (384) arasındaki 6 baytlık fark kodda böyle; cihazın ne beklediği denenmedi.

## 7) Firmware güncelleyici (`F75 firmware.exe`, aulastar.com, sha256 f694dab8…a9ce, 2026-10-31)
- Sinowealth "Update Tool V2.0" (PDB `SH25F273_MP_TOOL.pdb`). İmaj exe'nin içinde: `'Z' A5 'SINO'` (@0x18CD58) + BE u16 VID 258A + BE u16 PID 010C + BE u16 boy (0xF000) + şifreli imaj + 4 B anahtar (`85 2D A0 F5`).
- Şifre TEA çözme, 32 tur, BE kelimeler, **delta 0x9E3769B9** (standart değil; sum başlangıcı 0xC6ED3720), anahtar 4 bayt → k0..k3 (her biri ayrı kelime). `fwtool/extract.py`; çözülen imaj `02 9C 74` (LJMP) ile başlıyor, 8051.
- İmajdaki USB cihaz tanımlayıcısı @0x5FC6: bcdDevice **0x1100**. Klavyenin Windows'taki donanım kimliği `REV_1100` → test edilen klavye bu sürümdeydi.
- Akış (sinowisp ile aynı ISP): normal arayüze (usage page FF00) `05 75 …` → önyükleyici 0603:1020 olarak gelir (rapor 5 = 6 B komut, rapor 6 = 0x802 B veri). Yazma sayacı: `05 52 FC F0 04` + GetFeature `05 72` → [2]=='a', [5] bit maskesi: 00→8, 01→7, 03→6, 07→5, 0F→4, 1F→3, 3F→2, 7F→1, diğer→0 **kalan yazma hakkı** (0'da araç "Left Num 0" deyip durur). Silme `05 45…`, yazma `05 57 <boyLE>` + 0x800'lük `06 77…` sayfaları (ilk baytı 0 gönderilir, en sonda ilk sayfa tekrar yazılır), okuma `05 52 <boy>` + `06 72`, bitiş `05 55…`.
- **Önyükleyicinin toplam ~8 yazma hakkı var gibi görünüyor** → gereksiz güncelleme yapma.
