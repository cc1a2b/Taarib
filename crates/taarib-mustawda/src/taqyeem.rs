//! Ratings and reports submitted from the application, signed and rate-limited.

use jiff::Timestamp;
use serde::{Deserialize, Serialize, Serializer};
use taarib_khatm::{MiftahAam, MiftahKhass};
use taarib_mustalahat::musahim::MusahimId;
use taarib_mustalahat::ruqaa::{RuqaaId, RuqaaRevision};

/// The signed-payload format version this build writes and reads.
pub const ISDAR_IRSAL: u32 = 1;

/// The longest comment a rating may carry, in characters.
pub const AQSA_TUL_TAALIQ: usize = 500;

/// The shortest description a report is accepted with, in characters.
pub const ADNA_TUL_WASF: usize = 10;

/// The longest description a report may carry, in characters.
pub const AQSA_TUL_WASF: usize = 2_000;

/// Seconds between two ratings of the same patch lineage by the same rater.
///
/// A rating is an opinion about a published revision, not a stream, so one a
/// day is already more than the thing being rated changes.
pub const FASIL_TAQYEEM_LIL_RUQAA: i64 = 86_400;

/// Seconds between two reports about the same patch lineage by the same
/// reporter.
///
/// Shorter than a rating's, because a second report often carries information
/// the first one could not, and longer than a minute so a queue cannot be
/// buried under one patch.
pub const FASIL_BALAGH_LIL_RUQAA: i64 = 21_600;

/// Seconds between any two submissions of any kind from the same contributor.
///
/// A floor on the whole submission path, so an automated client cannot empty a
/// generated opinion set into the registry in one pass.
pub const FASIL_LIL_MUSAHIM: i64 = 60;

/// Domain separator for a signed rating.
const FASIL_MATN_TAQYEEM: &[u8] = b"taarib.taqyeem.v1\0";

/// Domain separator for a signed report.
const FASIL_MATN_BALAGH: &[u8] = b"taarib.balagh.v1\0";

/// The result type of every fallible submission operation.
pub type NatijatIrsal<T> = Result<T, KhataIrsal>;

/// Failures of building, reading or verifying a submission.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum KhataIrsal {
    /// A rating comment is longer than the limit.
    #[error("a rating comment may be at most {aqsa} characters and this one is {tul}")]
    TaaliqTaweel {
        /// How long it is.
        tul: usize,
        /// The limit.
        aqsa: usize,
    },

    /// A report description is missing or too short to act on.
    #[error("a report description must be at least {adna} characters and this one is {tul}")]
    WasfQaseer {
        /// How long it is.
        tul: usize,
        /// The minimum.
        adna: usize,
    },

    /// A report description is longer than the limit.
    #[error("a report description may be at most {aqsa} characters and this one is {tul}")]
    WasfTaweel {
        /// How long it is.
        tul: usize,
        /// The limit.
        aqsa: usize,
    },

    /// A score outside the closed 1 to 5 range.
    #[error("a rating is 1 to 5 and this one is {qeema}")]
    DarajaKharijNitaq {
        /// The value offered.
        qeema: u8,
    },

    /// A timestamp that is not RFC 3339.
    #[error("{waqt} is not an RFC 3339 timestamp")]
    WaqtTalif {
        /// The value offered.
        waqt: String,
    },

    /// A payload written in a format version this build does not read.
    #[error("submission format version {wujid} is not the {maqru} this build reads")]
    IsdarMajhul {
        /// The version offered.
        wujid: u32,
        /// The version read.
        maqru: u32,
    },

    /// A field that is not the lowercase hex the format defines.
    #[error("the {haql} field is not the lowercase hex this format defines")]
    HaqlTalif {
        /// Which field.
        haql: &'static str,
    },

    /// A submitter identity that is not a key fingerprint.
    #[error("the submitter identity is not a 64-character key fingerprint")]
    HuwiyaTalifa,

    /// The payload names a different signing key than the one it is checked
    /// against.
    #[error("the payload declares a signing key other than the one it was checked against")]
    MiftahGhayrMutabiq,

    /// The signature does not verify.
    #[error("the signature does not verify against the submitter key")]
    TawqeeGhayrSalih,

    /// The bytes are not a submission this build can read.
    #[error("the submission could not be read: {sabab}")]
    BaytTalifa {
        /// Why.
        sabab: String,
    },
}

