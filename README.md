# Uslu

focus tree tarzı yapılacaklar yöneticisi. İşi görsel bir ağaçta düzenlersin, bağımlılıklar oklarla bağlanır, markdown'a export edip tekrar import edebilirsin.

## Çalıştırma

```bash
cargo run --release
```

Release binary'sini çalıştırmak için:
```bash
./target/release/uslu
```

## Mimari

```
src/
├── main.rs        # Uygulama kurulumu, Message enum, update, view
├── models.rs      # FocusNode, Edge, FocusGraph, NodeStatus
├── sugiyama.rs    # 4 aşamalı DAG layout motoru
├── canvas.rs      # Infinite canvas: pan/zoom, drag, edge çizimi
├── sidebar.rs     # Düğüm ekleme/düzenleme formu, import/export
├── image.rs       # Resim ekleme/düzenleme formu
└── markdown.rs    # Markdown <-> FocusGraph serileştirme
```

### Sugiyama Layout (4 aşama)

1. **Layer assignment** — her düğümün katmanı = max(parent katmanı) + 1 (topolojik derinlik)
2. **Crossing reduction** — barycenter heuristic ile birkaç sweep, kenar kesişimlerini azaltır
3. **Coordinate assignment** — her katmanı yatayda ortala, dikeyde katman sırasına göre yerleştir
4. **Edge routing** — canvas tarafında orthogonal (sadece yatay/dikey, maksimum 2 kırılım)

### Canvas etkileşimleri

- **Sol tık + sürükle** (boş alanda) → kamera kaydır (pan)
- **Sol tık + sürükle** (düğümde) → düğümü taşı
- **Sol tık** (düğümde) → seç
- **Mouse tekerleği** → zoom (imleç konumuna göre)
- Sürüklenen düğümlerin konumu layout motoru tarafından ezilmez (`frozen` seti)

### Edge kuralları

- Sadece yatay ve dikey segmentler — asla çapraz
- Maksimum 2 yön değişimi (3 segment)
- Parent alt-merkezinden çıkar, child üst-merkezine iner
- Parent ve child aynı X eksenindeyse düz dikey çizgi (0 kırılım)

## Kullanım

### Düğüm ekleme

1. Sol panele başlık + açıklama yaz
2. ilerleme seç 
3. "Ekle" butonuna bas — Sugiyama layout otomatik çalışır

### Bağlantı ekleme

1. Bir düğüme tıkla (seç)
2. Shift tusuna basili tut
3. Hedef düğüme tıkla — Y eksenine göre parent/child otomatik belirlenir
4. Döngü oluşturacak bağlantılar reddedilir

## Tema

- **Kilitli** — koyu kırmızı
- **Hazır** — gri
- **Devam** — turuncu
- **Tamam** — yeşil

