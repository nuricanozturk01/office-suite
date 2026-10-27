# Kalan eksikler için plan

> **Nereden başlanacak:** Madde 3 (sunucu tarafı `sort=updated`), ilk adım
> `crates/platform/src/pagination.rs` (opak imleç: `<unix mikrosaniye>.<uuid>`, ayrıştırma + birim test),
> ardından `modules/documents` (repository, port, servis, route, test, migration), sonra aynı kalıp
> `pdfs`, `spreadsheets`, `presentations`; en son web (`entities/*/api/queries.ts`, `use-all-files.ts`).
> Sonra sırayla Madde 9 → 1 → 8 → 6 → 2 → 4 → 7. Madde 5 yapılmaz.
>
> **Açık karar (başlamadan önce kullanıcıdan onay):** Madde 3'te `sort=updated` isteğe bağlı parametre
> yerine, listelerin her zaman son düzenlemeye göre sıralanması öneriliyor (tek istemci "Dosyalarım";
> dört modülde çift kod yolu gerekmez). Onay gelirse Madde 3 buna göre sadeleşir (parametre yok,
> imleç her zaman opak, eski id sırası kalkar).
>
> **Durum:** henüz hiçbir madde uygulanmadı; kodda bu plana ait değişiklik yok.
> Kaynak: bu dosyanın çalışma kopyası `~/.claude/plans/snug-roaming-dove.md`; repodaki kopya esastır.

## Context

Sürüm geçmişi, sunucu tarafı birleştirme, kişi adları, rol bazlı geri yükleme, Docs gövde dışı
birleşme ve `scripts/upgrade-upstream.sh` tamam. Elde şu eksikler kaldı:

1. Sheets ve Slides'ta görünüm koruma ve yeniden adlandırma derlendi ama tarayıcıda denenmedi.
2. PDF'te yeniden adlandırma ve görünüm koruma yok; PDF birleştirmesi yok.
3. Liste API'si id'ye göre sayfalıyor; çok eski ama yeni düzenlenmiş dosyalar ilk sayfada çıkmıyor.
4. Docs'ta yenilemeden sonra imleç bir satır kayık.
5. İçe aktarılan Yjs belgeleri boş açılıyor.
6. Sürüm geçmişi 50 sürümle sınırlı ve sabit kodlu.
7. Docs birleştirmesi paragraf bazlı; aynı paragrafta iki kişi yazarsa çakışma çıkıyor.
8. Yükseltme script'i hiç gerçek bir yükseltmede çalıştırılmadı.
9. (Yeni istek) Kaydet != versiyon: her Ctrl+S / otomatik kayıt versiyon üretmemeli; versiyonu kullanıcı
   isteyince oluşturmalı, isim verebilmeli, isim vermezse sistem varsayılan isim atamalı.

Karar (tüm planı bağlayan kural): editör yamalarının yüzeyi büyütülmez. Yükseltme kolaylığı
(`docs/runbooks/upstream-updates.md`) bu işlerin faydasından önemli. Bu yüzden yama gerektiren
maddeler (2, 4) "ucuzsa yap, değilse bırak" olarak işaretli.

Önerilen sıra: 3 → 9 → 1 → 8 → 6 → 2 → 4 → 7. Madde 5 yapılmaz (aşağıda gerekçe).

## Madde 3: Sunucu tarafı `sort=updated` (en değerli, yama yok)

Keşif: dört modül de aynı sorguyu kullanıyor, `ORDER BY id DESC`, imleç yalnızca `id`:
`modules/{documents,pdfs}/core/src/infrastructure/repository.rs:150-185`,
`modules/{spreadsheets,presentations}/core/src/infrastructure/repository.rs:192-235`.
Port ve servis: `application/ports.rs` (`Page.next_cursor`), `application/service.rs`
(`DEFAULT_LIMIT=25`, `MAX_LIMIT=100`). API: `api/routes.rs` `ListQuery`.

Yaklaşım:
- Yeni sorgu parametresi `sort=updated` (yoksa eski id sırası, geriye uyumlu). Sıra
  `updated_at DESC, id DESC`, koşul `(updated_at, id) < ($cursor_updated, $cursor_id)`.