/// A score, closed at five values so no other number can be represented.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(into = "u8", try_from = "u8")]
pub enum DarajatTaqyeem {
    /// One star.
    Wahid,
    /// Two stars.
    Ithnan,
    /// Three stars.
    Thalatha,
    /// Four stars.
    Arbaa,
    /// Five stars.
    Khamsa,
}

impl DarajatTaqyeem {
    /// The score as the number the interface shows.
    #[must_use]
    pub const fn raqm(self) -> u8 {
        match self {
            Self::Wahid => 1,
            Self::Ithnan => 2,
            Self::Thalatha => 3,
            Self::Arbaa => 4,
            Self::Khamsa => 5,
        }
    }

    /// The score a number denotes, when it denotes one.
    #[must_use]
    pub const fn min_raqm(qeema: u8) -> Option<Self> {
        match qeema {
            1 => Some(Self::Wahid),
            2 => Some(Self::Ithnan),
            3 => Some(Self::Thalatha),
            4 => Some(Self::Arbaa),
            5 => Some(Self::Khamsa),
            _ => None,
        }
    }

    /// Whether the score is a complaint.
    #[must_use]
    pub const fn salbi(self) -> bool {
        matches!(self, Self::Wahid | Self::Ithnan)
    }

    /// The score's label, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::Wahid => "سيئة",
            Self::Ithnan => "دون المتوسط",
            Self::Thalatha => "مقبولة",
            Self::Arbaa => "جيدة",
            Self::Khamsa => "ممتازة",
        }
    }

    /// The same label in English.
    #[must_use]
    pub const fn wasf_injilizi(self) -> &'static str {
        match self {
            Self::Wahid => "Poor",
            Self::Ithnan => "Below average",
            Self::Thalatha => "Acceptable",
            Self::Arbaa => "Good",
            Self::Khamsa => "Excellent",
        }
    }
}

impl From<DarajatTaqyeem> for u8 {
    fn from(daraja: DarajatTaqyeem) -> Self {
        daraja.raqm()
    }
}

impl TryFrom<u8> for DarajatTaqyeem {
    type Error = KhataIrsal;

    fn try_from(qeema: u8) -> Result<Self, Self::Error> {
        Self::min_raqm(qeema).ok_or(KhataIrsal::DarajaKharijNitaq { qeema })
    }
}

/// Why a patch is being reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SababBalagh {
    /// It installs and the game does not run, or the text does not appear.
    LaTaamal,
    /// It applied to a build it does not fit.
    GhayrMutawafiqa,
    /// Its coverage, method or authorship is not what it declares.
    Mudallila,
    /// It republishes somebody else's work against their licence.
    IntihakRukhsa,
    /// It carries something that is not a translation.
    Khabith,
    /// Something the list does not name.
    Ukhra,
}

impl SababBalagh {
    /// The stable discriminant the signed encoding carries.
    #[must_use]
    pub const fn bayt(self) -> u8 {
        match self {
            Self::LaTaamal => 1,
            Self::GhayrMutawafiqa => 2,
            Self::Mudallila => 3,
            Self::IntihakRukhsa => 4,
            Self::Khabith => 5,
            Self::Ukhra => 6,
        }
    }

    /// The reason a discriminant denotes, when it denotes one.
    #[must_use]
    pub const fn min_bayt(qeema: u8) -> Option<Self> {
        match qeema {
            1 => Some(Self::LaTaamal),
            2 => Some(Self::GhayrMutawafiqa),
            3 => Some(Self::Mudallila),
            4 => Some(Self::IntihakRukhsa),
            5 => Some(Self::Khabith),
            6 => Some(Self::Ukhra),
            _ => None,
        }
    }

