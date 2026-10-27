# CLAUDE.md — Online Ofis Paketi (Docs · Sheets · Slides)

Bu dosya, bu repoda çalışan her AI ajanının ve geliştiricinin uyması gereken **bağlayıcı** kurallardır. Çelişki durumunda bu dosya kazanır; bir kuralı değiştirmek gerekiyorsa önce bu dosya (ve gerekiyorsa bir ADR) güncellenir, sonra kod.

> Dil kuralı: Kod, yorumlar, commit mesajları, PR başlıkları, ADR'ler ve API alan adları **İngilizce**. Kullanıcıya dönük metinler i18n dosyalarında (varsayılan `tr`, ikincil `en`).

---

## 1. Ürün Tanımı

Google Workspace benzeri, tarayıcıda çalışan, gerçek zamanlı ortak çalışmayı destekleyen bir ofis paketi:

| Ürün | Kısa ad | Açıklama |
|---|---|---|
| Doküman editörü | **Docs** | Zengin metin, başlıklar, tablolar, görseller, yorumlar, öneri modu |
| Hesap tablosu | **Sheets** | Hücre ızgarası, formüller, biçimlendirme, grafikler, filtre/sıralama |
| Sunum | **Slides** | Slayt düzenleyici, şablonlar, geçişler, sunum modu |
| Ortak katman | **Workspace** | Kimlik, klasörler/Drive, paylaşım, izinler, yorumlar, bildirimler, arama |

Ortak gereksinimler: gerçek zamanlı çoklu kullanıcı düzenleme (CRDT), sürüm geçmişi, çevrimdışı dayanıklılık, import/export (`.docx`, `.xlsx`, `.pptx`, `.pdf`, `.csv`), erişilebilirlik (WCAG 2.2 AA), i18n.

---

## 2. Teknoloji Yığını (Hedef)

> **Versiyon politikası:** Her zaman **güncel stable** sürümler kullanılır. Projeyi başlatırken ve bağımlılık eklerken güncel sürümü doğrula (`cargo search`, `cargo info`, `pnpm view <pkg> version`, resmi release notları). Versiyonlar `rust-toolchain.toml`, `package.json` (`engines`, `packageManager`) ve `Cargo.toml` (`[workspace.dependencies]`) içinde **sabitlenir**. Hafızadan versiyon yazma.

