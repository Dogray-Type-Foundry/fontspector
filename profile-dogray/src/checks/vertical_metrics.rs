use fontspector_checkapi::{
    pens::BezGlyph, prelude::*, FileTypeConvert, TestFont, DEFAULT_LOCATION,
};
use kurbo::Shape;
use skrifa::{raw::TableProvider, GlyphId, MetadataProvider};

/// Cap height is measured from these glyphs (lowest top, to ignore overshoots).
const CAP_GLYPHS: [char; 3] = ['H', 'T', 'Z'];
/// Descender is measured from these glyphs (lowest bottom below the baseline).
const DESCENDER_GLYPHS: [char; 5] = ['g', 'j', 'p', 'q', 'y'];

#[check(
    id = "dogray/vertical_metrics",
    rationale = "
        A family's vertical metrics should follow one coherent strategy. This check
        evaluates the metrics against two strategies, without preferring either, and
        reports which one the family follows. If the metrics fit neither, it fails.

        **Google Fonts strategy** (https://github.com/googlefonts/gf-docs/tree/main/VerticalMetrics):

        * USE_TYPO_METRICS (OS/2.fsSelection bit 7) is ON

        * OS/2 typo values equal the hhea values

        * typoLineGap and hheaLineGap are 0

        * winAscent/winDescent cover the family's yMax/yMin (no clipping)

        * typoAscender is higher than the tallest /Agrave

        * Recommended: caps centred (typoAscender - capHeight = |typoDescender|),
          typoDescender below the deepest descender, and ascender + |descender|
          around 120-130% of UPM

        **Dogray strategy**:

        * USE_TYPO_METRICS is OFF

        * typoAscender is the cap height and typoDescender is the descender
          (lowest of g j p q y; -UPM/5 for caps-only fonts), both measured in the
          Regular font; if several fonts are Regular (e.g. one per optical size),
          the one with the lowest descender

        * typoLineGap holds the extra line spacing, so the typo line
          (typoAscender + |typoDescender| + typoLineGap) equals the hhea line

        * hheaLineGap is 0 and caps are centred in the hhea line

        * winAscent/winDescent mirror hheaAscender/|hheaDescender|

        Both strategies need every font in the family to share the same values.
    ",
    proposal = "Dogray profile",
    title = "Vertical metrics follow the Google Fonts or the Dogray strategy",
    implementation = "all"
)]
fn vertical_metrics(c: &TestableCollection, _context: &Context) -> CheckFnResult {
    let fonts = TTF.from_collection(c);
    let infos = fonts
        .iter()
        .map(FontInfo::new)
        .collect::<Result<Vec<_>, _>>()?;
    let reference = pick_regular(&infos)
        .ok_or_else(|| FontspectorError::skip("no-fonts", "No TrueType/OpenType fonts to check"))?;
    let evaluation = evaluate(&infos, reference);
    Ok(Box::new(
        evaluation.into_statuses(&infos, reference).into_iter(),
    ))
}

/// Everything we need to know about one font.
struct FontInfo {
    name: String,
    upm: i32,
    weight: u16,
    width: u16,
    italic: bool,
    variable: bool,
    use_typo: bool,
    typo_asc: i32,
    typo_desc: i32,
    typo_gap: i32,
    hhea_asc: i32,
    hhea_desc: i32,
    hhea_gap: i32,
    win_asc: i32,
    win_desc: i32,
    y_min: i32,
    y_max: i32,
    cap_height: i32,
    /// Glyph the cap height was measured from, if any
    cap_glyph: Option<char>,
    descender: i32,
    /// Glyph the descender was measured from, if any
    descender_glyph: Option<char>,
    agrave_top: Option<f64>,
}

