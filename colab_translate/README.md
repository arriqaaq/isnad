# Hadith Translation via Google Colab

Translate Arabic hadiths to English using Qwen2.5-7B-Instruct-AWQ on Colab's free T4 GPU.
Each book is translated across multiple Colab sessions with checkpointing.

## Prerequisites

- Python 3.10+
- `data/sanadset.csv` in the project root (run `make ingest` first if missing)
- A Google account for Colab

## Step 1: Split CSV by book

```bash
cd colab_translate
python split_csv.py
```

This creates `splits/` with per-book files:
```
splits/bukhari.json              # 6,054 hadiths
splits/bukhari_narrators.json    # unique narrator names
splits/muslim.json               # 5,330 hadiths
...
```

## Step 2: Upload and run on Colab

1. Open `translate_colab.ipynb` in Google Colab
2. **Set runtime to GPU**: Runtime > Change runtime type > T4 GPU
3. In Cell 1, set `BOOK = "bukhari"` (or whichever book)
4. Run Cell 2: upload `splits/bukhari.json` + `splits/bukhari_narrators.json`
5. Run all remaining cells

The notebook will:
- Load the Qwen model (~2 min)
- Translate narrator names (~5-30 min depending on count)
- Translate hadiths one by one (~30s each)
- Checkpoint every 50 hadiths automatically

## Step 3: Resume across sessions

Each session translates ~1,440 hadiths (12h at ~30s each). To continue:

1. **Download** `checkpoint_{book}.json` at end of session (Cell 6)
2. Next session: upload `{book}.json` + `checkpoint_{book}.json`
3. The notebook detects the checkpoint and resumes where it left off

## Step 4: Collect results

After all sessions for a book are done, save:
- `translated_{book}.json` → `translations/`
- `translated_{book}_narrators.json` → `translations/`

## Session estimates

| Book | Hadiths | Sessions (~12h each) |
|------|---------|----------------------|
| bukhari | 6,054 | 4-5 |
| muslim | 5,330 | 4 |
| nasai | 5,662 | 4 |
| abudawud | 4,590 | 3-4 |
| ibnmajah | 4,332 | 3 |
| tirmidhi | 3,891 | 3 |
| **Total** | **~30,000** | **~21-23** |

## Output format

`translated_{book}.json`:
```json
[
  {"hadith_number": 1, "text_ar": "...", "text_en": "..."},
  {"hadith_number": 2, "text_ar": "...", "text_en": "..."}
]
```

`translated_{book}_narrators.json`:
```json
[
  {"name_ar": "أبو هريرة", "name_en": "Abu Hurayrah"},
  {"name_ar": "عائشة بنت أبي بكر", "name_en": "Aisha bint Abi Bakr"}
]
```

## Tips

- **Don't close the Colab tab** during translation — it may disconnect
- **Download checkpoint frequently** if you're worried about session drops
- You can adjust `CHECKPOINT_EVERY` in Cell 1 (default: 50)
- Narrators only need translating once per book — the checkpoint remembers this