    /// Whether the report needs the owner's attention before the queue's turn.
    #[must_use]
    pub const fn fawriya(self) -> bool {
        matches!(self, Self::Khabith | Self::IntihakRukhsa)
    }

    /// The reason's label, in Arabic.
    #[must_use]
    pub const fn wasf_arabi(self) -> &'static str {
        match self {
            Self::LaTaamal => "لا تعمل",
            Self::GhayrMutawafiqa => "غير متوافقة مع النسخة",
            Self::Mudallila => "بياناتها مضلّلة",
            Self::IntihakRukhsa => "تنتهك رخصة عمل آخر",
            Self::Khabith => "تحتوي ما ليس ترجمة",
            Self::Ukhra => "سبب آخر",
        }
    }

    /// The same label in English.
    #[must_use]
    pub const fn wasf_injilizi(self) -> &'static str {
        match self {
            Self::LaTaamal => "Does not work",
            Self::GhayrMutawafiqa => "Applied to a build it does not fit",
            Self::Mudallila => "Its declared metadata is misleading",
            Self::IntihakRukhsa => "Republishes another work against its licence",
            Self::Khabith => "Carries something that is not a translation",
            Self::Ukhra => "Another reason",
        }
    }
}

/// A rating of one published revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Taqyeem {
    ruqaa: RuqaaId,
    murajaa: RuqaaRevision,
    daraja: DarajatTaqyeem,
    taaliq: Option<String>,
    muqayyim: MusahimId,
    waqt: String,
}

impl Taqyeem {
    /// Builds a rating, trimming an empty comment away rather than storing one.
    ///
    /// # Errors
    ///
    /// [`KhataIrsal::TaaliqTaweel`] when the comment exceeds
    /// [`AQSA_TUL_TAALIQ`], and [`KhataIrsal::WaqtTalif`] when `waqt` is not
    /// RFC 3339.
    pub fn jadeed(
        ruqaa: RuqaaId,
        murajaa: RuqaaRevision,
        daraja: DarajatTaqyeem,
        taaliq: Option<String>,
        muqayyim: MusahimId,
        waqt: String,
    ) -> NatijatIrsal<Self> {
        let taaliq = nazzif(taaliq)
            .map(|nass| {
                aqsa_tul(nass, AQSA_TUL_TAALIQ, |tul, aqsa| {
                    KhataIrsal::TaaliqTaweel { tul, aqsa }
                })
            })
            .transpose()?;
        lahza(&waqt)?;
        Ok(Self {
            ruqaa,
            murajaa,
            daraja,
            taaliq,
            muqayyim,
            waqt,
        })
    }

    /// The patch lineage.
    #[must_use]
    pub const fn ruqaa(&self) -> RuqaaId {
        self.ruqaa
    }

    /// The revision rated.
    #[must_use]
    pub const fn murajaa(&self) -> RuqaaRevision {
        self.murajaa
    }

    /// The score.
    #[must_use]
    pub const fn daraja(&self) -> DarajatTaqyeem {
        self.daraja
    }

    /// The comment, when there is one.
    #[must_use]
    pub fn taaliq(&self) -> Option<&str> {
        self.taaliq.as_deref()
    }

    /// Who rated.
    #[must_use]
    pub const fn muqayyim(&self) -> &MusahimId {
        &self.muqayyim
    }

    /// When, RFC 3339, as the caller supplied it.
    #[must_use]
    pub fn waqt(&self) -> &str {
        &self.waqt
    }

    /// Signs the rating with the rater's key.
    #[must_use]
    pub fn waqqi(&self, miftah: &MiftahKhass) -> TaqyeemMuwaqqa {
        let aam = miftah.aam().bayt();
        let tawqee = miftah.waqqi(&matn_taqyeem(self, &aam));
        TaqyeemMuwaqqa {
            taqyeem: self.clone(),
            miftah: aam,
            tawqee,
        }
    }
}

/// A report about one published revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Balagh {
    ruqaa: RuqaaId,
    murajaa: RuqaaRevision,
    sabab: SababBalagh,
    wasf: String,
    mublagh: MusahimId,
    waqt: String,
}