### Backend
- **Rust** (stable, **edition 2024**), `rust-toolchain.toml` ile pin; MSRV `workspace.package.rust-version` içinde.
- **Web:** `axum` + `tokio` + `tower` / `tower-http` (CORS, trace, compression, timeout, request-id).
- **DB:** PostgreSQL (güncel major) + `sqlx` (compile-time checked queries, `sqlx migrate`). ORM kullanılmaz.
- **Gerçek zamanlı:** WebSocket (axum) + **`yrs`** (Yjs'in Rust portu, CRDT) — Docs ve Slides için Yjs belgeleri; Sheets için de başlangıçta Yrs tabanlı hücre modeli (ADR ile karar verilir).
- **Cache / pub-sub / presence:** Redis (veya Valkey).
- **Nesne depolama:** S3 uyumlu (MinIO lokalde). `object_store` crate'i ile soyutlanır.
- **Serileştirme / doğrulama:** `serde`, `validator` (veya elle value object doğrulama).
- **Hata:** kütüphane/modül katmanında `thiserror`, uygulama sınırında `anyhow` yalnızca `apps/api` içinde.
- **Gözlemlenebilirlik:** `tracing` + `tracing-subscriber` + OpenTelemetry (OTLP), `metrics`/Prometheus.
- **OpenAPI:** `utoipa` (+ `utoipa-axum`); şema tek kaynaktır, frontend client'ı buradan üretilir.
- **Kimlik:** Argon2id parola hash, JWT access + rotating refresh token (HttpOnly cookie), OIDC/OAuth2 (Google, Microsoft) `openidconnect`.
- **ID:** UUID v7 (`uuid` crate), tüm PK'ler.
- **Zaman:** `time` veya `chrono` (biri seçilir, tutarlı kullanılır; varsayılan `time`). Zaman her zaman UTC saklanır.
- **Test:** `cargo nextest`, `testcontainers` (Postgres/Redis), `insta` (snapshot), `proptest` (property-based, özellikle formül motoru ve CRDT), `mockall` yalnızca port mock'ları için.
- **Kalite araçları:** `rustfmt`, `clippy`, `cargo-deny`, `cargo-audit`, `cargo-machete` (kullanılmayan dep), `cargo-llvm-cov`, `typos`.

### Frontend
- **Vue 3** (güncel stable, `<script setup lang="ts">`, Composition API) + **TypeScript** (strict) + **Vite**.
- **Paket yöneticisi:** `pnpm` (workspace). Node LTS, `.nvmrc` + `engines`.
- **State:** `pinia` (global/UI state), sunucu durumu için **TanStack Query (Vue Query)**.
- **Routing:** `vue-router` (typed routes).
- **UI:** Tailwind CSS (güncel major) + headless bileşenler (Reka UI / Radix Vue) + `lucide-vue-next` ikonları. Tasarım token'ları CSS değişkenleriyle (açık/koyu tema).
- **Editörler:** Docs → **Tiptap** (ProseMirror) + `y-prosemirror`/Yjs; Sheets → Canvas tabanlı sanal ızgara (kendi render katmanımız veya ADR ile onaylı bir kütüphane); Slides → SVG/Canvas sahne + Yjs.
- **Gerçek zamanlı istemci:** `yjs`, `y-protocols`, özel WebSocket provider (backend ile aynı protokol).
- **API client:** `openapi-typescript` + `openapi-fetch` (elle yazılmış fetch yok).
- **i18n:** `vue-i18n`. Kullanıcıya görünen hiçbir string kodda sabit yazılmaz.
- **Form/doğrulama:** `vee-validate` + `zod` (veya `valibot`).
- **Test:** `vitest` + `@vue/test-utils` + Testing Library, **Playwright** (E2E + görsel regresyon), `msw` (API mock), Storybook (UI kataloğu, opsiyonel ama önerilir).
- **Kalite:** ESLint (flat config), Prettier, `vue-tsc`, `stylelint`, `knip` (ölü kod), `lefthook`/`husky` + `lint-staged`, `commitlint`.

### Altyapı / DevOps
- Docker (multi-stage, distroless/minimal runtime), `docker compose` lokal geliştirme (postgres, redis, minio, mailpit, otel-collector).
- CI: GitHub Actions (cache'li). Release: semantic versioning, conventional commits ile changelog.
- Konfigürasyon: 12-factor, ortam değişkenleri; sırlar repoya girmez.
- Görev çalıştırıcı: `just` (tek giriş noktası, bkz. §12).

---

## 3. Repo Yapısı (Monorepo)

```
.
├── CLAUDE.md
├── justfile
├── rust-toolchain.toml
├── Cargo.toml                    # workspace root, [workspace.dependencies], [workspace.lints]
├── deny.toml                     # cargo-deny
├── clippy.toml / rustfmt.toml
├── docker/                       # Dockerfile'lar, compose dosyaları
├── docs/
│   ├── adr/                      # Architecture Decision Records (0001-....md)
│   ├── architecture/             # C4 diyagramları, modül haritası
│   └── runbooks/
├── vendor/                       # Değiştirilmemiş üçüncü taraf motor crate'leri (ADR 0001); kök workspace'ten hariç
│   └── <repo>/                   # kendi iç içe workspace'i + VENDOR.toml (kaynak SHA) + LICENSE/NOTICE
├── apps/
│   ├── api/                      # BINARY: composition root (axum server)
│   ├── worker/                   # BINARY: outbox relay, arka plan işleri, export/import
│   └── web/                      # Vue SPA
├── crates/                       # Paylaşılan, modül-dışı Rust crate'leri
│   ├── shared-kernel/            # Id tipleri, Clock, Error trait'leri, Money vb. (saf, I/O yok)
│   ├── platform/                 # Altyapı yardımcıları: db pool, event bus, outbox, config, telemetry, http extractors
│   └── testing/                  # Test yardımcıları (fixtures, testcontainers)
├── modules/                      # İş modülleri (bounded context'ler)
│   ├── identity/
│   │   ├── contracts/            # public API: portlar, olaylar, DTO'lar  (crate: identity-contracts)
│   │   └── core/                 # implementasyon (crate: identity)
│   ├── workspace/                # klasörler, dosya meta, paylaşım, izinler
│   ├── documents/                # Docs
│   ├── spreadsheets/             # Sheets (+ formula engine)
│   ├── presentations/            # Slides
│   ├── collaboration/            # realtime sync, presence, yorumlar
│   ├── storage/                  # dosya/asset yükleme, nesne depolama
│   ├── notifications/
│   └── search/
└── packages/                     # Frontend paylaşılan paketler
    ├── api-client/               # OpenAPI'den üretilen TS client (commit'lenir veya CI'da üretilir)
    ├── ui/                       # tasarım sistemi bileşenleri
    └── config/                   # eslint, prettier, tsconfig, tailwind presetleri
```

Her iş modülü **iki crate**'ten oluşur:

- `<modül>-contracts`: Diğer modüllerin görebileceği **tek** yüzey. İçerik: port trait'leri (`async_trait`/native async fn in trait), entegrasyon olayları (event struct'ları), DTO/ID tipleri, hata enum'ları. **Hiçbir iş mantığı, DB, HTTP yok.**
- `<modül>` (core): Domain + application + infrastructure + HTTP adapter implementasyonu. Dışarıya sadece `pub fn module(...)`/`Module` kurucusu ve contracts'taki portların implementasyonunu açar.

---

## 4. Mimari İlkeler (Clean Architecture + Hexagonal + Modular Monolith)

### 4.1 Katmanlar (modül içi)

```
modules/<name>/core/src/
├── domain/            # Entity, Aggregate, Value Object, Domain Event, Domain Service, Repository TRAIT'leri
├── application/       # Use case / command & query handler'lar, port trait'leri (outbound), DTO mapping
├── infrastructure/    # Port implementasyonları: sqlx repository, redis, s3, dış servis client'ları
├── api/               # Inbound adapter: axum router, request/response modelleri, extractor'lar
└── lib.rs             # Modülün public yüzü (çok dar tutulur)
```

**Bağımlılık yönü (kesin):** `api → application → domain` ve `infrastructure → application/domain`. `domain` hiçbir şeye bağımlı değildir (sadece `shared-kernel`, `std`, saf yardımcı crate'ler: `serde` opsiyonel, `thiserror`, `uuid`, `time`). **Domain içinde `sqlx`, `axum`, `tokio` (I/O), `redis`, `reqwest` yasaktır.**

### 4.2 Modüller Arası Kurallar (EN KRİTİK BÖLÜM)

1. Bir modül başka bir modülün **yalnızca `-contracts` crate'ine** bağımlı olabilir. Başka modülün `core` crate'ine, tablolarına, SQL'ine, iç tiplerine erişim **yasak**.
2. Modüller arası iletişim iki yoldan biridir:
   - **Asenkron (varsayılan): Entegrasyon olayları.** Yayıncı modül kendi contracts'ında `pub struct DocumentCreated { ... }` tanımlar; abone modül bu contracts'a bağımlı olur ve handler kaydeder. Yayıncı aboneyi **bilmez**.
   - **Senkron (yalnızca gerekirse): Port.** İhtiyaç duyan modül kendi **application** katmanında bir port tanımlar (örn. `trait PermissionChecker`) **ya da** sağlayıcı modül contracts'ında bir trait sunar (örn. `identity_contracts::UserDirectory`). Composition root'ta (`apps/api`) implementasyon enjekte edilir.
3. **Veritabanı:** Her modül kendi Postgres **şemasına** sahiptir (`identity`, `documents`, ...). Modüller arası **foreign key yok**, cross-schema JOIN yok. Başka modülün verisi gerekirse: port ile sor veya olaylarla kendi **read-model/yansımasını** tut. Referanslar sadece ID (typed `UserId`, `DocumentId`) olarak saklanır.
4. **Transaction sınırı = modül.** Birden fazla modülü kapsayan atomik transaction yok. Tutarlılık **eventual consistency** (outbox) ile sağlanır; gerekirse saga/süreç yöneticisi.
5. **Outbox pattern zorunlu:** Olaylar iş verisiyle **aynı transaction'da** `<modül>.outbox` tablosuna yazılır; `apps/worker` (veya in-process relay) bunları yayınlar. İlk aşamada in-process event bus (`platform::event_bus`), ileride NATS/Kafka'ya geçişe hazır arayüz (`EventPublisher`, `EventSubscriber` trait'leri). Tüketiciler **idempotent** olmalı (event id ile dedupe tablosu).
6. **Composition Root:** Tüm bağımlılık bağlama (DI) yalnızca `apps/api` ve `apps/worker` içinde yapılır. Modüller birbirini import ederek bağlanmaz. DI container kullanılmaz; elle `Arc<dyn Trait>` / generic ile kurucu enjeksiyonu.
7. **Mimari testler zorunlu:** `crates/testing` veya `xtask` içinde `cargo metadata` okuyup şunları doğrulayan bir test CI'da koşar:
   - `modules/*/core` crate'leri birbirine bağımlı değil.
   - Modüller yalnızca diğer modüllerin `contracts` crate'ine bağımlı.
   - `*-contracts` crate'lerinde `sqlx`/`axum`/`tokio` (I/O features) yok.
   - `domain` modülünde yasak crate'ler yok.
   - `vendor/*` crate'lerine yalnızca `modules/<m>/engine` adaptör crate'leri bağımlı olabilir; vendor crate'ler workspace crate'lerine bağımlı olamaz (ADR 0001). Vendor kodu asla düzenlenmez; `vendor/` workspace lint'lerinden muaftır.
   Frontend için `eslint-plugin-boundaries` ile eşdeğer kurallar (bkz. §7).
8. Contracts değişiklikleri **public API değişikliğidir**: geriye dönük uyumlu tut (olaylara alan ekle, kaldırma); kırıcı değişiklikte olay versiyonla (`DocumentCreatedV2`).

### 4.3 Domain Modelleme (DDD-lite)
- Aggregate kökleri invariants'ı korur; dış dünyaya sadece kök üzerinden erişilir.
- Value Object'ler **parse, don't validate**: `Email::parse(..)?`, `DocumentTitle::new(..)?`. Ham `String`/`Uuid` ile iş mantığı yazma; **newtype** kullan (`DocumentId(Uuid)`).
- Domain Event'ler aggregate içinde toplanır, application katmanı transaction'ı commit ederken outbox'a yazar.
- Repository trait'leri domain/application'da, implementasyonu infrastructure'da. Repository aggregate döner, DTO değil.
- CQRS-lite: Command handler'lar aggregate üzerinde çalışır; Query handler'lar gerekirse doğrudan SQL ile read-model/DTO döner (domain'i dolaşmadan).
- Formül motoru (Sheets) **saf bir crate**dir (`modules/spreadsheets/formula-engine`): I/O yok, deterministik, `proptest` ile sıkı test; WASM'a derlenebilir olmalı (frontend'de aynı motoru kullanma opsiyonu).

### 4.4 Gerçek Zamanlı Mimari
- Belge durumu = Yrs `Doc`. WebSocket oturumu: auth → izin kontrolü (port: `PermissionChecker`) → sync protokolü (state vector / update) → awareness (presence/cursor).
- Güncellemeler append-only `document_updates` tablosuna yazılır; periyodik **snapshot + compaction** (worker).
- Çoklu instance: Redis pub/sub ile update fan-out. Sticky session **zorunlu değil**, tasarım buna bağlı olmamalı.
- İzin değişikliği/erişim iptali açık WS oturumlarını **düşürmeli** (olay: `PermissionRevoked`).

---

## 5. Backend Kodlama Standartları

### 5.1 Genel
- `#![forbid(unsafe_code)]` her crate'te (istisna: ADR + ayrı izole crate).
- `unwrap()` / `expect()` / `panic!` / `todo!` / `unimplemented!` **üretim kodunda yasak** (testlerde serbest). Clippy ile enforce edilir. `expect` yalnızca "programlama hatası, imkânsız durum" için ve açıklayıcı mesajla, bir `// INVARIANT:` yorumuyla.
- Hata tasarımı: Her modülün kendi `thiserror` enum'ı. HTTP eşlemesi **sadece** `api` katmanında (`IntoResponse` / `ProblemDetails` RFC 9457). Domain hataları HTTP bilmez.
- Async: Bloklayıcı iş (CPU yoğun: formül hesaplama, docx/xlsx dönüşümü, hashing) `spawn_blocking` veya worker. Lock'u `.await` boyunca tutma. `tokio::select!` iptal güvenliğine dikkat.
- Native `async fn` in traits kullan; `dyn` gereken yerde `async-trait` veya `trait-variant`. Karar tutarlı olsun.
- Logging: `tracing` yapılandırılmış alanlarla (`user_id`, `doc_id`, `request_id`). **PII ve belge içeriği loglanmaz.** `println!`/`dbg!` yasak.
- Config: `serde` ile tipli `Settings` struct; başlangıçta doğrula, fail-fast. Sırlar `secrecy::SecretString`.
- Zaman/rastgelelik testlenebilir olmalı: `Clock` ve `IdGenerator` port'ları (shared-kernel).
- Public API'ler `///` doc yorumuyla; `#![warn(missing_docs)]` contracts crate'lerinde.

### 5.2 Lint Konfigürasyonu
`Cargo.toml` (workspace) içinde:

```toml
[workspace.lints.rust]
unsafe_code = "forbid"
missing_debug_implementations = "warn"
unreachable_pub = "warn"

[workspace.lints.clippy]
all = { level = "deny", priority = -1 }
pedantic = { level = "warn", priority = -1 }
nursery = { level = "warn", priority = -1 }
unwrap_used = "deny"
expect_used = "warn"
panic = "deny"
todo = "deny"
dbg_macro = "deny"
print_stdout = "deny"
print_stderr = "deny"
module_name_repetitions = "allow"
```

- Her crate `[lints] workspace = true`.
- `rustfmt.toml`: `edition = "2024"`, `max_width = 100`, `imports_granularity = "Module"`, `group_imports = "StdExternalCrate"` (nightly-only seçenekler kullanılmaz; stable'da çalışanlarla sınırlı kal).
- `deny.toml`: lisans allowlist (MIT/Apache-2.0/BSD/ISC/Unicode), bilinen zafiyet = deny, yinelenen sürüm = warn, kaynak = yalnızca crates.io.
- CI'da: `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo deny check`, `cargo audit`, `cargo machete`, `cargo nextest run`.

### 5.3 Veritabanı
- Migration'lar **ileri-only**, modül başına dizin: `modules/<name>/core/migrations/`. Her migration idempotent değil ama atomik; geri alma planı yorumda.
- Zero-downtime kuralı: expand → migrate → contract (kolon silme/yeniden adlandırma iki aşamalı).
- `sqlx::query!`/`query_as!` kullan (compile-time check), `SQLX_OFFLINE` için `.sqlx/` cache commit'lenir (`cargo sqlx prepare --workspace` CI'da doğrulanır).
- Tüm sorgular parametrik; string birleştirme ile SQL yasak.
- Her tabloda `id uuid pk`, `created_at`, `updated_at`; gerektiğinde `version` (optimistic locking) ve `deleted_at` (soft delete).
- İndeks ihtiyacı sorguyla birlikte PR'da gerekçelendirilir (`EXPLAIN` çıktısı).

### 5.4 HTTP API
- REST + JSON, kaynak odaklı URL'ler: `/api/v1/documents/{id}`. Versiyon URL'de.
- Sayfalama: **cursor-based**. Filtre/sıralama whitelist'li.
- Hata formatı: RFC 9457 `application/problem+json`, makine-okunur `code` alanı.
- İdempotency: Yan etkili POST'larda `Idempotency-Key` desteği (özellikle import/export, ödeme).
- Her endpoint `utoipa` ile belgelenir; CI'da üretilen `openapi.json` ile commit'li sürüm farkı varsa build kırılır (drift kontrolü).
- Rate limiting (`tower-governor` benzeri), body size limit, request timeout varsayılan olarak açık.
- Authz: Her handler'da **açık** yetki kontrolü (varsayılan: reddet). Yetki kararı `PermissionChecker` portu ile; kaynak sahipliği/paylaşım kuralları `workspace` modülünde.

### 5.5 Test Stratejisi (Backend)
- **Test piramidi:** Domain unit testleri (çok, hızlı, I/O yok) → Application testleri (in-memory fake portlarla) → Infrastructure/integration testleri (testcontainers ile gerçek Postgres/Redis) → API/contract testleri (tam axum app, gerçek DB) → birkaç E2E.
- Port'lar için **in-memory fake** implementasyonlar yaz (mock yerine tercih); `mockall` yalnızca etkileşim doğrulaması gerektiğinde.
- Formül motoru ve CRDT mantığı için `proptest` zorunlu.
- Snapshot testleri `insta` (OpenAPI, export çıktıları).
- Kapsam hedefi: domain ≥ %90, application ≥ %80 (`cargo llvm-cov`). Kapsam tek başına hedef değildir; davranışı test et.
- Testler birbirinden bağımsız, paralel çalışabilir; her test kendi şemasını/DB'sini izole eder.

---

## 6. Frontend Mimarisi (Vue)

### 6.1 Yapı — Feature-Sliced / Modüler

```
apps/web/src/
├── app/                 # bootstrap: main.ts, router, providers, global stiller, i18n kurulumu
├── pages/               # route seviyesinde ince bileşenler (sadece kompozisyon)
├── widgets/             # birden fazla feature'ı birleştiren büyük UI blokları (örn. EditorShell)
├── features/            # kullanıcı senaryoları (share-document, comment-thread, formula-bar ...)
│   └── <feature>/
│       ├── ui/          # Vue bileşenleri
│       ├── model/       # pinia store, composable'lar, state machine
│       ├── api/         # TanStack Query hook'ları (api-client üzerinden)
│       └── index.ts     # public API (sadece buradan import edilir)
├── entities/            # iş varlıkları (document, spreadsheet, presentation, user) — tipler, küçük UI
├── editors/             # editör motorları (docs/, sheets/, slides/) — framework-bağımsız çekirdek + Vue adaptörü
└── shared/              # ui-kit, lib, config, i18n, api (http client kurulumu), utils
```

**Import kuralı (üstten alta tek yön):** `app → pages → widgets → features → entities → shared`. Alt katman üst katmanı import edemez; aynı katmandaki feature'lar birbirini import edemez (ortak ihtiyaç → `entities` veya `shared`'a taşı ya da üst katmanda kompoze et). Her slice yalnızca `index.ts` public API'si üzerinden tüketilir. `eslint-plugin-boundaries` (veya `eslint-plugin-import` restricted paths) ile **enforce edilir**.

### 6.2 Vue Standartları
- Sadece `<script setup lang="ts">`; Options API yok. SFC sırası: `<script>`, `<template>`, `<style scoped>`.
- Props/emit tipli (`defineProps<{...}>()`, `defineEmits<{...}>()`); `any` yasak; `as` cast'leri gerekçelendirilmeden yasak.
- Bileşenler küçük ve tek sorumlu; 200 satırı geçen SFC bölünür. İş mantığı **composable**'lara veya `model/` altına taşınır; bileşen sunumdur.
- Reaktivite: `ref`/`computed` öncelikli, `reactive` dikkatli. Büyük editör durumu (hücre matrisi, ProseMirror doc, Yjs doc) için `shallowRef`/`markRaw` kullan — reaktivite maliyetinden kaçın.
- Sunucu durumu **sadece** TanStack Query'de; Pinia'ya sunucu verisini kopyalama. Pinia yalnızca UI/oturum/istemci durumu için.
- Yan etkiler (`watch`, `onMounted`) temizlenir (`onScopeDispose`, `onBeforeUnmount`); WS/Yjs provider yaşam döngüsü composable içinde yönetilir.
- Performans: Sanal kaydırma (Sheets ızgarası, büyük listeler), `v-memo`/`defineAsyncComponent` ile route/editör bazlı code-splitting, ağır işler (formül hesaplama, import parse) **Web Worker**'da.
- Erişilebilirlik: Semantik HTML, klavye ile tam kullanım (editörlerde de), `aria-*`, odak yönetimi, renk kontrastı AA. `eslint-plugin-vuejs-accessibility` aktif.
- Stil: Tailwind utility + tasarım token'ları (CSS değişkenleri). Rastgele hex/px yazma; token kullan. Koyu tema birinci sınıf.
- i18n: `t('key')` — sabit metin yok. Tarih/sayı için `Intl`. RTL'e hazır (`logical properties`).

### 6.3 API Kullanımı
- `packages/api-client` OpenAPI'den üretilir (`pnpm gen:api`); elle düzenlenmez. Backend şeması değişince client yeniden üretilir ve CI drift kontrolü yapar.
- Sorgu anahtarları feature başına `keys.ts` içinde merkezi; mutation sonrası invalidate/optimistic update açıkça yazılır.
- Hata eşleme tek yerde (`shared/api/errors.ts`): `problem+json` → UI hata modeli.
- Gizli bilgi/token localStorage'a yazılmaz; refresh HttpOnly cookie ile.

### 6.4 Lint / Format Konfigürasyonu (Frontend)
- **ESLint flat config** (`eslint.config.ts`): `@eslint/js`, `typescript-eslint` (**strict-type-checked + stylistic-type-checked**), `eslint-plugin-vue` (`flat/recommended` + `vue3-strongly-recommended`), `eslint-plugin-vuejs-accessibility`, `eslint-plugin-import-x` (veya `eslint-plugin-boundaries`), `eslint-plugin-unicorn` (seçili kurallar), `eslint-plugin-vitest`, `eslint-plugin-vue-i18n` (sabit metin yakalama), `eslint-plugin-tailwindcss` (veya Prettier tailwind plugin).
- Önemli kurallar: `@typescript-eslint/no-explicit-any: error`, `no-floating-promises: error`, `no-misused-promises: error`, `consistent-type-imports: error`, `vue/block-order`, `vue/define-macros-order`, `vue/no-v-html: error`, `no-console: error` (warn/error hariç), `no-restricted-imports` (katman ihlalleri).
- **Prettier**: `printWidth: 100`, `singleQuote: true`, `semi: false`, `trailingComma: 'all'`, `prettier-plugin-tailwindcss`. ESLint ile çakışma `eslint-config-prettier` ile giderilir; stil için ESLint kural değil Prettier kullanılır.
- **TypeScript**: `strict: true`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`, `noImplicitOverride`, `verbatimModuleSyntax`, `isolatedModules`. Tip kontrol: `vue-tsc --noEmit`.
- **stylelint** (`stylelint-config-standard` + tailwind uyumu), **knip** (kullanılmayan export/dep), `.editorconfig`.
- **Git hook** (`lefthook`): pre-commit → `lint-staged` (eslint --fix, prettier --write, rustfmt), commit-msg → `commitlint` (Conventional Commits), pre-push → hızlı testler.

### 6.5 Test Stratejisi (Frontend)
- Birim: Vitest (composable, util, store, editör çekirdeği, formül parser köprüsü).
- Bileşen: `@vue/test-utils` + Testing Library — **kullanıcı davranışı** test edilir (rol/etiket ile sorgu), implementasyon detayı değil.
- API: `msw` ile mock; gerçek şemaya karşı tip güvenli.
- E2E: Playwright (çoklu tarayıcı); kritik akışlar: giriş, belge oluştur/düzenle, **iki kullanıcı eşzamanlı düzenleme**, paylaşım, import/export. Görsel regresyon (editör render'ları) seçili ekranlarda.
- A11y: `axe` (Playwright + Vitest) kritik sayfalarda.

---

## 7. Güvenlik (Her PR için gözden geçir)

- OWASP ASVS seviyesi hedefle. Girdi **sınırda** doğrulanır (Rust: tipler + validator; TS: zod).
- XSS: Kullanıcı içeriği (belge, hücre, slayt) asla `v-html` ile basılmaz; zengin içerik sanitize edilmiş şemadan (ProseMirror schema) render edilir. CSP sıkı (nonce/strict-dynamic), `X-Content-Type-Options`, `Referrer-Policy`, `frame-ancestors`.
- CSRF: Cookie auth'ta SameSite=Lax/Strict + CSRF token (state-değiştiren isteklerde).
- Yükleme: MIME doğrulama (uzantıya güvenme), boyut limiti, virüs taraması hook'u, depolama anahtarları tahmin edilemez (UUID), imzalı URL'ler kısa ömürlü.
- Import (docx/xlsx/pptx) **güvenilmez girdi**: zip bomb, XML entity (XXE kapalı), makro/OLE çalıştırma yok, sandbox'lı worker, kaynak limitleri (CPU/bellek/süre).
- Formüller: Dış kaynak çağıran fonksiyonlar (IMPORTDATA vb.) allowlist + SSRF koruması. Formül motoru sonsuz döngü/derin özyinelemeye karşı limitli.
- Sırlar: repoya girmez; `.env.example` tutulur; loglarda maskelenir. `cargo deny`/`pnpm audit`/Dependabot (veya Renovate) açık.
- Multi-tenancy/yetki: Her sorguda sahiplik/izin filtresi; IDOR testleri yazılır. Paylaşım linkleri için ayrı, iptal edilebilir token'lar.
- Denetim günlüğü (audit log): oturum, paylaşım, izin değişikliği, silme, dışa aktarma olayları.

---

## 8. Gözlemlenebilirlik ve Operasyon

- Her istek `request_id` taşır (header → tracing span → log → hata yanıtı).
- Metrikler: HTTP gecikme/hata, WS bağlantı sayısı, update fan-out gecikmesi, outbox backlog, DB pool, formül hesaplama süresi.
- Sağlık uçları: `/healthz` (liveness), `/readyz` (DB/Redis hazır).
- Graceful shutdown: SIGTERM'de yeni bağlantı kabul etme, WS'leri düzgün kapat, outbox'ı boşalt.
- Feature flag altyapısı (basit, config tabanlı başla).

---

## 9. Git, PR ve Sürüm Süreci

- Trunk-based; kısa ömürlü branch'ler: `feat/…`, `fix/…`, `chore/…`, `refactor/…`.
- **Conventional Commits** (`feat(documents): add comment threads`). Scope = modül adı veya `web`/`api`/`infra`.
- PR küçük ve tek amaçlı (tercihen < 400 satır diff). PR şablonu: amaç, değişiklik, test, risk, ekran görüntüsü (UI), migration notu.
- Merge koşulları: tüm CI yeşil, en az 1 onay, mimari test yeşil, OpenAPI/client drift yok.
- Mimari etkisi olan her karar için `docs/adr/NNNN-baslik.md` (Bağlam, Karar, Alternatifler, Sonuçlar).

### Definition of Done
1. Çalışıyor ve **test edildi** (uygun piramit seviyesinde).
2. Lint, format, tip kontrolü, mimari testler yeşil.
3. Yeni/değişen public API için OpenAPI + contracts güncel; client yeniden üretildi.
4. Güvenlik gözden geçirmesi (§7) yapıldı; yeni girdi noktaları doğrulanıyor.
5. i18n anahtarları eklendi, a11y kontrol edildi (UI).
6. Gerekli ADR/doküman güncellendi; migration geri dönüş planı var.
7. Gözlemlenebilirlik (log/metrik/span) eklendi.

---

## 10. AI Ajanı İçin Çalışma Kuralları

**Başlamadan önce**
- Görevle ilgili modülün `contracts` ve `lib.rs`'ini, ilgili ADR'leri ve mevcut benzer kodu oku. Mevcut desenleri taklit et; yeni desen icat etme.
- Belirsizlik varsa ve yanlış tahmin pahalıysa (mimari, veri modeli, kırıcı API) **önce sor**; ucuzsa makul varsayımla ilerle ve varsayımı belirt.
- Büyük işi küçük, bağımsız doğrulanabilir adımlara böl; her adımda derlenen/test edilen kod bırak.

**Yaparken**
- **Modül sınırlarını asla ihlal etme.** Başka modülün verisi/davranışı gerekiyorsa: (a) olay yayınla/dinle ya da (b) port tanımla — ve bunu kullanıcıya açıkça söyle. Kısayol olarak cross-module import veya cross-schema SQL yazma.
- Yeni modül/port/olay eklerken önce `contracts`'ı tasarla, sonra implementasyonu yaz.
- Domain katmanına altyapı bağımlılığı sızdırma. Yeni crate eklemeden önce gerekçelendir (bakım durumu, lisans, boyut); `[workspace.dependencies]` üzerinden ekle.
- Kapsam dışına taşma: istenmeyen refactor, toplu yeniden biçimlendirme, ilgisiz dosya değişikliği yapma. Fark ettiğin ilgisiz sorunları ayrı not olarak bildir.
- Lint/format/test hatalarını **bastırma** (`#[allow]`, `eslint-disable`, `@ts-ignore`) — kök nedeni düzelt. Kaçınılmazsa dar kapsamda, gerekçe yorumuyla ve `expect` varyantıyla (`#[expect(...)]`, `@ts-expect-error`) kullan.
- Testi geçirmek için testi zayıflatma/silme. Hatalı testi düzeltiyorsan nedenini açıkla.
- Asla uydurma API/crate/fonksiyon kullanma; emin değilsen dokümantasyonu veya kaynak kodu kontrol et.
- Sır, token, gerçek kullanıcı verisi yazma/loglama.

**Bitirirken**
- Değişiklikten sonra §12'deki ilgili komutları çalıştır: en azından `just check` (fmt + lint + typecheck + hızlı testler). Çalıştıramadıysan bunu açıkça belirt; "çalışıyor" iddiasında bulunma.
- Özet ver: ne değişti, neden, hangi testler eklendi, bilinen sınırlar/riskler, sonraki mantıklı adım. Adım adım yapılanların dökümünü verme.

---

## 11. Yapılacaklar / Yapılmayacaklar Hızlı Referans

**YAP**
- Contracts-first modül tasarımı; olay + port ile gevşek bağlılık.
- Newtype ID'ler, value object'ler, açık hata tipleri.
- Outbox ile güvenilir olay yayını; idempotent tüketiciler.
- OpenAPI → TS client üretimi; tek gerçek kaynak.
- Küçük PR, Conventional Commits, ADR.
- Editör ağır işlerini Worker/`spawn_blocking`'e al.

**YAPMA**
- Modüller arası doğrudan `core` import'u, ortak DB tablosu, cross-schema JOIN/FK.
- Domain'de sqlx/axum/HTTP tipi; handler'da iş mantığı; bileşende iş mantığı.
- `unwrap()` üretimde, `any` TS'de, `v-html` kullanıcı içeriğinde.
- Sunucu verisini Pinia'ya kopyalamak; elle yazılmış API client.
- Kullanıcıya görünen sabit metin (i18n dışı).
- Lint'i susturarak ilerlemek.

---

## 12. Komutlar (`just` — tek giriş noktası)

Justfile `SQLX_OFFLINE=true` export eder. `query!` derlemesi commit'lenmiş `.sqlx` önbelleğini kullanır;
şema, süreç ayağa kalkınca migration ile oluşur. Canlı veritabanına karşı önbellek yenilemek yalnızca
`just db-prepare` içindedir (`SQLX_OFFLINE=false`).

```
just dev-infra      # docker compose: postgres, redis
just dev-api        # yoksa .env yazar, migration uygular, 127.0.0.1:8080
just dev-web        # vite
just jwt            # yeni JWT_SECRET için export satırı basar; .env'i değiştirmez

just fmt            # cargo fmt + prettier --write
just lint           # clippy -D warnings + eslint
just typecheck      # cargo check + vue-tsc
just test           # cargo test + vitest
just arch           # xtask bağımlılık testleri
just check          # fmt-check + lint + typecheck + test + arch + gen-api-check

just db-migrate     # xtask migrate
just db-prepare     # cargo sqlx prepare --workspace (çevrimdışı kapalı)
just gen-api        # openapi.json + TS client
just gen-api-check  # client drift
just vendor-check   # vendor/ kaynak, lisans ve SHA kontrolü
just budget         # gzip boyut bütçeleri (web build gerekir; `check` içinde değil)
```

Henüz yok (hedef, şu an reçete değil): `just setup`, `just dev`, `just dev-worker`, `just test-e2e`,
`just new-module`. Lint hedefine stylelint, knip, cargo-deny ve cargo-machete de eklenecek.

---

## 13. Modül Başına Sorumluluk Özeti (başlangıç haritası)

| Modül | Sorumluluk | Yayınladığı olaylar (örnek) | Tükettiği (örnek) |
|---|---|---|---|
| identity | Kullanıcı, oturum, OIDC, takım/organizasyon | `UserRegistered`, `UserDeleted` | — |
| workspace | Klasör, dosya meta, paylaşım, izin (ACL), çöp kutusu | `FileShared`, `PermissionRevoked`, `FileTrashed` | `UserDeleted`, `DocumentCreated` (meta yansıması) |
| documents | Docs içeriği, sürümler, yorum bağlantıları | `DocumentCreated`, `DocumentPublished` | `PermissionRevoked` |
| spreadsheets | Sheets, formül motoru, aralıklar, grafikler | `SpreadsheetCreated` | `PermissionRevoked` |
| presentations | Slides, şablonlar, sunum modu | `PresentationCreated` | `PermissionRevoked` |
| collaboration | WS sync, presence, yorumlar, öneriler | `CommentAdded`, `MentionCreated` | `PermissionRevoked`, `*Created` |
| storage | Asset/dosya yükleme, import/export işleri | `AssetUploaded`, `ExportReady` | `FileTrashed` |
| notifications | E-posta/uygulama içi bildirim | — | `MentionCreated`, `FileShared`, `ExportReady` |
| search | Tam metin indeks (Postgres FTS → ileride Meilisearch/OpenSearch) | — | `*Created`, `FileTrashed` |

> Bu tablo başlangıç önerisidir; değişiklikler ADR ile yapılır. Modül sayısı arttıkça sınır ihlali en büyük risktir — şüphede olay/port kullan.

---

## 14. Gelecek Evrim Yolu (Mimariyi buna uygun tut)

Modüler monolit → gerektiğinde mikroservis. Bu yüzden: modüller arası iletişim yalnızca contracts/olay/port; paylaşılan veritabanı yok; olaylar serileştirilebilir ve versiyonlu; senkron portlar ağ üzerinden de implement edilebilecek şekilde (`async`, hata tipli, idempotent) tasarlanır. Ayrılması en muhtemel adaylar: `collaboration` (WS ölçeği), `storage` (import/export işçileri), `search`.

---

## 15. Karar Bekleyen Konular (Başlangıçta ADR ile kapat)

1. Sheets ızgara motoru: tamamen özel Canvas mı, yoksa lisans/performans açısından onaylı bir kütüphane mi?
2. Sheets verisi için CRDT modeli (hücre bazlı Y.Map vs. yapılandırılmış op log).
3. Slides sahne modeli (SVG vs. Canvas) ve şablon formatı.
4. OOXML import/export stratejisi (Rust crate'leri vs. ayrı dönüştürücü servis).
5. Çok kiracılık modeli (organizasyon bazlı satır filtresi vs. şema-per-tenant).
6. Olay veriyolu: in-process → NATS geçiş zamanı.
7. Arama: Postgres FTS ile ne kadar gidilecek.
