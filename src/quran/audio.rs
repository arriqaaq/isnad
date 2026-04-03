use anyhow::Result;
use surrealdb::Surreal;
use surrealdb::types::RecordId;

use crate::db::Db;

fn rid(table: &str, key: &str) -> RecordId {
    RecordId::new(table, key)
}

struct ReciterSeed {
    name_en: &'static str,
    name_ar: Option<&'static str>,
    style: Option<&'static str>,
    folder_name: &'static str,
    bitrate: Option<&'static str>,
}

const RECITERS: &[ReciterSeed] = &[
    ReciterSeed {
        name_en: "Mishary Alafasy",
        name_ar: Some("مشاري العفاسي"),
        style: Some("Murattal"),
        folder_name: "Alafasy_128kbps",
        bitrate: Some("128kbps"),
    },
    ReciterSeed {
        name_en: "Abdul Basit (Murattal)",
        name_ar: Some("عبد الباسط عبد الصمد"),
        style: Some("Murattal"),
        folder_name: "Abdul_Basit_Murattal_192kbps",
        bitrate: Some("192kbps"),
    },
    ReciterSeed {
        name_en: "Abdul Basit (Mujawwad)",
        name_ar: None,
        style: Some("Mujawwad"),
        folder_name: "Abdul_Basit_Mujawwad_128kbps",
        bitrate: Some("128kbps"),
    },
    ReciterSeed {
        name_en: "Al-Husary",
        name_ar: Some("محمود خليل الحصري"),
        style: Some("Murattal"),
        folder_name: "Husary_128kbps",
        bitrate: Some("128kbps"),
    },
    ReciterSeed {
        name_en: "Al-Husary (Muallim)",
        name_ar: None,
        style: Some("Teaching"),
        folder_name: "Husary_Muallim_128kbps",
        bitrate: Some("128kbps"),
    },
    ReciterSeed {
        name_en: "As-Sudais",
        name_ar: Some("عبدالرحمن السديس"),
        style: Some("Murattal"),
        folder_name: "Abdurrahmaan_As-Sudais_192kbps",
        bitrate: Some("192kbps"),
    },
    ReciterSeed {
        name_en: "Al-Ghamdi",
        name_ar: Some("سعد الغامدي"),
        style: Some("Murattal"),
        folder_name: "Ghamadi_40kbps",
        bitrate: Some("40kbps"),
    },
    ReciterSeed {
        name_en: "Maher Al-Muaiqly",
        name_ar: Some("ماهر المعيقلي"),
        style: Some("Murattal"),
        folder_name: "MaherAlMuaiqly128kbps",
        bitrate: Some("128kbps"),
    },
    ReciterSeed {
        name_en: "Muhammad Ayyoub",
        name_ar: Some("محمد أيوب"),
        style: Some("Murattal"),
        folder_name: "Muhammad_Ayyoub_128kbps",
        bitrate: Some("128kbps"),
    },
    ReciterSeed {
        name_en: "Hani Ar-Rifai",
        name_ar: Some("هاني الرفاعي"),
        style: Some("Murattal"),
        folder_name: "Hani_Rifai_192kbps",
        bitrate: Some("192kbps"),
    },
    ReciterSeed {
        name_en: "Abu Bakr Ash-Shatri",
        name_ar: Some("أبو بكر الشاطري"),
        style: Some("Murattal"),
        folder_name: "Abu_Bakr_Ash-Shaatree_128kbps",
        bitrate: Some("128kbps"),
    },
    ReciterSeed {
        name_en: "Nasser Al-Qatami",
        name_ar: Some("ناصر القطامي"),
        style: Some("Murattal"),
        folder_name: "Nasser_Alqatami_128kbps",
        bitrate: Some("128kbps"),
    },
    ReciterSeed {
        name_en: "Warsh (Abdul Basit)",
        name_ar: None,
        style: Some("Warsh"),
        folder_name: "warsh/warsh_Abdul_Basit_128kbps",
        bitrate: Some("128kbps"),
    },
];

pub async fn init_reciters(db: &Surreal<Db>) -> Result<()> {
    for r in RECITERS {
        let key = r.folder_name.replace('/', "_");

        let result = db
            .query(
                "CREATE $rid CONTENT { \
                 name_en: $name_en, name_ar: $name_ar, style: $style, \
                 folder_name: $folder_name, bitrate: $bitrate }",
            )
            .bind(("rid", rid("reciter", &key)))
            .bind(("name_en", r.name_en))
            .bind(("name_ar", r.name_ar))
            .bind(("style", r.style))
            .bind(("folder_name", r.folder_name))
            .bind(("bitrate", r.bitrate))
            .await
            .and_then(|r| r.check());

        if let Err(e) = result {
            let msg = e.to_string();
            if msg.contains("already exists") || msg.contains("Database record") {
                continue;
            }
            tracing::error!("Failed to insert reciter {key}: {e}");
            return Err(e.into());
        }
    }
    tracing::info!("Reciters initialized ({} entries)", RECITERS.len());
    Ok(())
}
