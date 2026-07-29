# Uslu

**Uslu**, oyunlardaki *Focus Tree* (Odak Ağacı) sistemlerinden esinlenilerek geliştirilmiş, yapılması gerekenleri görsel bir Yönlü Adevirsel Grafik (DAG) üzerinde düzenlemeni sağlayan Rust ve Iced tabanlı bir görev yöneticisidir.

İşlerinizi görsel bir ağaçta düzenleyebilir, ön koşulları (bağımlılıkları) oklarla bağlayabilir, metin/resim içeriklerini zenginleştirip tüm veriyi yerel `.md` (Markdown) dosyalarında saklayabilirsiniz.


## Çalıştırma

```bash
cargo run --release
```

Release binary'sini çalıştırmak için:
```bash
./target/release/uslu
```

## Dosya Yapisi

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

## Öne Çıkan Ana Özellikler

### 1. Sugiyama Tabanlı Otomatik Görsel Düzenleme Engine (Layout Motoru)

Grafiği düzenli tutmak için 4 aşamalı özel Sugiyama Algoritması (`sugiyama.rs`) kullanılır:

* **Katmanlama (Layer Assignment):** Düğümlerin topolojik derinliğine göre katmanları belirlenir ($Layer = \max(Parent) + 1$).
* **Çakışma Azaltma (Crossing Reduction):** Barycenter heuristic ve çift yönlü (sweep down/up) geçişler ile bağlantı çizgilerinin kesişimi minimize edilir.
* **Doğrudan Kardeş İzolasyonu & Merkezleme (Coordinate Assignment):**
* Tek ebeveynli düğümler eşit aralıklı kardeş grupları oluşturur.
* Çoklu ebeveyne sahip (*Merge*) düğümler ve bunların alt ağaçları bağımsız bir blok olarak hesaplanıp ebeveynlerinin orta noktasına hizalanır.


* **Serbest Taşıma (Frozen Node Handling):** Kullanıcı elle bir düğümü taşıdığında konumu kilitlenir (`frozen`), otomatik layout bu düğümü ezmez.

---

### 2. Sonsuz Tuval (Infinite Canvas) & Etkileşimler

* **Pan & Zoom:** Fare ile boş bir alanda sol tık + sürükle ile tuvalde gezinebilir; fare tekerleği ile imlecin olduğu noktaya odaklı yakınlaşma/uzaklaşma (0.2x - 3.0x) yapabilirsiniz.
* **Akıllı Başlık Görünürlüğü & Dinamik Metin Hizalama:** Zoom seviyesine göre başlıkların okunabilirliği korunur. Sığmayan uzun başlıklar otomatik olarak iki satıra bölünür, font boyutu sığacak şekilde ölçeklenir veya kırpılır (`…`).
* **Sütun/Orthogonal Bağlantı Çizgileri (Edge Routing):**
* Bağlantılar sadece yatay ve dikey dik çizgilerden oluşur (çapraz çizgi yoktur).
* Maksimum 2 kırılım noktası bulunur (Parent alt-orta noktadan çıkar, Child üst-orta noktaya iner).
* Yönü gösteren ok başı ve bağlantı çizgileri bulunur.


* **Modüler Görsel Rozetler (Badges):**
* **Shift Basılıyken:** Düğümler üzerinde kırmızılı Silme (`×`) rozetleri ve bağlantı hatlarında Bağlantı Silme rozetleri belirir.
* **Ctrl Basılıyken:** Alt ağaçları açıp kapatmak için Genişlet/Daralt (`+` / `−`) rozetleri belirir.



---

### 3. Esnek Alt Ağaç Gizleme (Tree Collapsing) & Seviye Kontrolü

* **Ctrl + Tık (Tuvalde):** Bir düğüme Ctrl tuşuyla tıklandığında, o düğüme bağımlı tüm alt ağaç dalları görünürlükten gizlenir veya tekrar açılır.
* **Toplu Seviye Görünürlük Sınırı (Slider):** Yan paneldeki *Bilgi & Seviye* sekmesinden slider ile hedeflenen katmana kadar olan düğümler toplu halde açılıp kapatılabilir.

---

### 4. Gelişmiş Yan Panel (Sidebar) & İçerik Yönetimi

Sol yan panel, 4 ana sekmeye ayrılmış modüler bir yapıya sahiptir:

####  Genel (General)

