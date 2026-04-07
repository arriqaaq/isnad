# Data Sources

## 1. SemanticHadith KG V2 (Primary — Hadith + Narrator Data)

**Source**: GitHub - A-Kamran/SemanticHadith-V2
**URL**: https://github.com/A-Kamran/SemanticHadith-V2
**Paper**: Journal of Web Semantics, 2023
**Format**: RDF/Turtle (202 MB), extracted to JSON for ingestion
**License**: Academic open access

The primary data source for hadith ingestion. Contains 34,457 hadiths across the 6 major books (Kutub al-Sittah) with fully identified narrator chains, narrator biographical data, topic classifications, similarity links, and Quran verse references.

### What it provides

**Per hadith (34,457 total)**:
- Arabic text (diacritized, full with isnad)
- English translation (Bukhari + Abu Dawud only, ~12.5K)
- Ordered narrator chain with unique narrator IDs
- Hadith type (elevated/suspended/etc.) — 78% coverage
- Topic classification (Salah, Hajj, Zakah, etc.) — 30%
- Entity mentions (people, places, events) — 81%
- Quran verse references — 2%
- Similar/strongly-similar hadith links — 16-18%
- Chapter structure + prefaces — 100%

**Per narrator (6,786 total)**:
- popularName (98%), full genealogical name (98%)
- teknonym / kunya (61%)
- generation / tabaqa (97%)
- lineage / nisba (94%)
- residence (85%)
- death year (25%), birth year (4%)
- title (25%), office (19%)
- physical attributes (4%)

### Book Coverage

| Book | Arabic Name | Hadiths |
|------|------------|---------|
| Sahih al-Bukhari | صحيح البخاري | 7,322 |
| Sahih Muslim | صحيح مسلم | 7,454 |
| Sunan an-Nasa'i | سنن النسائى الصغرى | 5,736 |
| Sunan Abi Dawud | سنن أبي داود | 5,244 |
| Sunan Ibn Majah | سنن ابن ماجه | 4,330 |
| Jami at-Tirmidhi | جامع الترمذي | 3,925 |

### Setup

```bash
make semantic-download    # Download TTL from GitHub (~27 MB)
make semantic-extract     # Extract to data/semantic_hadith.json (~56 MB)
make semantic-verify      # Verify data integrity
```

---

## 2. Sunnah.com Translations (English)

**Source**: HuggingFace - meeAtif/hadith_datasets
**URL**: https://huggingface.co/datasets/meeAtif/hadith_datasets/
**License**: Various (from sunnah.com)

Human-verified English translations for the 6 canonical collections. Downloaded automatically during ingestion via `merge_human_translations()`. Fills in English translations where SemanticHadith doesn't provide them (Muslim, Tirmidhi, Nasa'i, Ibn Majah).

### Available Books

| Book | Arabic Name | HuggingFace File |
|---|---|---|
| Sahih al-Bukhari | صحيح البخاري | `Sahih al-Bukhari.csv` |
| Sahih Muslim | صحيح مسلم | `Sahih Muslim.csv` |
| Sunan Abi Dawud | سنن أبي داود | `Sunan Abi Dawud.csv` |
| Sunan an-Nasa'i | سنن النسائى الصغرى | `Sunan an-Nasa'i.csv` |
| Jami at-Tirmidhi | جامع الترمذي | `Jami at-Tirmidhi.csv` |
| Sunan Ibn Majah | سنن ابن ماجه | `Sunan Ibn Majah.csv` |

---

## 3. Reference / Future Data Sources

### Sanadset 650K

**Source**: Mendeley Data
**URL**: https://data.mendeley.com/datasets/5xth87zwb5/5
**Paper**: https://pmc.ncbi.nlm.nih.gov/articles/PMC9440281/
**License**: CC BY 4.0

Contains 650,986 hadith records with isnad chains from 926 books. Not currently used in the ingestion pipeline (SemanticHadith KG is used instead for the 6 major books). Available for future expansion to non-major hadith collections.

```bash
make sanadset-download    # Download to data/sanadset.csv
```

### AR-Sanad Narrator Biographical Data

**Source**: GitHub - somaia02/Narrator-Disambiguation
**URL**: https://github.com/somaia02/Narrator-Disambiguation
**Paper**: https://www.mdpi.com/2078-2489/13/2/55

Contains 18,298 narrators with Ibn Hajar's reliability classifications from Taqrib al-Tahdhib. Not currently used (SemanticHadith provides narrator bios directly). Available for future reliability rating enrichment.

### Other Potential Sources

- **Hadith Narrators (+24K)**: https://www.kaggle.com/datasets/fahd09/hadith-narrators — CC0
- **Multi-IsnadSet (MIS)**: https://data.mendeley.com/datasets/gzprcr93zn/2 — Sahih Muslim graph
- **Dorar.net API**: https://github.com/AhmedElTabarani/dorar-hadith-api — hadith search + grading
- **fawazahmed0/hadith-api**: https://github.com/fawazahmed0/hadith-api — multi-language hadith API