impl Balagh {
    /// Builds a report; the description is required, not optional.
    ///
    /// # Errors
    ///
    /// [`KhataIrsal::WasfQaseer`] below [`ADNA_TUL_WASF`],
    /// [`KhataIrsal::WasfTaweel`] above [`AQSA_TUL_WASF`], and
    /// [`KhataIrsal::WaqtTalif`] when `waqt` is not RFC 3339.
    pub fn jadeed(
        ruqaa: RuqaaId,
        murajaa: RuqaaRevision,
        sabab: SababBalagh,
        wasf: &str,
        mublagh: MusahimId,
        waqt: String,
    ) -> NatijatIrsal<Self> {
        let wasf = wasf.trim().to_owned();
        let tul = wasf.chars().count();
        if tul < ADNA_TUL_WASF {
            return Err(KhataIrsal::WasfQaseer {
                tul,
                adna: ADNA_TUL_WASF,
            });
        }
        let wasf = aqsa_tul(wasf, AQSA_TUL_WASF, |tul, aqsa| KhataIrsal::WasfTaweel {
            tul,
            aqsa,
        })?;
        lahza(&waqt)?;
        Ok(Self {
            ruqaa,
            murajaa,
            sabab,
            wasf,
            mublagh,
            waqt,
        })
    }

    /// The patch lineage.
    #[must_use]
    pub const fn ruqaa(&self) -> RuqaaId {
        self.ruqaa
    }

    /// The revision reported.
    #[must_use]
    pub const fn murajaa(&self) -> RuqaaRevision {
        self.murajaa
    }

    /// Why.
    #[must_use]
    pub const fn sabab(&self) -> SababBalagh {
        self.sabab
    }

    /// The written description.
    #[must_use]
    pub fn wasf(&self) -> &str {
        &self.wasf
    }

    /// Who reported.
    #[must_use]
    pub const fn mublagh(&self) -> &MusahimId {
        &self.mublagh
    }

    /// When, RFC 3339, as the caller supplied it.
    #[must_use]
    pub fn waqt(&self) -> &str {
        &self.waqt
    }

    /// Signs the report with the reporter's key.
    #[must_use]
    pub fn waqqi(&self, miftah: &MiftahKhass) -> BalaghMuwaqqa {
        let aam = miftah.aam().bayt();
        let tawqee = miftah.waqqi(&matn_balagh(self, &aam));
        BalaghMuwaqqa {
            balagh: self.clone(),
            miftah: aam,
            tawqee,
        }
    }
}

// transport is the registry submission path's job; these types are the payload only
/// A rating carrying the rater's key and their signature over it.
#[derive(Debug, Clone)]
pub struct TaqyeemMuwaqqa {
    taqyeem: Taqyeem,
    miftah: [u8; 32],
    tawqee: [u8; 64],
}

impl TaqyeemMuwaqqa {
    /// Reads a signed rating and verifies it against `miftah`.
    ///
    /// # Errors
    ///
    /// [`KhataIrsal::BaytTalifa`] when the bytes do not parse,
    /// [`KhataIrsal::IsdarMajhul`] for another format version,
    /// [`KhataIrsal::HaqlTalif`] for a malformed key or signature,
    /// [`KhataIrsal::HuwiyaTalifa`] for an identity that is not a fingerprint,
    /// [`KhataIrsal::DarajaKharijNitaq`] for a score outside 1 to 5,
    /// [`KhataIrsal::MiftahGhayrMutabiq`] when the declared key is not
    /// `miftah`, [`KhataIrsal::TawqeeGhayrSalih`] when the signature does not
    /// verify, and every error [`Taqyeem::jadeed`] returns.
    pub fn min_bayt(bayt: &[u8], miftah: &MiftahAam) -> NatijatIrsal<Self> {
        let khaam: TaqyeemKhaam =
            serde_json::from_slice(bayt).map_err(|khata| KhataIrsal::BaytTalifa {
                sabab: khata.to_string(),
            })?;
        isdar_maqru(khaam.isdar)?;
        let (miftah_muallan, tawqee) = khatm(&khaam.miftah, &khaam.tawqee, miftah)?;
        let daraja =
            DarajatTaqyeem::min_raqm(khaam.daraja).ok_or(KhataIrsal::DarajaKharijNitaq {
                qeema: khaam.daraja,
            })?;
        let taqyeem = Taqyeem::jadeed(
            khaam.ruqaa,
            khaam.murajaa,
            daraja,
            khaam.taaliq,
            huwiya(&khaam.muqayyim)?,
            khaam.waqt,
        )?;
        let muwaqqa = Self {
            taqyeem,
            miftah: miftah_muallan,
            tawqee,
        };
        if !muwaqqa.tahaqquq(miftah) {
            return Err(KhataIrsal::TawqeeGhayrSalih);
        }
        Ok(muwaqqa)
    }