impl FontInfo {
    fn new(f: &TestFont) -> Result<Self, FontspectorError> {
        let m = f.vertical_metrics()?;
        let os2 = f.font().os2()?;
        let head = f.font().head()?;
        let upm = m.upm as i32;
        let lowest_top = CAP_GLYPHS
            .iter()
            .filter_map(|c| bounds(f, *c).map(|b| (*c, b.y1)))
            .min_by(|a, b| a.1.total_cmp(&b.1));
        let (cap_height, cap_glyph) = if let Some((c, top)) = lowest_top {
            (top.floor() as i32, Some(c))
        } else if let Some(cap) = os2.s_cap_height().filter(|c| *c > 0) {
            (cap as i32, None)
        } else {
            ((upm as f64 * 0.7) as i32, None)
        };
        let lowest = DESCENDER_GLYPHS
            .iter()
            .filter_map(|c| bounds(f, *c).map(|b| (*c, b.y0)))
            .filter(|(_, y)| *y < 0.0)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        let (descender, descender_glyph) = match lowest {
            Some((c, y)) => (y.floor() as i32, Some(c)),
            None => (-((upm as f64 / 5.0).round() as i32), None),
        };
        Ok(FontInfo {
            name: f.filename.file_name().map_or_else(
                || f.filename.to_string_lossy().to_string(),
                |n| n.to_string_lossy().to_string(),
            ),
            upm,
            weight: os2.us_weight_class(),
            width: os2.us_width_class(),
            italic: f.is_italic().unwrap_or(false),
            variable: f.is_variable_font(),
            use_typo: f.use_typo_metrics()?,
            typo_asc: m.os2_typo_ascender as i32,
            typo_desc: m.os2_typo_descender as i32,
            typo_gap: m.os2_typo_linegap as i32,
            hhea_asc: m.hhea_ascent as i32,
            hhea_desc: m.hhea_descent as i32,
            hhea_gap: m.hhea_linegap as i32,
            win_asc: m.os2_win_ascent as i32,
            win_desc: m.os2_win_descent as i32,
            y_min: head.y_min() as i32,
            y_max: head.y_max() as i32,
            cap_height,
            cap_glyph,
            descender,
            descender_glyph,
            agrave_top: bounds(f, 'À').map(|b| b.y1),
        })
    }

    /// The eight values both strategies set identically in every font.
    fn values(&self) -> [i32; 8] {
        [
            self.typo_asc,
            self.typo_desc,
            self.typo_gap,
            self.hhea_asc,
            self.hhea_desc,
            self.hhea_gap,
            self.win_asc,
            self.win_desc,
        ]
    }

    fn typo_line(&self) -> i32 {
        self.typo_asc + self.typo_desc.abs() + self.typo_gap
    }

    fn hhea_line(&self) -> i32 {
        self.hhea_asc + self.hhea_desc.abs() + self.hhea_gap
    }
}

/// Exact outline bounds of the glyph mapped to `c` at the default location.
fn bounds(f: &TestFont, c: char) -> Option<kurbo::Rect> {
    let gid: GlyphId = f.font().charmap().map(c)?;
    let mut pen = BezGlyph::default();
    f.draw_glyph(gid, &mut pen, DEFAULT_LOCATION).ok()?;
    pen.iter()
        .filter(|p| !p.elements().is_empty())
        .map(|p| p.bounding_box())
        .reduce(|a, b| a.union(b))
}

/// The Regular (or most Regular-like) font, where Dogray measures cap height
/// and descender. Static fonts are preferred over variable ones; when several
/// fonts qualify (e.g. the Regular of each optical size), the one with the
/// lowest descender is used.
fn pick_regular(infos: &[FontInfo]) -> Option<&FontInfo> {
    infos.iter().min_by_key(|i| {
        (
            i.italic,
            i.variable,
            (i.width as i32 - 5).abs(),
            (i.weight as i32 - 400).abs(),
            i.descender,
        )
    })
}

#[derive(Clone, Copy, PartialEq)]
enum Verdict {
    Ok,
    /// Required by the strategy and not met
    Fail,
    /// Recommended by the strategy and not met
    Warn,
    /// Not part of the strategy
    NotApplicable,
}

struct Cell {
    verdict: Verdict,
    detail: String,
}

impl Cell {
    fn require(ok: bool, detail: impl Into<String>) -> Self {
        Cell {
            verdict: if ok { Verdict::Ok } else { Verdict::Fail },
            detail: detail.into(),
        }
    }
    fn recommend(ok: bool, detail: impl Into<String>) -> Self {
        Cell {
            verdict: if ok { Verdict::Ok } else { Verdict::Warn },
            detail: detail.into(),
        }
    }
    fn na() -> Self {
        Cell {
            verdict: Verdict::NotApplicable,
            detail: String::new(),
        }
    }
    fn symbol(&self) -> &'static str {
        match self.verdict {
            Verdict::Ok => "✅",
            Verdict::Fail => "❌",
            Verdict::Warn => "⚠️",
            Verdict::NotApplicable => "—",
        }
    }
}

struct Row {
    requirement: &'static str,
    google: Cell,
    dogray: Cell,
}

/// Picks one strategy's column out of a row.
type Strategy = fn(&Row) -> &Cell;

struct Evaluation {
    rows: Vec<Row>,
}