* **Güvenli Düzenleme Kilidi (Lock/Edit):** İçeriğin kazara bozulmasını önlemek için kilit düğmesi bulunur. Kilit açıkken veya boş tuvaldeyken yeni düğüm eklenebilir ve seçili düğümün bilgileri güncellenebilir.
* **Başlık & İlerleme Çubuğu:** Düğüm başlığı ve `%0` ile `%100` arasında ayarlanabilir dinamik ilerleme yüzdesi (renk skalası kırmızıdan yeşile yumuşak geçiş yapar).
* **Resim Seçimi & Dahili Kırpıcı (Cropper):** Resim yükleme ve tuval üzerinde dahili kadraj ayarlama/zoom imkanı sunar.

#### Açıklama (Description) & İlerleme Notları (Progress Notes)

* **Çift Modlu Çalışma:** Düzenleme modunda tam metin editörü (`text_editor`), görünüm modunda ise biçimlendirilmiş Markdown listesi olarak render edilir.
* **Etkileşimli Yapılacaklar Listesi (Task/Checkbox Toggle):** Markdown biçiminde yazılan `- [ ]` veya `- [x]` maddelerine görünüm modunda doğrudan tıklayarak durumları (tamamlandı/tamamlanmadı) anında değiştirilebilir.

#### Bilgi & Seviye (Info & Level)

* Toplam düğüm sayısı, toplam bağlantı sayısı ve derinlik seviyesi istatistikleri.
* Seviye bazlı düğüm dağılımı ve katman bazlı toplu görünürlük denetimi.

---

### 5. Dahili Resim Kırpma Motoru (Image Cropper Canvas)

* Yerel dosya sisteminden resim seçme (Linux'ta `zenity`, Windows'ta `rfd` native diyaloğu).
* Seçilen resmi $240 \times 240$ boyutundaki kırpma tuvalinde fare ile sürükleyerek hizalama ve tekerlek ile zoom yapma.
* Kırpılan görseli $128 \times 128$ PNG formatına dönüştürüp Base64 olarak kaydetme.

---

### 6. Yerel Markdown İçe / Dışa Aktarma (Markdown I/O)

* Tüm grafik yapısı yerel ve insan tarafından okunabilir **`tree.md`** ve görseller **`images.md`** dosyalarında saklanır.
* **`tree.md` Çıktı Biçimi:**
```markdown
# Uslu Focus Tree

## Görev Başlığı
- id: "uuid-v4-string"
- description: "Çok satırlı açıklamalar..."
- progress_notes: "İlerleme notları..."
- image_id: "uuid-v4-string"
- status: 75.0
- prerequisites: ["parent-uuid-1", "parent-uuid-2"]
- position: [120.0, 360.0]

```


* **Otomatik Kaydetme (Autosave):** Grafikte bir değişiklik yapıldığında 60 saniyede bir veya `Ctrl + S` kısayoluna basıldığında (ya da pencere kapatılırken) arka planda otomatik kaydedilir.
* **Döngü Engelleme (Cycle Prevention):** Grafik üzerinde sonsuz döngü yaratacak (A → B → C → A) hatalı ebeveyn-çocuk bağlantıları otomatik olarak tespit edilir ve engellenir.

---

### 7. Çapraz Platform (Cross-Platform) & Cross-Compilation

* **NixOS / Linux:** Fully native Nix devshell yapısı (`shell.nix`), Wayland/X11 & Vulkan/OpenGL grafik desteği.
* **Windows Cross-Compile:** MinGW kütüphaneleri ve Fenix Rust toolchain desteği ile Linux ortamından doğrudan `.exe` çıktısı alabilme.
* **Türkçe Karakter Desteği:** Arayüz ikonları için *Material Symbols* fontu ile Türkçe karakter destekli sistem fontlarının hibrit kullanımı.

---

##  Kullanım İpuçları & Kısayollar

| Eylem | Yöntem |
| --- | --- |
| **Kamera Kaydırma (Pan)** | Boş alanda **Sol Tık + Sürükle** |
| **Yakınlaşma / Uzaklaşma** | **Fare Tekerleği** |
| **Düğüm Taşıma** | Düğüm üzerinde **Sol Tık + Sürükle** |
| **Düğüm Seçme** | Düğüme **Sol Tık** |
| **Seçimi Kaldırma** | Boş tuvala **Sol Tık** |
| **Bağlantı (Edge) Ekleme** | Düğüm seçiliyken **Shift + Tık** (Hedef düğüme) |
| **Düğüm / Bağlantı Silme** | **Shift** basılı tutun, beliren kırmızı **`×`** rozetine tıklayın |
| **Alt Ağaç Gizle / Aç** | **Ctrl** basılı tutun, düğüm üzerindeki **`+` / `−**` rozetine tıklayın |
| **Manuel Kaydetme** | **Ctrl + S** |
