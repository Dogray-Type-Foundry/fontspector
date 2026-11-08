use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::LazyLock;

use fontspector_checkapi::{prelude::*, testfont, FileTypeConvert};
use google_fonts_languages::LANGUAGES;
use unicode_normalization::UnicodeNormalization;

/// A country using a currency, and its languages (google-fonts-languages ids).
struct Country {
    name: &'static str,
    /// The country's principal language(s): a supported main language with a
    /// missing currency symbol is a FAIL.
    main: &'static [&'static str],
    /// Other official languages: a missing symbol is a WARN.
    other: &'static [&'static str],
}

struct Currency {
    symbol: char,
    name: &'static str,
    countries: &'static [Country],
}

const fn country(
    name: &'static str,
    main: &'static [&'static str],
    other: &'static [&'static str],
) -> Country {
    Country { name, main, other }
}

/// Currencies with a dedicated Unicode symbol, and the countries that use them.
/// Currencies written with letters (kr, zł, R$…) or with ASCII `$` are not listed.
/// Edit this table to change the Dogray currency policy.
const CURRENCIES: &[Currency] = &[
    Currency {
        symbol: '€',
        name: "Euro",
        countries: &[
            country("Austria", &["de_Latn"], &[]),
            country("Belgium", &["nl_Latn", "fr_Latn"], &["de_Latn"]),
            country("Bulgaria", &["bg_Cyrl"], &[]),
            country("Croatia", &["hr_Latn"], &[]),
            country("Cyprus", &["el_Grek"], &["tr_Latn"]),
            country("Estonia", &["et_Latn"], &[]),
            country("Finland", &["fi_Latn"], &["sv_Latn"]),
            country("France", &["fr_Latn"], &[]),
            country("Germany", &["de_Latn"], &[]),
            country("Greece", &["el_Grek"], &[]),
            country("Ireland", &["en_Latn"], &["ga_Latn"]),
            country("Italy", &["it_Latn"], &[]),
            country("Latvia", &["lv_Latn"], &[]),
            country("Lithuania", &["lt_Latn"], &[]),
            country("Luxembourg", &["lb_Latn"], &["fr_Latn", "de_Latn"]),
            country("Malta", &["mt_Latn"], &["en_Latn"]),
            country("Netherlands", &["nl_Latn"], &["fy_Latn"]),
            country("Portugal", &["pt_Latn"], &[]),
            country("Slovakia", &["sk_Latn"], &[]),
            country("Slovenia", &["sl_Latn"], &[]),
            country("Spain", &["es_Latn"], &["ca_Latn", "eu_Latn", "gl_Latn"]),
            country("Andorra", &["ca_Latn"], &[]),
            country("Kosovo", &["sq_Latn"], &["sr_Cyrl"]),
            country("Montenegro", &["cnr_Latn"], &["sr_Latn"]),
        ],
    },
    Currency {
        symbol: '£',
        name: "Pound sterling",
        countries: &[country(
            "United Kingdom",
            &["en_Latn"],
            &["cy_Latn", "gd_Latn"],
        )],
    },
    Currency {
        symbol: '¥',
        name: "Yen / Yuan",
        countries: &[
            country("Japan", &["ja_Jpan"], &[]),
            country("China", &["zh_Hans"], &[]),
        ],
    },
    Currency {
        symbol: '₩',
        name: "Won",
        countries: &[
            country("South Korea", &["ko_Kore"], &[]),
            country("North Korea", &["ko_Kore"], &[]),
        ],
    },
    Currency {
        symbol: '₹',
        name: "Indian rupee",
        countries: &[country(
            "India",
            &[
                "hi_Deva", "bn_Beng", "te_Telu", "mr_Deva", "ta_Taml", "gu_Gujr", "kn_Knda",
                "ml_Mlym", "or_Orya", "pa_Guru", "as_Beng", "mai_Deva", "sat_Olck", "ne_Deva",
                "kok_Deva", "doi_Deva", "mni_Mtei", "brx_Deva", "sa_Deva",
            ],
            &["en_Latn", "ur_Arab", "ks_Arab", "sd_Arab"],
        )],
    },
    Currency {
        symbol: '₨',
        name: "Rupee (Pakistan, Sri Lanka, Nepal)",
        countries: &[
            country("Pakistan", &["ur_Arab", "pa_Arab"], &["en_Latn"]),
            country("Sri Lanka", &["si_Sinh"], &["ta_Taml"]),
            country("Nepal", &["ne_Deva"], &[]),
        ],
    },
    Currency {
        symbol: '৳',
        name: "Taka",
        countries: &[country("Bangladesh", &["bn_Beng"], &[])],
    },
    Currency {
        symbol: '₦',
        name: "Naira",
        countries: &[country(
            "Nigeria",
            &["ha_Latn", "yo_Latn", "ig_Latn"],
            &["en_Latn"],
        )],
    },
    Currency {
        symbol: '₱',
        name: "Philippine peso",
        countries: &[country("Philippines", &["fil_Latn"], &["en_Latn"])],
    },
    // Ghana's principal language (Akan) is not in the language data.
    Currency {
        symbol: '₵',
        name: "Cedi",
        countries: &[country("Ghana", &[], &["en_Latn"])],
    },
    Currency {
        symbol: '₲',
        name: "Guaraní",
        countries: &[country("Paraguay", &["es_Latn", "gn_Latn"], &[])],
    },
    Currency {
        symbol: '₡',
        name: "Colón",
        countries: &[country("Costa Rica", &["es_Latn"], &[])],
    },
    Currency {
        symbol: '₴',
        name: "Hryvnia",
        countries: &[country("Ukraine", &["uk_Cyrl"], &[])],
    },
    Currency {
        symbol: '₽',
        name: "Ruble",
        countries: &[country("Russia", &["ru_Cyrl"], &[])],
    },
    Currency {
        symbol: '₸',
        name: "Tenge",
        countries: &[country("Kazakhstan", &["kk_Cyrl"], &["ru_Cyrl"])],
    },
    Currency {
        symbol: '\u{20C0}',
        name: "Som",
        countries: &[country("Kyrgyzstan", &["ky_Cyrl"], &["ru_Cyrl"])],
    },
    Currency {
        symbol: '₼',
        name: "Manat",
        countries: &[country("Azerbaijan", &["az_Latn"], &[])],
    },
    Currency {
        symbol: '₾',
        name: "Lari",
        countries: &[country("Georgia", &["ka_Geor"], &[])],
    },
    Currency {
        symbol: '֏',
        name: "Dram",
        countries: &[country("Armenia", &["hy_Armn"], &[])],
    },
    Currency {
        symbol: '₺',
        name: "Turkish lira",
        countries: &[country("Turkey", &["tr_Latn"], &[])],
    },
    Currency {
        symbol: '₪',
        name: "New shekel",
        countries: &[country("Israel", &["he_Hebr"], &[])],
    },
    Currency {
        symbol: '₫',
        name: "Đồng",
        countries: &[country("Vietnam", &["vi_Latn"], &[])],
    },
    Currency {
        symbol: '฿',
        name: "Baht",
        countries: &[country("Thailand", &["th_Thai"], &[])],
    },
    Currency {
        symbol: '₭',
        name: "Kip",
        countries: &[country("Laos", &["lo_Laoo"], &[])],
    },
    Currency {
        symbol: '៛',
        name: "Riel",
        countries: &[country("Cambodia", &["km_Khmr"], &[])],
    },
    Currency {
        symbol: '₮',
        name: "Tögrög",
        countries: &[country("Mongolia", &["mn_Cyrl"], &[])],
    },
    Currency {
        symbol: '؋',
        name: "Afghani",
        countries: &[country("Afghanistan", &["prs_Arab", "ps_Arab"], &[])],
    },
    Currency {
        symbol: '﷼',
        name: "Rial",
        countries: &[country("Iran", &["fa_Arab"], &[])],
    },
    // Encoded in Unicode 17.0 (2025)
    Currency {
        symbol: '\u{20C1}',
        name: "Saudi riyal",
        countries: &[country("Saudi Arabia", &["ar_Arab"], &[])],
    },
];