    /// Whether the signature verifies against `miftah` over the canonical form.
    #[must_use]
    pub fn tahaqquq(&self, miftah: &MiftahAam) -> bool {
        miftah.bayt() == self.miftah
            && miftah.tahaqquq(&matn_taqyeem(&self.taqyeem, &self.miftah), &self.tawqee)
    }

    /// The rating itself.
    #[must_use]
    pub const fn taqyeem(&self) -> &Taqyeem {
        &self.taqyeem
    }

    /// The signing key the payload declares.
    #[must_use]
    pub const fn miftah(&self) -> [u8; 32] {
        self.miftah
    }

    /// The signature.
    #[must_use]
    pub const fn tawqee(&self) -> [u8; 64] {
        self.tawqee
    }
}

impl Serialize for TaqyeemMuwaqqa {
    fn serialize<S: Serializer>(&self, musalsil: S) -> Result<S::Ok, S::Error> {
        TaqyeemKhaam {
            isdar: ISDAR_IRSAL,
            ruqaa: self.taqyeem.ruqaa,
            murajaa: self.taqyeem.murajaa,
            daraja: self.taqyeem.daraja.raqm(),
            taaliq: self.taqyeem.taaliq.clone(),
            muqayyim: self.taqyeem.muqayyim.to_string(),
            waqt: self.taqyeem.waqt.clone(),
            miftah: nass_hex(&self.miftah),
            tawqee: nass_hex(&self.tawqee),
        }
        .serialize(musalsil)
    }
}

/// A report carrying the reporter's key and their signature over it.
#[derive(Debug, Clone)]
pub struct BalaghMuwaqqa {
    balagh: Balagh,
    miftah: [u8; 32],
    tawqee: [u8; 64],
}

impl BalaghMuwaqqa {
    /// Reads a signed report and verifies it against `miftah`.
    ///
    /// # Errors
    ///
    /// [`KhataIrsal::BaytTalifa`] when the bytes do not parse,
    /// [`KhataIrsal::IsdarMajhul`] for another format version,
    /// [`KhataIrsal::HaqlTalif`] for a malformed key or signature,
    /// [`KhataIrsal::HuwiyaTalifa`] for an identity that is not a fingerprint,
    /// [`KhataIrsal::MiftahGhayrMutabiq`] when the declared key is not
    /// `miftah`, [`KhataIrsal::TawqeeGhayrSalih`] when the signature does not
    /// verify, and every error [`Balagh::jadeed`] returns.
    pub fn min_bayt(bayt: &[u8], miftah: &MiftahAam) -> NatijatIrsal<Self> {
        let khaam: BalaghKhaam =
            serde_json::from_slice(bayt).map_err(|khata| KhataIrsal::BaytTalifa {
                sabab: khata.to_string(),
            })?;
        isdar_maqru(khaam.isdar)?;
        let (miftah_muallan, tawqee) = khatm(&khaam.miftah, &khaam.tawqee, miftah)?;
        let balagh = Balagh::jadeed(
            khaam.ruqaa,
            khaam.murajaa,
            khaam.sabab,
            &khaam.wasf,
            huwiya(&khaam.mublagh)?,
            khaam.waqt,
        )?;
        let muwaqqa = Self {
            balagh,
            miftah: miftah_muallan,
            tawqee,
        };
        if !muwaqqa.tahaqquq(miftah) {
            return Err(KhataIrsal::TawqeeGhayrSalih);
        }
        Ok(muwaqqa)
    }