- İmleç `sort=updated` için opak: `base64url("<rfc3339>|<uuid>")`. `ListQuery.cursor` bu modda `String`,
  `ListResponse.next_cursor` formatsız `string`. Geçersiz imleç `422 validation_failed`.
  Tek bir ortak ayrıştırıcı `crates/platform` içine (`platform::pagination`), dört modül kullanır
  (şu an dört kopya sorgu var; sorgu kopyası kalır, imleç kodu ortaklaşır).
- Her modülde yeni migration: kısmi indeks `(owner_id, updated_at DESC, id DESC) WHERE deleted_at IS NULL`
  (`<ts>_<modül>_updated_index.sql`, geri alma planı yorumda). `EXPLAIN` çıktısı PR açıklamasına.
- sqlx offline önbelleği: yeni migration + yeni sorgular için manuel bootstrap
  (dev DB'ye SQL uygula → `just db-prepare` → nesneyi DROP → `xtask migrate`), `.sqlx/` yenilenir.
- Not: `updated_at` `PackageSaved` ile ilerliyor (`platform::touch`), yani "son düzenleme" sırası
  doğru. Düzenleme sayfalama sırasında bir satırı taşıyabilir; istemci id ile tekilleştirir.

Web (`apps/web/src`):
- `entities/{document,spreadsheet,presentation,pdf}/api/queries.ts`: `sort: 'updated'` gönder,
  sorgu anahtarına ekle.
- `widgets/recent-files/model/use-all-files.ts`: dört listeyi birleştirirken yalnızca,
  hâlâ sayfası olan listelerin en eski `updated_at` değerinden yeni satırları göster
  (aksi halde sonraki sayfa gelince sıra zıplar); `hasMore` ve `loadMore` korunur; id ile tekilleştir.

Testler:
- Her modülün `tests/*_api.rs` içinde `crud_pagination_and_isolation` benzeri: eski dosya düzenlenince
  `sort=updated` ilk sayfada gelir; imleç ile ikinci sayfa; geçersiz imleç 422; varsayılan sıra değişmedi.
  (spreadsheets ve presentations'ta şu an hiç imleç testi yok, eklenir.)
- Vitest: `use-all-files` için sınır mantığı birim testi.
- `just gen-api` (OpenAPI + TS client), `just gen-api-check`.

## Madde 9: Kaydet != versiyon (isimli, elle oluşturulan versiyonlar)

Kararlar (kullanıcıyla netleşti): "son versiyon" = çalışma kopyası. Kaydetmeler yalnızca çalışma kopyasını
günceller, dosyayı herkes hep onunla açar. Versiyonlar yalnızca elle (ya da geri yüklemeden önce otomatik
güvenlik kopyası olarak) oluşur; günlük otomatik versiyon yok.

Bugünkü durum: her kayıt `storage.package_history` satırı yazıyor ve arayüz bunu "sürüm" diye listeliyor
(`modules/storage/core/src/infrastructure/repository.rs`, `api/routes.rs` `/versions`). Bu tablo aslında
birleştirmenin taban dosyası için gerekli iç kayıt günlüğü; kullanıcıya görünmemeli.

Veri modeli:
- `package_history` kalır, ama yalnızca iç günlük: birleştirme tabanı. Arayüz ve `/versions` uçları ondan
  okumayı bırakır. Saklama sayısı Madde 6'daki ayarla (varsayılan 50) sınırlanır; kullanıcıya gösterilmez.
- Yeni tablo `storage.package_versions` (yeni migration, geri alma planı yorumda):
  `id uuid pk (v7)`, `resource_id uuid`, `number int` (kaynak başına 1,2,3…; `UNIQUE (resource_id, number)`),
  `name text NULL` (boş = varsayılan isim), `kind text CHECK (kind IN ('manual','before_restore'))`,
  `bytes bytea`, `file_name text`, `size bigint`, `content_hash bytea` (sha256, `sha2` zaten workspace
  bağımlılığı), `created_by uuid`, `created_at timestamptz`, `package_version bigint` (hangi kayıttan alındı).
  Kaynak başına üst sınır 200 versiyon (aşılırsa `422 version_limit`, kullanıcı eskiyi siler).
- Varsayılan isim sunucuda saklanmaz (`name NULL`); istemci i18n ile üretir:
  `Version {number} · {tarih saat}` / `Versiyon {number} · …`, `before_restore` için
  `Before restore` / `Geri yüklemeden önce`. Böylece "sistem varsayılan isim verir" kuralı çalışır ve
  CLAUDE.md'nin "kullanıcıya görünen sabit metin yok" kuralı bozulmaz.

Uçlar (`/api/v1/packages/{id}/versions`, hepsi `utoipa` ile belgeli, rol kontrolü açık):
- `GET` liste (okuma yetkisi): `{ can_manage, has_unversioned_changes, versions:[{number,name,kind,size,
  created_by,created_by_name,created_at}] }`. `has_unversioned_changes` = çalışma kopyasının sha256'sı en yeni
  versiyonunkinden farklı mı.
- `POST` `{ name? }` (yazma yetkisi): çalışma kopyasının anlık görüntüsünü alır; ad boşsa `NULL`; ad kırpılır,
  en çok 80 karakter. Çalışma kopyası en yeni versiyonla aynıysa `409 nothing_to_version`.
- `PATCH /{number}` `{ name }` (yazma): yeniden adlandır; boş ad varsayılana döner.
- `DELETE /{number}` (yazma): sil.
- `GET /{number}` (okuma): indir/önizle (mevcut `readPackageVersion` buraya bağlanır).
- `POST /{number}/restore` (yazma): bu versiyonu "son" yapar = çalışma kopyasına yazar (`put`, böylece
  `PackageSaved` ile diğer açık pencereler yenilenir). Yazmadan önce çalışma kopyası en yeni versiyondan
  farklıysa otomatik `before_restore` versiyonu oluşturulur (veri kaybı olmaz).
- Eski `/versions/{n}/restore` ve iç günlük uçları kaldırılır (henüz yayında değil, geriye uyum gerekmiyor).

Kod yerleşimi: `storage-contracts` değişmez (birleştirme portu aynı). `storage/core`: domain'e
`VersionName` değer nesnesi (`parse, don't validate`), application'a `VersionRepository` portu ve servis
metotları, infrastructure'a sqlx deposu, api'ye uçlar. Çalışma kopyası ile versiyon kopyaları aynı
transaction'da yazılır (`restore`). Olay: yeni olay gerekmez (`PackageSaved` yeterli).

Web (`apps/web/src`):
- `entities/package/api/package.ts`: `listPackageVersions` yeni şekle; `createVersion`, `renameVersion`,
  `deleteVersion`, `restoreVersion`, `readPackageVersion(number)`; `keys.ts` aynı kalır.
- `features/version-history`: dialog "Versiyon oluştur" alanı (isteğe bağlı ad) + düğme, üstte "Çalışma
  kopyası · kaydedilmemiş versiyon değişiklikleri var" satırı (`has_unversioned_changes`), her satırda
  yeniden adlandır (yerinde düzenleme), sil (onaylı), indir, "Bunu son yap", Docs/Sheets/Slides'ta "Görüntüle".
  200 satırı geçmemesi için `VersionRow.vue` ve `CreateVersionForm.vue` ayrılır. Hata iletileri mevcut
  `errorMessageKey` eşlemesine yeni kodlar (`version_limit`, `nothing_to_version`) eklenerek.
- Önizleme çubuğu (`widgets/embed-workspace`) aynı kalır; "Bunu son yap" yeni restore ucunu çağırır.
- i18n: `history.*` anahtarları en/tr (varsayılan isim şablonları, `beforeRestore`, `create`, `rename`,
  `delete`, `deleteConfirm`, `unversioned`, `limit`).
- Ctrl+S / otomatik kayıt davranışı değişmez (yalnızca çalışma kopyası); editör arayüzünde yama gerekmez.
  İsteğe bağlı: editörün alt çubuğundaki "History" düğmesinin yanına "Create version" kısayolu.

Testler:
- `storage_api.rs`: kayıtlar versiyon üretmez; `POST` varsayılan ve adlı; aynı içerikte ikinci `POST` 409;
  yeniden adlandır/sil; restore `before_restore` üretir ve yalnızca fark varsa; üst sınır; rol matrisi
  (viewer liste/indir yapar, oluşturamaz/silemez/geri yükleyemez; yabancı 404); `has_unversioned_changes`.
- Birleştirme testleri değişmez (iç günlük aynı).
- Vitest: dialog (oluştur, yeniden adlandır, varsayılan isim biçimi), oturum önizleme testi aynı.
- OpenAPI/TS client yeniden üretilir (`just gen-api`); `.sqlx` önbelleği manuel bootstrap ile (Madde 3'teki yöntem).

## Madde 1: Sheets ve Slides tarayıcı doğrulaması (kod yok, bulgu çıkarsa yama düzeltmesi)

Araç: scratchpad'deki CDP sürücüsü (`lib.py`, `t50` iki kullanıcı, `t62` API ile uzaktan kayıt). Test portları:
API 8081, vite 5174, Chrome 9333/9334 (kullanıcının 5173/8080 süreçlerine dokunulmaz).

Senaryolar (her biri Sheets ve Slides için):
1. Aç, düzenle, otomatik kayıt "Saved" gösteriyor.
2. Info > Title ile yeniden adlandır → dosya listesinde yeni ad (renamed olayı).
3. İkinci tarayıcı/API ile uzaktan kayıt: ilk pencere temiz kopyayı yeniler, Sheets'te seçili hücre/sayfa,
   Slides'ta aktif slayt korunur.
4. İki taraf farklı hücre/slayt düzenler: birleşme, "combined" bildirimi; aynı hücre/slayt: çakışma bandı.
5. Versiyonlar (Madde 9): oluştur (adlı/adsız), yeniden adlandır, sil, View ile eski versiyon (kaydedilmez), "Bunu son yap", izleyici rolünde yönetim düğmeleri yok.

Çıktı: bulunan hatalar `scripts/embed-patches/{gridcraft,deckcraft}.patch` içinde düzeltilir
(patch yenileme kuralı: klonun kökünden `git add -N . && git diff HEAD -- . ':!target' ':!Cargo.lock'`),
`scripts/build-embeds.sh <app>` sonrası Vite yeniden başlatılır.

## Madde 8: Yükseltme script'ini gerçekten denemek

Önce risksiz: `scripts/check-upstream.sh` ve her repo için `upgrade-upstream.sh <repo> --dry-run`.
Sonra bir kopyada gerçek çalıştırma (çalışma ağacını bozmamak için): repoyu `target/` dışına
`rsync` ile kopyala (`target/`, `node_modules/` hariç), kopyada `deckcraft` (en küçük yama) ile başla,
sonra `wordcraft`. Gözlenecekler: 3-way rebase yolu, geri alma (rollback) yolu (bilerek bozuk sha ile),
NOTICE güncellemesi, adaptör testleri, `trunk build`. pdfcraft: `craft-libs` yayınlanmamış olabilir,
başarısızlıkta geri alma doğrulanır. Bulunan script hataları `scripts/upgrade-upstream.sh` içinde düzeltilir;
runbook'un hata tablosu gerçek çıktıya göre güncellenir.

## Madde 6: İç kayıt günlüğü saklama ayarı

Madde 9'dan sonra bu sayı kullanıcıya görünen sürüm sayısı değil, birleştirme taban dosyası günlüğünün
uzunluğu. `KEPT_VERSIONS = 50` sabiti `modules/storage/core/src/infrastructure/repository.rs` içinde.
Yapılacak: `Settings` (`apps/api`) içine `STORAGE_KEPT_VERSIONS` (varsayılan 50, 5..=1000 doğrulama),
`storage::module(...)` bir `Config` alır, repository'ye geçirilir. `.env.example` güncellenir.
Test: küçük değerle (3) budama (`storage_api.rs`). Günlük budanınca çok eski tabanla gelen kayıt birleşemez
(çakışma bandı); 50 bunun için yeterli, ayar yalnızca yer/bellek içindir.

## Madde 2: PDF (iki ayrı iş)

PDF birleştirmesi bilinçli olarak yok: PDF sayfa içeriği satır/hücre gibi bağımsız birimlere ayrışmıyor.
Çakışmada "Use their version / Keep mine" kalır; runbook ve STATUS'ta "tasarım gereği" diye yazılır.

a) Yeniden adlandırma: pdfcraft'ta belge adı dosya adı; özellik penceresi başlığı var mı kontrol edilecek
(`target/embeds/pdfcraft/crates/ui-egui/src/lib.rs`, `host::Bridge` `apps/pdfcraft-web/src/main.rs`).
Küçük bir `pdfcraft:renamed` mesajı ve `useRenameFile` bağlantısı (diğer üç editördeki `on_renamed` deseni).
Ucuz değilse (rename arayüzü yoksa) yapılmaz; kullanıcı listeden yeniden adlandırır.

b) Görünüm koruma: `last_view.rs` yalnızca masaüstünde ve dosya yoluyla çalışıyor (web'de yol yok).
Yaklaşım: hosted modda `pdfcraft:open` ikinci kez gelince, mevcut görünümü (sayfa, zoom, görüntü biçimi)
`LastView` yapısıyla "host:<ad>" anahtarına yaz, açılışta uygula. Mevcut yapı yeniden kullanılır, yeni model yok.
Doğrulama: uzaktan kayıttan sonra aynı sayfa/zoom.

## Madde 4: Docs imleç kayması (yalnızca ucuzsa)

Neden (kod okuma): konum `Pos {story, path, off}` bloğun indeksi ve bayt kaydırması; metne bağlı değil
(`crates/doc/src/lib.rs:330`). Başkası yukarıya paragraf eklerse indeks bir paragraf kayar. Ayrıca satır sonu
eşdeğerliği yok (`line_of`, `layout/src/para.rs:3163`), yenilemede imleç sonraki satır başına düşer.
Kaydırma piksel olarak geri konuyor (`canvas.rs:1209-1218`).

Yaklaşım (yama küçük kalacak şekilde): `drain_inbox` (`lib.rs:3146-3157`) imleci kaydederken blok metninin kısa
özetini de saklar; geri yüklerken aynı indeksteki metin farklıysa, ±5 blok içinde aynı metni arar. Satır sonu
eşdeğerliği için ayrı alan eklenmez. Bu yeterince küçük çıkmazsa (yaklaşık 30 satırdan fazlaysa) vazgeçilir ve
"bilinen sınır" olarak kalır.

## Madde 7: Docs paragraf içi birleştirme (isteğe bağlı, son)

Şimdi aynı paragraf iki tarafta değişirse çakışma. Yaklaşım: paragrafın düz metni ve çalıştırma
(run) biçimleri iki tarafta aynı kalıyorsa (yalnızca metin değişmişse), metni sözcük düzeyinde `merge3`
ile birleştir (`crates/merge3` zaten genel). Biçim değişmişse çakışma kalır. Yer:
`modules/documents/engine/src/merge.rs`, çakışan blok için `merge_paragraph(base, ours, theirs)` yedek yolu.
Testler: iki kişi aynı paragrafın farklı yerlerine yazar → birleşir; aynı sözcüğü değiştirir → çakışma;
`proptest` ile "birleşme hiçbir tarafın metnini kaybetmez". Risk: wordcraft `Paragraph` yapısı yükseltmede
değişebilir; adaptör testleri bunu yakalar.

## Madde 5: Yjs boş açılma, yapılmaz

Keşif: içe aktarılan docx varsayılan yolda `storeOriginalFile` ile paket depolamaya yazılıyor ve gömülü
wordcraft onu açıyor. Boşluk yalnızca `?legacy` Tiptap editöründe, bellekteki tek seferlik
`pending-import` aktarımı kaçırılınca oluşuyor. Legacy yol kullanıcıya açık değil. Önerilen: dokunma,
`docs/STATUS.md`'de "yalnızca `?legacy`" notu düş. (İstenirse ayrı iş: içe aktarmada ProseMirror
içeriğini sunucuda Yjs belgesine tohumlamak.)

## Dokümantasyon (her madde ile birlikte)

- `docs/STATUS.md` bilinen eksikler bölümü: bitenler silinir, kalanlar güncellenir.
- `docs/runbooks/upstream-updates.md`: PDF birleştirme yok notu, madde 8 sonrası hata tablosu.
- Yeni mimari karar yok (yeni modül/bağımlılık yok), ADR gerekmiyor.

## Doğrulama (uçtan uca)

Her madde sonunda ilgili crate'lerde `cargo clippy -D warnings` ve `cargo test -p <crate>`; web için
`pnpm exec vue-tsc --noEmit`, `eslint src`, `vitest run`. Plan sonunda tek seferde `just check`
(uzun derleme; kullanıcı onayıyla, gece/arka planda). API değişen her maddeden sonra `just gen-api`.
Madde 1/2/4 için tarayıcı senaryoları yukarıda; sonuçlar ekran görüntüsü yerine betik çıktısı ve
seçici sorgularıyla raporlanır. Commit yok (git geçmişi kararı kullanıcıda).