fn evaluate(infos: &[FontInfo], r: &FontInfo) -> Evaluation {
    let mut rows = vec![];
    let tol = 1;
    // Measured cap height and descender may be off by rounding in the build
    let measure_tol = 2;

    // Consistency across the family (required by both)
    let inconsistent: Vec<&str> = infos
        .iter()
        .filter(|i| i.values() != r.values() || i.use_typo != r.use_typo)
        .map(|i| i.name.as_str())
        .collect();
    let consistency = if inconsistent.is_empty() {
        format!("all {} font(s) match", infos.len())
    } else {
        format!("differs from {} in: {}", r.name, inconsistent.join(", "))
    };
    rows.push(Row {
        requirement: "Same values in every font",
        google: Cell::require(inconsistent.is_empty(), consistency.clone()),
        dogray: Cell::require(inconsistent.is_empty(), consistency),
    });

    let state = if r.use_typo { "ON" } else { "OFF" };
    rows.push(Row {
        requirement: "USE_TYPO_METRICS (fsSelection bit 7)",
        google: Cell::require(r.use_typo, format!("is {state}; must be ON")),
        dogray: Cell::require(!r.use_typo, format!("is {state}; must be OFF")),
    });

    let typo_eq_hhea =
        r.typo_asc == r.hhea_asc && r.typo_desc == r.hhea_desc && r.typo_gap == r.hhea_gap;
    rows.push(Row {
        requirement: "Typo values equal hhea values",
        google: Cell::require(
            typo_eq_hhea,
            format!(
                "typo {}/{}/{} vs hhea {}/{}/{}",
                r.typo_asc, r.typo_desc, r.typo_gap, r.hhea_asc, r.hhea_desc, r.hhea_gap
            ),
        ),
        dogray: Cell::na(),
    });

    rows.push(Row {
        requirement: "Line gaps",
        google: Cell::require(
            r.typo_gap == 0 && r.hhea_gap == 0,
            format!(
                "typoLineGap {}, hheaLineGap {}; both must be 0",
                r.typo_gap, r.hhea_gap
            ),
        ),
        dogray: Cell::require(
            r.hhea_gap == 0,
            format!(
                "hheaLineGap {} must be 0 (typoLineGap {} holds the line spacing)",
                r.hhea_gap, r.typo_gap
            ),
        ),
    });

    let fam_ymax = infos.iter().map(|i| i.y_max).max().unwrap_or(r.y_max);
    let fam_ymin = infos.iter().map(|i| i.y_min).min().unwrap_or(r.y_min);
    rows.push(Row {
        requirement: "winAscent / winDescent",
        google: Cell::require(
            r.win_asc >= fam_ymax && r.win_desc >= fam_ymin.abs(),
            format!(
                "{}/{} must cover family yMax/|yMin| {}/{}",
                r.win_asc,
                r.win_desc,
                fam_ymax,
                fam_ymin.abs()
            ),
        ),
        dogray: Cell::require(
            r.win_asc == r.hhea_asc && r.win_desc == r.hhea_desc.abs(),
            format!(
                "{}/{} must equal hheaAscender/|hheaDescender| {}/{}",
                r.win_asc,
                r.win_desc,
                r.hhea_asc,
                r.hhea_desc.abs()
            ),
        ),
    });

    let agrave = infos.iter().filter_map(|i| i.agrave_top).reduce(f64::max);
    rows.push(Row {
        requirement: "typoAscender above /Agrave",
        google: match agrave {
            Some(top) => Cell::require(
                r.typo_asc as f64 >= top,
                format!(
                    "typoAscender {} vs tallest Agrave {}",
                    r.typo_asc,
                    top.ceil()
                ),
            ),
            None => Cell::na(),
        },
        dogray: Cell::na(),
    });

    let cap_src = match r.cap_glyph {
        Some(c) => format!("top of /{c}, lowest of H T Z"),
        None => "from OS/2.sCapHeight".to_string(),
    };
    rows.push(Row {
        requirement: "typoAscender = cap height",
        google: Cell::na(),
        dogray: Cell::require(
            (r.typo_asc - r.cap_height).abs() <= measure_tol,
            format!(
                "typoAscender {} vs cap height {} ({cap_src} in {})",
                r.typo_asc, r.cap_height, r.name
            ),
        ),
    });

    let desc_src = match r.descender_glyph {
        Some(c) => format!("bottom of /{c}, lowest of g j p q y"),
        None => "caps-only: -UPM/5".to_string(),
    };
    rows.push(Row {
        requirement: "typoDescender vs descender",
        google: Cell::recommend(
            r.typo_desc <= r.descender,
            format!(
                "typoDescender {} should be at or below descender {} ({desc_src})",
                r.typo_desc, r.descender
            ),
        ),
        dogray: Cell::require(
            (r.typo_desc - r.descender).abs() <= measure_tol,
            format!(
                "typoDescender {} must equal descender {} ({desc_src} in {})",
                r.typo_desc, r.descender, r.name
            ),
        ),
    });

    let typo_space_above = r.typo_asc - r.cap_height;
    let hhea_space_above = r.hhea_asc - r.cap_height;
    let raised_for_agrave = agrave.is_some_and(|top| (r.typo_asc as f64 - top.ceil()).abs() <= 1.0);
    rows.push(Row {
        requirement: "Caps centred in the line",
        google: Cell::recommend(
            (typo_space_above - r.typo_desc.abs()).abs() <= tol || raised_for_agrave,
            format!(
                "{} above caps vs {} below baseline (typo){}",
                typo_space_above,
                r.typo_desc.abs(),
                if raised_for_agrave {
                    "; ascender raised to clear Agrave"
                } else {
                    ""
                }
            ),
        ),
        dogray: Cell::require(
            (hhea_space_above - r.hhea_desc.abs()).abs() <= tol,
            format!(
                "{} above caps vs {} below baseline (hhea)",
                hhea_space_above,
                r.hhea_desc.abs()
            ),
        ),
    });

    rows.push(Row {
        requirement: "Typo line = hhea line",
        google: Cell::na(),
        dogray: Cell::require(
            (r.typo_line() - r.hhea_line()).abs() <= 2,
            format!("typo line {} vs hhea line {}", r.typo_line(), r.hhea_line()),
        ),
    });

    let ratio = r.hhea_line() as f64 / r.upm as f64;
    let glyph_span = r.cap_height + r.descender.abs();
    let multiplier = if glyph_span > 0 {
        r.typo_line() as f64 / glyph_span as f64
    } else {
        0.0
    };
    rows.push(Row {
        requirement: "Line height",
        google: Cell::recommend(
            (1.2..=1.3).contains(&((ratio * 100.0).round() / 100.0)),
            format!(
                "{:.0}% of UPM; recommended 120-130% (may be higher for non-Latin primary scripts)",
                ratio * 100.0
            ),
        ),
        dogray: Cell {
            verdict: Verdict::Ok,
            detail: format!("{multiplier:.2}× (cap height + |descender|); Dogray default is 1.40×"),
        },
    });

    Evaluation { rows }
}