    /// Whether the signature verifies against `miftah` over the canonical form.
    #[must_use]
    pub fn tahaqquq(&self, miftah: &MiftahAam) -> bool {
        miftah.bayt() == self.miftah
            && miftah.tahaqquq(&matn_balagh(&self.balagh, &self.miftah), &self.tawqee)
    }

    /// The report itself.
    #[must_use]
    pub const fn balagh(&self) -> &Balagh {
        &self.balagh
    }

    /// The signing key the payload declares.
    #[must_use]
    pub const fn miftah(&self) -> [u8; 32] {
        self.miftah
    }

    /// The signature.
    #[must_use]
    pub const fn tawqee(&self) -> [u8; 64] {
        self.tawqee
    }
}

impl Serialize for BalaghMuwaqqa {
    fn serialize<S: Serializer>(&self, musalsil: S) -> Result<S::Ok, S::Error> {
        BalaghKhaam {
            isdar: ISDAR_IRSAL,
            ruqaa: self.balagh.ruqaa,
            murajaa: self.balagh.murajaa,
            sabab: self.balagh.sabab,
            wasf: self.balagh.wasf.clone(),
            mublagh: self.balagh.mublagh.to_string(),
            waqt: self.balagh.waqt.clone(),
            miftah: nass_hex(&self.miftah),
            tawqee: nass_hex(&self.tawqee),
        }
        .serialize(musalsil)
    }
}

/// Which submission kind a rate-limit check is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NawIrsal {
    /// A rating.
    Taqyeem,
    /// A report.
    Balagh,
}

impl NawIrsal {
    /// The minimum interval between two submissions of this kind about one
    /// patch lineage, in seconds.
    #[must_use]
    pub const fn fasil_lil_ruqaa(self) -> i64 {
        match self {
            Self::Taqyeem => FASIL_TAQYEEM_LIL_RUQAA,
            Self::Balagh => FASIL_BALAGH_LIL_RUQAA,
        }
    }
}

/// The last-submission instants the caller keeps.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AakhirIrsal {
    /// When this contributor last submitted about this patch lineage, RFC 3339.
    pub lil_ruqaa: Option<String>,
    /// When this contributor last submitted anything at all, RFC 3339.
    pub lil_musahim: Option<String>,
}

/// Whether a submission is allowed at the instant the caller supplied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HalatMuaddal {
    /// Allowed.
    Masmuh,
    /// Too soon after the last submission about this patch.
    MamnuLilRuqaa {
        /// Seconds still to wait.
        mutabaqqi: i64,
    },
    /// Too soon after this contributor's last submission of any kind.
    MamnuLilMusahim {
        /// Seconds still to wait.
        mutabaqqi: i64,
    },
}

impl HalatMuaddal {
    /// Whether the submission may go ahead.
    #[must_use]
    pub const fn masmuh(self) -> bool {
        matches!(self, Self::Masmuh)
    }

    /// Seconds still to wait, zero when allowed.
    #[must_use]
    pub const fn mutabaqqi(self) -> i64 {
        match self {
            Self::Masmuh => 0,
            Self::MamnuLilRuqaa { mutabaqqi } | Self::MamnuLilMusahim { mutabaqqi } => mutabaqqi,
        }
    }

    /// The state as one line, in Arabic.
    #[must_use]
    pub fn wasf_arabi(self) -> String {
        match self {
            Self::Masmuh => "يمكن الإرسال الآن.".to_owned(),
            Self::MamnuLilRuqaa { mutabaqqi } => {
                format!("أرسلت رأيك في هذه الرقعة قريبًا؛ انتظر {mutabaqqi} ثانية.")
            },
            Self::MamnuLilMusahim { mutabaqqi } => {
                format!("أرسلت للتو؛ انتظر {mutabaqqi} ثانية قبل الإرسال مرة أخرى.")
            },
        }
    }