fn parse_chars(chars: &str) -> HashSet<char> {
    chars
        .split_whitespace()
        .map(|x| x.trim_start_matches('{').trim_end_matches('}'))
        .flat_map(|x| {
            let nfc = x.nfc().collect::<String>();
            [x.to_string(), nfc]
        })
        .flat_map(|x| x.chars().collect::<Vec<_>>())
        .collect()
}

/// Base exemplar characters of every language the table mentions.
static EXEMPLARS: LazyLock<HashMap<&'static str, HashSet<char>>> = LazyLock::new(|| {
    CURRENCIES
        .iter()
        .flat_map(|c| c.countries)
        .flat_map(|c| c.main.iter().chain(c.other))
        .filter_map(|id| {
            let lang = LANGUAGES.get(*id)?;
            let bases = lang
                .exemplar_chars
                .as_ref()
                .map(|e| parse_chars(e.base()))?;
            (!bases.is_empty()).then_some((*id, bases))
        })
        .collect()
});

fn language_name(id: &str) -> String {
    LANGUAGES
        .get(id)
        .map(|l| l.name().to_string())
        .unwrap_or_else(|| id.to_string())
}

#[check(
    id = "dogray/currency_symbols",
    rationale = "
        If a font supports a language of a country whose currency has a Unicode
        symbol, the font should include that symbol.

        A language counts as supported when the font covers all of its base
        exemplar characters (from the Google Fonts language data).

        * FAIL: the font supports a country's principal language (e.g. German → €,
          Hindi → ₹, Yoruba → ₦) but lacks the currency symbol.

        * WARN: the font only supports another official language of that country
          (e.g. English in India → ₹, English in Nigeria → ₦).

        This replaces the Universal rupee check, which only covered ₹.
    ",
    proposal = "Dogray profile",
    title = "Fonts include the currency symbols of the countries whose languages they support"
)]
fn currency_symbols(t: &Testable, context: &Context) -> CheckFnResult {
    let f = testfont!(t);
    let codepoints = f.codepoints(Some(context));
    let supports = |id: &str| {
        EXEMPLARS
            .get(id)
            .is_some_and(|bases| bases.iter().all(|c| codepoints.contains(&(*c as u32))))
    };
    let mut fails = vec![];
    let mut warns = vec![];
    for currency in CURRENCIES {
        if codepoints.contains(&(currency.symbol as u32)) {
            continue;
        }
        let mut main_hits: BTreeSet<String> = BTreeSet::new();
        let mut other_hits: BTreeSet<String> = BTreeSet::new();
        for c in currency.countries {
            for id in c.main.iter().filter(|id| supports(id)) {
                main_hits.insert(format!("{} ({})", language_name(id), c.name));
            }
            for id in c.other.iter().filter(|id| supports(id)) {
                other_hits.insert(format!("{} ({})", language_name(id), c.name));
            }
        }
        let row = |hits: BTreeSet<String>| {
            vec![
                format!("`{}`", currency.symbol),
                currency.name.to_string(),
                format!("U+{:04X}", currency.symbol as u32),
                hits.into_iter().collect::<Vec<_>>().join(", "),
            ]
        };
        if !main_hits.is_empty() {
            fails.push(row(main_hits));
        } else if !other_hits.is_empty() {
            warns.push(row(other_hits));
        }
    }
    let headers = [
        "Symbol",
        "Currency",
        "Code point",
        "Supported languages (country)",
    ];
    let mut problems = vec![];
    if !fails.is_empty() {
        problems.push(Status::fail(
            "missing-currency",
            &format!(
                "Missing currency symbols of countries whose principal language the font supports ({}):\n\n{}",
                fails.len(),
                markdown_table(context, &headers, fails)
            ),
        ));
    }
    if !warns.is_empty() {
        problems.push(Status::warn(
            "missing-currency-official-language",
            &format!(
                "Missing currency symbols of countries where the font supports an official, but not the principal, language ({}):\n\n{}",
                warns.len(),
                markdown_table(context, &headers, warns)
            ),
        ));
    }
    return_result(problems)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_language_id_exists() {
        let missing: Vec<&str> = CURRENCIES
            .iter()
            .flat_map(|c| c.countries)
            .flat_map(|c| c.main.iter().chain(c.other))
            .copied()
            .filter(|id| !LANGUAGES.contains_key(*id))
            .collect();
        assert!(missing.is_empty(), "Unknown language ids: {missing:?}");
    }
}