impl Evaluation {
    fn missing(&self, strategy: Strategy) -> usize {
        self.rows
            .iter()
            .filter(|row| strategy(row).verdict == Verdict::Fail)
            .count()
    }

    fn matches(&self, strategy: Strategy) -> bool {
        self.missing(strategy) == 0
    }

    fn table(&self) -> String {
        let mut out = String::from("| Requirement | Google Fonts | Dogray |\n|---|---|---|\n");
        // Literal pipes (|yMin|) would split cells, so use U+2223 instead.
        let cell = |s: &str| s.replace('|', "∣");
        for row in &self.rows {
            out.push_str(&format!(
                "| {} | {} {} | {} {} |\n",
                row.requirement,
                row.google.symbol(),
                cell(&row.google.detail),
                row.dogray.symbol(),
                cell(&row.dogray.detail)
            ));
        }
        out
    }

    fn into_statuses(self, infos: &[FontInfo], r: &FontInfo) -> Vec<Status> {
        let google = self.matches(|row| &row.google);
        let dogray = self.matches(|row| &row.dogray);
        let values = format!(
            "Values (from {}): typo {}/{}/{}, hhea {}/{}/{}, win {}/{}, UPM {}, {} font(s) checked.",
            r.name,
            r.typo_asc,
            r.typo_desc,
            r.typo_gap,
            r.hhea_asc,
            r.hhea_desc,
            r.hhea_gap,
            r.win_asc,
            r.win_desc,
            r.upm,
            infos.len()
        );
        let mut statuses = vec![];
        let matched: Option<(&str, Strategy)> = match (google, dogray) {
            (true, false) => Some(("Google Fonts", |row| &row.google)),
            (false, true) => Some(("Dogray", |row| &row.dogray)),
            _ => None,
        };
        match matched {
            Some((name, strategy)) => {
                statuses.push(Status::info(
                    &format!("{}-strategy", name.to_lowercase().replace(' ', "-")),
                    &format!("Vertical metrics follow the **{name}** strategy.\n\n{values}\n\n{}", self.table()),
                ));
                for row in self.rows.iter().filter(|row| strategy(row).verdict == Verdict::Warn) {
                    statuses.push(Status::warn(
                        "strategy-recommendation",
                        &format!("{name} strategy recommendation not met ({}): {}", row.requirement, strategy(row).detail),
                    ));
                }
            }
            None => statuses.push(Status::fail(
                "no-strategy",
                &format!(
                    "Vertical metrics fit neither the Google Fonts nor the Dogray strategy. Requirements not met: Google Fonts {}, Dogray {} (marked ❌).\n\n{values}\n\n{}",
                    self.missing(|row| &row.google),
                    self.missing(|row| &row.dogray),
                    self.table()
                ),
            )),
        }
        statuses
    }
}