    /// The same line in English.
    #[must_use]
    pub fn wasf_injilizi(self) -> String {
        match self {
            Self::Masmuh => "You can submit now.".to_owned(),
            Self::MamnuLilRuqaa { mutabaqqi } => {
                format!("You submitted about this patch recently; wait {mutabaqqi} seconds.")
            },
            Self::MamnuLilMusahim { mutabaqqi } => {
                format!("You just submitted; wait {mutabaqqi} seconds before submitting again.")
            },
        }
    }
}

/// Whether a submission is allowed, from a caller-supplied instant and the
/// caller's record of the last ones. No clock is read here.
///
/// # Errors
///
/// [`KhataIrsal::WaqtTalif`] when `al_aan` or either recorded instant is not
/// RFC 3339.
pub fn fahs_muaddal(
    al_aan: &str,
    aakhir: &AakhirIrsal,
    naw: NawIrsal,
) -> NatijatIrsal<HalatMuaddal> {
    let al_aan = lahza(al_aan)?;
    if let Some(sabiq) = aakhir.lil_ruqaa.as_deref() {
        let mutabaqqi = mutabaqqi(al_aan, lahza(sabiq)?, naw.fasil_lil_ruqaa());
        if mutabaqqi > 0 {
            return Ok(HalatMuaddal::MamnuLilRuqaa { mutabaqqi });
        }
    }
    if let Some(sabiq) = aakhir.lil_musahim.as_deref() {
        let mutabaqqi = mutabaqqi(al_aan, lahza(sabiq)?, FASIL_LIL_MUSAHIM);
        if mutabaqqi > 0 {
            return Ok(HalatMuaddal::MamnuLilMusahim { mutabaqqi });
        }
    }
    Ok(HalatMuaddal::Masmuh)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TaqyeemKhaam {
    isdar: u32,
    ruqaa: RuqaaId,
    murajaa: RuqaaRevision,
    daraja: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    taaliq: Option<String>,
    muqayyim: String,
    waqt: String,
    miftah: String,
    tawqee: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BalaghKhaam {
    isdar: u32,
    ruqaa: RuqaaId,
    murajaa: RuqaaRevision,
    sabab: SababBalagh,
    wasf: String,
    mublagh: String,
    waqt: String,
    miftah: String,
    tawqee: String,
}

// canonical form both signer and verifier reproduce: domain sep, LE length-prefixed
fn matn_taqyeem(taqyeem: &Taqyeem, miftah: &[u8; 32]) -> Vec<u8> {
    let mut matn = Vec::new();
    matn.extend_from_slice(FASIL_MATN_TAQYEEM);
    matn.extend_from_slice(&ISDAR_IRSAL.to_le_bytes());
    matn.extend_from_slice(taqyeem.ruqaa.uuid().as_bytes());
    matn.extend_from_slice(&taqyeem.murajaa.qeema().to_le_bytes());
    matn.push(taqyeem.daraja.raqm());
    ikhtiyari(&mut matn, taqyeem.taaliq.as_deref());
    lp(&mut matn, taqyeem.muqayyim.nass().as_bytes());
    lp(&mut matn, taqyeem.waqt.as_bytes());
    matn.extend_from_slice(miftah);
    matn
}

fn matn_balagh(balagh: &Balagh, miftah: &[u8; 32]) -> Vec<u8> {
    let mut matn = Vec::new();
    matn.extend_from_slice(FASIL_MATN_BALAGH);
    matn.extend_from_slice(&ISDAR_IRSAL.to_le_bytes());
    matn.extend_from_slice(balagh.ruqaa.uuid().as_bytes());
    matn.extend_from_slice(&balagh.murajaa.qeema().to_le_bytes());
    matn.push(balagh.sabab.bayt());
    lp(&mut matn, balagh.wasf.as_bytes());
    lp(&mut matn, balagh.mublagh.nass().as_bytes());
    lp(&mut matn, balagh.waqt.as_bytes());
    matn.extend_from_slice(miftah);
    matn
}

fn ikhtiyari(matn: &mut Vec<u8>, qeema: Option<&str>) {
    match qeema {
        Some(nass) => {
            matn.push(1);
            lp(matn, nass.as_bytes());
        },
        None => matn.push(0),
    }
}

fn lp(matn: &mut Vec<u8>, bayt: &[u8]) {
    matn.extend_from_slice(&tul(bayt.len()).to_le_bytes());
    matn.extend_from_slice(bayt);
}

fn tul(adad: usize) -> u64 {
    u64::try_from(adad).unwrap_or(u64::MAX)
}

fn nazzif(nass: Option<String>) -> Option<String> {
    let mahdhub = nass?.trim().to_owned();
    (!mahdhub.is_empty()).then_some(mahdhub)
}

fn aqsa_tul(
    nass: String,
    aqsa: usize,
    tajawuz: impl FnOnce(usize, usize) -> KhataIrsal,
) -> NatijatIrsal<String> {
    let tul = nass.chars().count();
    if tul > aqsa {
        return Err(tajawuz(tul, aqsa));
    }
    Ok(nass)
}

const fn isdar_maqru(wujid: u32) -> NatijatIrsal<()> {
    if wujid == ISDAR_IRSAL {
        Ok(())
    } else {
        Err(KhataIrsal::IsdarMajhul {
            wujid,
            maqru: ISDAR_IRSAL,
        })
    }
}

fn huwiya(nass: &str) -> NatijatIrsal<MusahimId> {
    MusahimId::jadeed(nass).map_err(|_| KhataIrsal::HuwiyaTalifa)
}

fn khatm(
    miftah_nassi: &str,
    tawqee_nassi: &str,
    miftah: &MiftahAam,
) -> NatijatIrsal<([u8; 32], [u8; 64])> {
    let Some(muallan) = bayt_hex::<32>(miftah_nassi) else {
        return Err(KhataIrsal::HaqlTalif { haql: "miftah" });
    };
    let Some(tawqee) = bayt_hex::<64>(tawqee_nassi) else {
        return Err(KhataIrsal::HaqlTalif { haql: "tawqee" });
    };
    if muallan != miftah.bayt() {
        return Err(KhataIrsal::MiftahGhayrMutabiq);
    }
    Ok((muallan, tawqee))
}

fn nass_hex(bayt: &[u8]) -> String {
    use std::fmt::Write as _;
    bayt.iter().fold(
        String::with_capacity(bayt.len().saturating_mul(2)),
        |mut khraj, wahid| {
            let _ = write!(khraj, "{wahid:02x}");
            khraj
        },
    )
}

fn bayt_hex<const N: usize>(nass: &str) -> Option<[u8; N]> {
    // lowercase hex only, and exactly the declared width — the canonical form
    if nass.len() != N.checked_mul(2)? {
        return None;
    }
    if nass
        .bytes()
        .any(|wahid| !wahid.is_ascii_hexdigit() || wahid.is_ascii_uppercase())
    {
        return None;
    }
    let mut khraj = [0u8; N];
    let mut azwaj = nass.as_bytes().chunks_exact(2);
    for khana in &mut khraj {
        let nassi = std::str::from_utf8(azwaj.next()?).ok()?;
        *khana = u8::from_str_radix(nassi, 16).ok()?;
    }
    Some(khraj)
}

fn lahza(nass: &str) -> NatijatIrsal<Timestamp> {
    nass.parse::<Timestamp>()
        .map_err(|_| KhataIrsal::WaqtTalif {
            waqt: nass.to_owned(),
        })
}

fn mutabaqqi(al_aan: Timestamp, sabiq: Timestamp, fasil: i64) -> i64 {
    // a last submission dated in the future is clock skew, and never shortens the wait
    let munqadi = al_aan.as_second().saturating_sub(sabiq.as_second()).max(0);
    fasil.saturating_sub(munqadi)
}
