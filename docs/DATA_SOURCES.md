# Data Sources

## 1. Sanadset 650K (Primary Hadith Data)

**Source**: Mendeley Data
**URL**: https://data.mendeley.com/datasets/5xth87zwb5/5
**Paper**: https://pmc.ncbi.nlm.nih.gov/articles/PMC9440281/
**License**: CC BY 4.0

Contains 650,986 hadith records with isnad chains from 926 books. The ingestion pipeline filters for the Kutub al-Sittah (6 canonical collections) by default.

**Auto-download**: Runs automatically on `hadith ingest` if `data/sanadset.csv` is missing.

### Columns
- Hadith text (Arabic, with XML-style markup)
- Book name (Arabic)
- Hadith number
- Matn (hadith text body)
- Sanad (narrator chain as Python-style list)

---

## 2. AR-Sanad Narrator Biographical Data

**Source**: GitHub - somaia02/Narrator-Disambiguation
**URL**: https://github.com/somaia02/Narrator-Disambiguation
**Paper**: https://www.mdpi.com/2078-2489/13/2/55
**Data file**: `Narrators data.csv` (18,298 narrators)

Contains narrator biographical data scraped from the Khadem Al-haramyn website, including Ibn Hajar's reliability classifications from Taqrib al-Tahdhib.

### Manual Download

```bash
curl -sL "https://raw.githubusercontent.com/somaia02/Narrator-Disambiguation/main/Narrators%20data.csv" \
  -o data/ar_sanad_narrators.csv
```

### Auto-download

The ingestion module downloads automatically when you run:
```bash
cargo run -- ingest --narrator-bio data/ar_sanad_narrators.csv
```

If the file doesn't exist at the specified path, it downloads from GitHub.

### CSV Columns

| Column | Description | Example |
|--------|-------------|---------|
| `name` | Full formal Arabic name | علي بن إبراهيم بن سلمة بن بحر |
| `namings` | Alternative name forms (Python list) | ['أبو الحسن', 'أبو الحسن القطان', ...] |
| `Ibnhajar_rank` | Ibn Hajar's reliability assessment | ثقة, صدوق, مجهول, ضعيف, متروك |
| `shuhra` | Famous name / common epithet | - |
| `laqab` | Title or epithet | الحافظ , شيخ الإسلام |
| `nasab` | Lineage description | القطان , الحافظ , القزويني |
| `selat_karaba` | Family relationships | كان له بنون ثلاثة |
| `mazhab` | School of thought | - |
| `kunia` | Patronymic (Abu/Umm X) | أبو الحسن |
| `death_year` | Death year (Hijri) | 345 هـ |
| `tabaqa` | Generation/era | - |
| `zahabi_rank` | Al-Dhahabi's assessment | - |
| `living_city` | City of residence | قزوين |
| `birth_year` | Birth year (Hijri) | 254 هـ |
| `death_city` | City of death | - |
| `journey_city` | Cities traveled to | اليمن |
| `id` | Unique numeric ID | 0 |
| `narrated_from` | IDs of teachers | [672, 8537, ...] |
| `narrated_to` | IDs of students | [986] |

### Ibn Hajar Rank Distribution (top values)

| Rank | Count | Mapped To |
|------|-------|-----------|
| `-` (no rank) | 10,166 | Skipped |
| ثقة (trustworthy) | 1,411 | thiqah (0.75) |
| مقبول (acceptable) | 1,239 | saduq (0.65) |
| صدوق (truthful) | 872 | saduq (0.65) |
| مجهول (unknown) | 510 | majhul (0.50) |
| صحابي (companion) | 353 | thiqah (0.75) |
| ضعيف (weak) | 292 | daif (0.35) |
| مستور (concealed) | 118 | majhul (0.50) |
| صدوق يخطئ (truthful, errs) | 101 | saduq (0.65) |
| متروك (abandoned) | 88 | matruk (0.20) |

### Matching Algorithm

The ingestion module matches AR-Sanad narrators to existing database records:

1. **Exact match**: Normalized Arabic name from DB == normalized shuhra from AR-Sanad
2. **Substring match**: DB narrator name found within AR-Sanad full formal name
3. **Ambiguous**: Multiple matches skipped (logged for manual review)

Normalization strips diacritics, normalizes alef/taa marbuta/alef maqsura variants, and keeps only Arabic letters + spaces.

### What Gets Updated

For each matched narrator, the following fields are set:
- `reliability_rating` (thiqah/saduq/majhul/daif/matruk)
- `reliability_prior` (0.75/0.65/0.50/0.35/0.20)
- `reliability_source` ("Taqrib al-Tahdhib (AR-Sanad)")
- `death_year`, `death_calendar` (if available)
- `birth_year`, `birth_calendar` (if available)
- `kunya` (if available)
- `locations` (living city, if available)
- `generation` (tabaqa, if available)
- `tags` (laqab + rating)

An `evidence` record is also created linking the narrator to the specific Ibn Hajar assessment.

---

## 3. Sunnah.com Translations (English)

**Source**: HuggingFace - meeAtif/hadith_datasets
**URL**: https://huggingface.co/datasets/meeAtif/hadith_datasets/
**License**: Various (from sunnah.com)

Human-verified English translations for the 6 canonical collections. Downloaded automatically during ingestion via `merge_human_translations()`.
