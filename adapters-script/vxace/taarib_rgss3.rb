# encoding: utf-8
#==============================================================================
# taarib_rgss3 — the Win32API binding to Taarib's native ABI, for RPG Maker
# VX Ace.
#
# This script is injected into Game.rgss3a's script list by the Phase 10
# patcher. It is the only way the VX Ace adapter reaches the engine, and it
# binds the C surface of taarib_jisr.dll exactly as
# crates/taarib-jisr/src/anwa.rs froze it.
#
# The interpreter, stated honestly
# --------------------------------
# RGSS3 embeds Ruby 1.9.2 with almost no standard library, and its only
# foreign-function mechanism is the player's own Win32API class. That is a
# hard boundary, and this file is written inside it:
#
#   * No fiddle, no dl, no ffi gem — none of them exist here.
#   * No keyword arguments (Ruby 2.0); options travel as a Hash.
#   * No String#byteslice (1.9.3); substrings are taken with String#[] on
#     buffers forced to BINARY encoding, where characters are bytes.
#   * No pack endian modifiers "V<"/"Q<" (1.9.3); native "s" and "Q" are
#     used for signed 16-bit and 64-bit fields, which is correct here and
#     only here because the one platform this runs on is little-endian x86.
#   * No stabby lambdas, no Module#prepend, no frozen_string_literal magic
#     comment — the last is parsed as an ordinary comment on 1.9 and relying
#     on it would mean relying on behaviour that does not exist.
#
# The platform, stated honestly
# -----------------------------
# 32-bit Windows, and nothing else, because RPG Maker VX Ace runs nowhere
# else. Every pointer and every usize below is 32 bits, packed "V". There is
# no pretence of portability in this file; portability lives on the other
# side of the C ABI.
#
# What Win32API can and cannot pass
# ---------------------------------
# Win32API marshals exactly three shapes: an integer ("i"/"l", 32 bits), a
# String ("p", passed as the pointer to the String's own byte buffer), and
# void ("v"). It cannot pass a struct by value, cannot pass a float, and
# cannot dereference a returned pointer. Every structure therefore crosses
# as a String built with Array#pack and read back with String#unpack, using
# the format tables below, which are derived field by field from anwa.rs.
# Because "p" hands the callee the String's real buffer, an out-parameter is
# simply a pre-sized String the DLL writes into and Ruby then unpacks — the
# same memory, no copy.
#
# Two consequences of those limits are worth naming rather than hiding:
#
#   * A structure that contains a pointer to another buffer (the layout
#     request points at the text; the layout buffer points at the glyph
#     array) needs the raw address of a Ruby String. The documented trick is
#     [str].pack("P").unpack("V")[0] — "P" packs the address of the
#     String's buffer. The address is valid while the String object stays
#     referenced and un-resized; MRI 1.9's garbage collector does not move
#     objects, so a held reference is sufficient. Every such buffer is kept
#     referenced for the full duration of the call that uses it.
#
#   * The one float that crosses by value (taarib_khatt_qiyasat's size) is
#     passed as its IEEE-754 bit pattern in an "i" slot:
#     [f].pack("e").unpack("l")[0]. On i686, cdecl and stdcall both pass a
#     32-bit float in a 4-byte stack slot, so the bits land exactly where
#     the callee reads a float from. This is ugly, it is the only way
#     Win32API can do it, and it is correct only because the platform is
#     fixed.
#
# The calling convention, stated honestly
# ---------------------------------------
# taarib_jisr exports extern "C", which on i686 Windows is cdecl.
# Win32API nominally assumes stdcall. The player's Win32API implementation
# (modelled on Ruby 1.8's) saves the stack pointer before pushing arguments
# and restores it after the call rather than trusting the callee to clean
# up — which is the only reason two decades of RGSS scripts calling
# msvcrt's cdecl functions have ever worked, and it is what this binding
# relies on too. There is no parameter to declare the convention; the
# dependence is documented here instead of being left as a mystery for
# whoever ports this next.
#
# Errors
# ------
# Any entry point that returns non-zero raises Taarib::KhataTaarib, carrying
# the permanent code (TAARIB-E-2500), the Arabic sentence, the English
# sentence, and the next-action number, read from the library's thread-local
# stash before any other call can clear it. The exception's message is the
# Arabic sentence.
#==============================================================================

module Taarib
  # The ABI major this binding matches. Any other major is refused at load:
  # the pack formats below are the layouts of major 1, and calling a
  # different major with them corrupts memory instead of failing.
  ISDAR_KABIR = 1

  #--------------------------------------------------------------------------
  # Status codes — crates/taarib-jisr/src/khata_c.rs, verbatim.
  #--------------------------------------------------------------------------
  NAJAH                 = 0
  KHATA_AAM             = -1
  MUASHIR_BATIL         = -2
  MAQBAD_BATIL          = -3
  SIAT_QASIRA           = -4
  ISDAR_GHAYR_MUTAWAFIQ = -5
  TARMIZ_BATIL          = -6
  KHATT_MARFUD          = -7
  TASHKEEL_FASHIL       = -8
  DHAKIRA               = -9
  QEEMA_BATILA          = -10
  INHIYAR               = -11
  LAWHA_MUMTALIA        = -12
  GHAYR_MADUM           = -13
  GHAYR_MUHAYYAA        = -14

  #--------------------------------------------------------------------------
  # Flags — crates/taarib-jisr/src/anwa.rs, verbatim.
  #--------------------------------------------------------------------------
  HARF_ALAMA        = 1        # TaaribHarf#alam: a combining mark.
  SATR_AKHIR        = 1        # TaaribSatr#alam: last line of its paragraph.
  SATR_YAMEEN       = 2        # TaaribSatr#alam: base direction right-to-left.
  KHIYAR_HIWAR      = 1        # TaaribKhiyarat#alam: this text is dialogue.
  KHIYAR_SATR_WAHID = 2        # TaaribKhiyarat#alam: wrapping forbidden.
  TAKHTIT_YAMEEN    = 1        # layout is right-to-left overall.
  TAKHTIT_MAQSUS    = 2        # the overflow policy truncated.
  TAKHTIT_TAJAWUZ   = 4        # the text did not fit; the report says how.
  TAKHTIT_MAKHZAN   = 8        # served from the layout cache.
  USLUB_MAAIL       = 1        # span: italic.
  USLUB_DHARRA      = 2        # span: an opaque atom, never shaped.
  USLUB_KHATT       = 4        # span: sets a font index.
  USLUB_WAZN        = 8        # span: sets a weight.
  USLUB_HAJM        = 16       # span: sets a size.
  USLUB_LAWN        = 32       # span: sets a colour.

  # Option discriminants, as TaaribKhiyarat carries them.
  ITTIJAH_TILQAI = 0; ITTIJAH_YAMEEN = 1; ITTIJAH_YASAR = 2
  LUGHA_TILQAI = 0; LUGHA_ARABI = 1; LUGHA_FARISI = 2
  LUGHA_URDU = 3; LUGHA_LATINI = 4
  DABT_BILA = 0; DABT_MASAFAT = 1; DABT_KASHIDA = 2; DABT_KASHIDA_MASAFAT = 3
  MUHADHAHA_BIDAYA = 0; MUHADHAHA_NIHAYA = 1
  MUHADHAHA_WASAT = 2; MUHADHAHA_DABT = 3
  TASHKEEL_IBQA = 0; TASHKEEL_HADHF = 1; TASHKEEL_HIWAR = 2
  ARQAM_KAMA_HIYA = 0; ARQAM_LATINI = 1; ARQAM_ARABI = 2; ARQAM_FARISI = 3
  TAJAWUZ_BALLAGH = 0; TAJAWUZ_TAQLIS = 1; TAJAWUZ_IKHTISAR = 2
  NAMAT_TAGHTIYA = 0; NAMAT_MISAFA = 1

  #--------------------------------------------------------------------------
  # Pack formats — the load-bearing part of this file.
  #
  # Every format below is derived from anwa.rs on i686 (pointers and usize
  # are 32 bits, everything naturally aligned, no implicit padding in any of
  # these structures at this width). Directives:
  #   V = u32 / usize / pointer, little-endian
  #   v = u16, little-endian
  #   s = i16, native (little-endian on the only CPU this runs on)
  #   C = u8
  #   e = f32, little-endian
  #   Q = u64, native (little-endian likewise; "Q<" needs Ruby 1.9.3)
  #   a4 = four raw bytes (an OpenType tag)
  #
  # TaaribHarf — 24 bytes                     | field    | type | dir | off
  #   muarrif u32 V 0 | anqud u32 V 4 | s f32 e 8 | a f32 e 12
  #   taqaddum f32 e 16 | nitaq u16 v 20 | khatt u8 C 22 | alam u8 C 23
  HARF_FMT = "VVeeevCC".freeze
  HARF_HAJM = 24

  # TaaribSatr — 48 bytes
  #   awwal_harf u32 V 0 | adad_huruf u32 V 4 | bidayat_mantiqi u32 V 8
  #   nihayat_mantiqi u32 V 12 | asas f32 e 16 | bidaya f32 e 20
  #   ard f32 e 24 | irtifa f32 e 28 | suud f32 e 32 | hubut f32 e 36
  #   dabt f32 e 40 | alam u32 V 44
  SATR_FMT = "VVVVeeeeeeeV".freeze
  SATR_HAJM = 48

  # TaaribTaqreerTajawuz — 24 bytes
  #   ard f32 e 0 | ard_mutah f32 e 4 | irtifa f32 e 8 | irtifa_mutah f32 e 12
  #   awwal_satr u32 V 16 | adad_sutur u32 V 20
  TAJAWUZ_FMT = "eeeeVV".freeze
  TAJAWUZ_HAJM = 24

  # TaaribMakhzanTakhtit — 64 bytes on i686
  #   huruf ptr V 0 | siaat_huruf usize V 4 | adad_huruf usize V 8
  #   sutur ptr V 12 | siaat_sutur usize V 16 | adad_sutur usize V 20
  #   ard f32 e 24 | irtifa f32 e 28 | hajm f32 e 32 | alam u32 V 36
  #   tajawuz TaaribTaqreerTajawuz 40..63 (format appended below)
  MAKHZAN_FMT = ("VVVVVV" + "eee" + "V" + "eeeeVV").freeze
  MAKHZAN_HAJM = 64

  # TaaribQiyasNass — 24 bytes
  #   ard e 0 | irtifa e 4 | suud e 8 | hubut e 12
  #   adad_sutur u32 V 16 | hashw u32 V 20
  QIYAS_FMT = "eeeeVV".freeze
  QIYAS_HAJM = 24

  # TaaribNitaqUslub — 52 bytes
  #   bidaya u32 V 0 | tul u32 V 4 | lawn u32 V 8 | alam u32 V 12
  #   hajm f32 e 16 | tabaud f32 e 20 | izaha f32 e 24
  #   ard_dharra f32 e 28 | irtifa_dharra f32 e 32 | asas_dharra f32 e 36
  #   marja_dharra u32 V 40 | wazn u16 v 44 | id u16 v 46 | khatt u8 C 48
  #   hashw [u8;3] CCC 49..51
  NITAQ_FMT = "VVVVeeeeeeVvvCCCC".freeze
  NITAQ_HAJM = 52

  # TaaribSifa — 8 bytes
  #   wasm [u8;4] a4 0 | qeema u32 V 4
  SIFA_FMT = "a4V".freeze
  SIFA_HAJM = 8

  # TaaribKhiyarat — 56 bytes on i686
  #   sifat ptr V 0 | adad_sifat usize V 4 | ittijah u32 V 8 | lugha u32 V 12
  #   dabt u32 V 16 | muhadhaha u32 V 20 | tashkeel u32 V 24 | arqam u32 V 28
  #   tajawuz u32 V 32 | hajm_adna f32 e 36 | irtifa_satr f32 e 40
  #   tabaud_ahruf f32 e 44 | tabaud_kalimat f32 e 48 | alam u32 V 52
  KHIYARAT_FMT = ("VV" + "VVVVVVV" + "eeee" + "V").freeze
  KHIYARAT_HAJM = 56

  # TaaribTalab — 88 bytes on i686 (no padding at this width; the 64-bit
  # build pads before khiyarat, which is the other side's business, not
  # ours — one more reason these formats are 32-bit-only by declaration)
  #   nass ptr V 0 | tul_nass usize V 4 | nitaqat ptr V 8
  #   adad_nitaqat usize V 12 | silsila handle V 16 | hajm f32 e 20
  #   ard_mutah f32 e 24 | irtifa_mutah f32 e 28
  #   khiyarat TaaribKhiyarat 32..87 (format appended)
  TALAB_FMT = ("VVVVV" + "eee" + KHIYARAT_FMT).freeze
  TALAB_HAJM = 88

  # TaaribKhiyaratSiyaq — 12 bytes on i686
  #   mizaniyat_makhzan usize V 0 | adad_makhazin u32 V 4 | hashw u32 V 8
  SIYAQ_KHIYARAT_FMT = "VVV".freeze
  SIYAQ_KHIYARAT_HAJM = 12

  # TaaribQiyasatKhatt — 32 bytes
  #   suud e 0 | hubut e 4 | fajwa e 8 | irtifa_satr e 12
  #   uluw_kabital e 16 | uluw_saghir e 20 | wahdat u32 V 24 | hashw u32 V 28
  QIYASAT_KHATT_FMT = "eeeeeeVV".freeze
  QIYASAT_KHATT_HAJM = 32

  # TaaribMiftahShakl — 8 bytes
  #   muarrif u32 V 0 | hajm_rubi u16 v 4 | khatt u8 C 6 | bakat u8 C 7
  MIFTAH_FMT = "VvCC".freeze
  MIFTAH_HAJM = 8

  # TaaribMawdiShakl — 20 bytes
  #   taqaddum f32 e 0 | s u16 v 4 | a u16 v 6 | ard u16 v 8 | irtifa u16 v 10
  #   izaha_s i16 s 12 | izaha_a i16 s 14 | safha u16 v 16 | hashw u16 v 18
  MAWDI_FMT = "evvvvssvv".freeze
  MAWDI_HAJM = 20

  # TaaribSafha — 16 bytes on i686
  #   bayt ptr V 0 | tul usize V 4 | ard u16 v 8 | irtifa u16 v 10
  #   namat u32 V 12
  SAFHA_FMT = "VVvvV".freeze
  SAFHA_HAJM = 16

  # TaaribIhsaatLawha — 56 bytes
  #   isabat u64 Q 0 | ikhfaqat u64 Q 8 | ikhlaat u64 Q 16
  #   ahdath_namu u64 Q 24 | bayt u64 Q 32 | mizaniya u64 Q 40
  #   ashkal u32 V 48 | safahat u32 V 52
  IHSAAT_FMT = "QQQQQQVV".freeze
  IHSAAT_HAJM = 56

  # TaaribIhsaatKhazina — 48 bytes (appended to the ABI with
  # taarib_makhzan_ihsaat; awamir.rs rather than anwa.rs on the Rust side,
  # under the same layout law)
  #   isabat u64 Q 0 | ikhfaqat u64 Q 8 | ikhlaat u64 Q 16 | bayt u64 Q 24
  #   mizaniya u64 Q 32 | madkhalat u32 V 40 | hashw u32 V 44
  # Five Qs then two Vs, not six Qs: "QQQQQQ" also packs to 48 bytes and,
  # with hashw zeroed on this little-endian CPU, even unpacks the same
  # numbers — the wrong shape works until the padding is given a meaning,
  # then fails silently on the field that was absorbing it.
  IHSAAT_KHAZINA_FMT = "QQQQQVV".freeze
  IHSAAT_KHAZINA_HAJM = 48

  # The frozen next-action numbers taarib_khata_khutwa returns.
  KHUTWA_ASMA = {
    0 => :la_shay, 1 => :aada_muhawala, 2 => :aada_fahs_maktaba,
    3 => :aada_fahs_muharrik, 4 => :ikhtiyar_masar,
    5 => :ikhtiyar_khatt_aakhar, 6 => :fath_idadat, 7 => :fath_tashkhis,
    8 => :fath_taqreer_tajawuz, 9 => :fath_nusus, 10 => :tahdith_taarib,
    11 => :iadat_tarkib_ittar, 12 => :ilgha_tathbeet,
    13 => :iadat_mutabaqa_bina, 14 => :tahaqquq_salamat_luba,
    15 => :iblagh_lil_musahim, 16 => :iblagh_lil_malik,
    17 => :tahrir_masaha, 18 => :manh_salahiya
  }.freeze

  #--------------------------------------------------------------------------
  # Buffer helpers
  #--------------------------------------------------------------------------

  # A writable byte buffer of exactly +tul+ bytes, in BINARY encoding.
  #
  # BINARY is not cosmetic: this file carries an encoding: utf-8 magic
  # comment, so a bare "\0" * n literal would be a UTF-8 string, and once
  # the DLL writes bytes above 0x7F into it, String#[] would start counting
  # malformed characters instead of bytes and every record offset in the
  # unpack loops would drift. Forcing BINARY makes characters bytes again.
  def self.hijz(tul)
    ("\0" * tul).force_encoding("BINARY")
  end

  # The address of a String's byte buffer, as an integer.
  #
  # "P" packs a pointer to the String's buffer; unpacking that pointer as
  # "V" yields the address on this 32-bit process. The address stays valid
  # only while the String object is referenced and never resized — MRI 1.9
  # does not move objects, so no other precaution exists or is needed.
  # Callers keep the String alive for the whole call that consumes the
  # address; helpers below do that by holding them in local scope.
  def self.unwan(buffer)
    [buffer].pack("P").unpack("V")[0]
  end

  # A float's IEEE-754 bit pattern as a signed 32-bit integer, for the one
  # place a float must cross by value through an "i" slot.
  def self.awama_bit(qeema)
    [qeema.to_f].pack("e").unpack("l")[0]
  end

  # Verifies every pack format produces exactly the frozen byte size, once,
  # at script load. A one-character typo in a format string would otherwise
  # shear every later field into its neighbour and surface as glyphs drawn
  # at garbage positions — data-dependent, unreproducible, blamed on the
  # game. Failing load is the loud version.
  def self.tahaqqaq_ahjam
    fuhus = [
      ["TaaribHarf",           HARF_FMT,          HARF_HAJM],
      ["TaaribSatr",           SATR_FMT,          SATR_HAJM],
      ["TaaribTaqreerTajawuz", TAJAWUZ_FMT,       TAJAWUZ_HAJM],
      ["TaaribMakhzanTakhtit", MAKHZAN_FMT,       MAKHZAN_HAJM],
      ["TaaribQiyasNass",      QIYAS_FMT,         QIYAS_HAJM],
      ["TaaribNitaqUslub",     NITAQ_FMT,         NITAQ_HAJM],
      ["TaaribSifa",           SIFA_FMT,          SIFA_HAJM],
      ["TaaribKhiyarat",       KHIYARAT_FMT,      KHIYARAT_HAJM],
      ["TaaribTalab",          TALAB_FMT,         TALAB_HAJM],
      ["TaaribKhiyaratSiyaq",  SIYAQ_KHIYARAT_FMT, SIYAQ_KHIYARAT_HAJM],
      ["TaaribQiyasatKhatt",   QIYASAT_KHATT_FMT, QIYASAT_KHATT_HAJM],
      ["TaaribMiftahShakl",    MIFTAH_FMT,        MIFTAH_HAJM],
      ["TaaribMawdiShakl",     MAWDI_FMT,         MAWDI_HAJM],
      ["TaaribSafha",          SAFHA_FMT,         SAFHA_HAJM],
      ["TaaribIhsaatLawha",    IHSAAT_FMT,        IHSAAT_HAJM],
      ["TaaribIhsaatKhazina",  IHSAAT_KHAZINA_FMT, IHSAAT_KHAZINA_HAJM]
    ]
    fuhus.each do |ism, namat, mutawaqqa|
      sifr = []
      namat.scan(/([a-zA-Z])(\d*)/) do |harf, adad|
        takrar = adad.empty? ? 1 : adad.to_i
        if harf == "a"
          sifr << ("\0" * takrar)
        else
          takrar.times { sifr << 0 }
        end
      end
      fielisi = sifr.pack(namat).bytesize
      if fielisi != mutawaqqa
        raise "Taarib: #{ism} packs to #{fielisi} bytes but the frozen ABI " \
              "layout is #{mutawaqqa}; the format string is wrong and no " \
              "call may proceed."
      end
    end
  end

  tahaqqaq_ahjam

  #--------------------------------------------------------------------------
  # The error that crosses back into Ruby
  #--------------------------------------------------------------------------

  # Sentences for failures that carry no stashed error on the native side —
  # a null pointer, a stale handle, a caught panic are recorded there as a
  # bare code, and the retrieval calls honestly return empty strings. The
  # exception still owes its reader a sentence in both languages.
  JUMAL_IHTIYAT = {
    KHATA_AAM => ["فشلت العملية داخل محرك تعريب.",
                  "The operation failed inside the Taarib engine."],
    MUASHIR_BATIL => ["مُرِّر مؤشر فارغ إلى جسر تعريب.",
                      "A null pointer was passed to the Taarib bridge."],
    MAQBAD_BATIL => ["استُخدم مقبض بعد تدميره أو لم يصدر عن هذه المكتبة.",
                     "A handle was used after destruction, or was never " \
                     "issued by this library."],
    SIAT_QASIRA => ["السعة الممنوحة أصغر من المطلوب.",
                    "The supplied capacity is smaller than required."],
    ISDAR_GHAYR_MUTAWAFIQ => ["إصدار مكتبة تعريب لا يطابق هذا الرابط.",
                              "The Taarib library's version does not " \
                              "match this binding."],
    TARMIZ_BATIL => ["النص المُمرَّر ليس UTF-8 صالحًا.",
                     "The supplied text is not valid UTF-8."],
    KHATT_MARFUD => ["رُفض الخط لأنه لا يحمل جداول العربية المطلوبة.",
                     "The font was rejected: it lacks the required Arabic " \
                     "tables."],
    TASHKEEL_FASHIL => ["لم يُنتج التشكيل شيئًا لنص غير فارغ.",
                        "Shaping produced nothing for non-empty text."],
    DHAKIRA => ["فشل حجز الذاكرة داخل المكتبة.",
                "An allocation failed inside the library."],
    QEEMA_BATILA => ["قيمة وسيطة غير صالحة للاستخدام.",
                     "An argument's value is not usable."],
    INHIYAR => ["التُقط انهيار داخلي عند الحدود؛ هذا خلل يستحق البلاغ.",
                "An internal panic was caught at the boundary; this is a " \
                "bug worth reporting."],
    LAWHA_MUMTALIA => ["اللوحة ممتلئة ولا شيء فيها قابل للإخلاء الآن.",
                       "The atlas is full and nothing in it may be " \
                       "evicted right now."],
    GHAYR_MADUM => ["العملية غير متاحة في هذه النسخة.",
                    "The operation is not available in this build."],
    GHAYR_MUHAYYAA => ["المكتبة لم تُهيَّأ أو أُغلقت.",
                       "The library is not initialised, or was shut down."]
  }.freeze

  # A failure that crossed the ABI, carrying all four parts of Taarib's
  # error model: the permanent code (TAARIB-E-2500), the Arabic sentence,
  # the English sentence, and the next-action number. The exception's
  # message — what a rescue prints — is the Arabic sentence, because Arabic
  # is the primary text of this product.
  class KhataTaarib < StandardError
    attr_reader :halat, :ramz, :arabi, :injilizi, :khutwa

    def initialize(halat, ramz, arabi, injilizi, khutwa)
      badeel = JUMAL_IHTIYAT[halat] || JUMAL_IHTIYAT[KHATA_AAM]
      @halat = halat
      @ramz = ramz.to_s
      @arabi = (arabi.nil? || arabi.empty?) ? badeel[0] : arabi
      @injilizi = (injilizi.nil? || injilizi.empty?) ? badeel[1] : injilizi
      @khutwa = khutwa
      super(@arabi)
    end

    # The next action's stable name, or nil for a number this binding does
    # not know — a newer library, not an error.
    def ism_khutwa
      KHUTWA_ASMA[@khutwa]
    end
  end

  #--------------------------------------------------------------------------
  # The loaded library
  #--------------------------------------------------------------------------

  # One loaded taarib_jisr.dll: every exported function as a Win32API
  # object, plus the error retrieval that turns a status into a raised
  # KhataTaarib. The idiomatic classes below sit on top; the injected
  # window code normally never touches this directly.
  class Jisr
    attr_reader :masar, :kabir, :sagheer
    attr_reader :abi_isdar_api, :isdar_nass_api, :khata_akhir_api,
                :khata_khutwa_api, :khata_ramz_api, :khata_nass_api,
                :siyaq_insha_api, :siyaq_ihdham_api, :siyaq_amsah_api,
                :khatt_min_dhakira_api, :khatt_ihdham_api,
                :khatt_huwiya_api, :khatt_qiyasat_api, :khatt_aila_api,
                :silsila_insha_api, :silsila_ihdham_api,
                :takhtit_api, :qiyas_api, :makhzan_ihsaat_api,
                :lawha_insha_api, :lawha_ihdham_api, :lawha_ibda_itar_api,
                :lawha_shakl_api, :lawha_safha_api,
                :lawha_adad_safahat_api, :lawha_ihsaat_api

    # Binds every export. A missing DLL or a missing symbol raises here,
    # at load, with the path named — not as a NoMethodError three frames
    # into the first battle. +masar+ may be relative; LoadLibrary resolves
    # it against the process working directory, which for VX Ace is the
    # game's own folder.
    def initialize(masar)
      @masar = masar
      @abi_isdar_api          = rabt("taarib_abi_isdar", "pp", "i")
      @isdar_nass_api         = rabt("taarib_isdar_nass", "pip", "i")
      @khata_akhir_api        = rabt("taarib_khata_akhir", "v", "i")
      @khata_khutwa_api       = rabt("taarib_khata_khutwa", "v", "i")
      @khata_ramz_api         = rabt("taarib_khata_ramz", "pip", "i")
      @khata_nass_api         = rabt("taarib_khata_nass", "ipip", "i")
      @siyaq_insha_api        = rabt("taarib_siyaq_insha", "pp", "i")
      @siyaq_ihdham_api       = rabt("taarib_siyaq_ihdham", "i", "i")
      @siyaq_amsah_api        = rabt("taarib_siyaq_amsah", "i", "i")
      @khatt_min_dhakira_api  = rabt("taarib_khatt_min_dhakira", "ipiiip", "i")
      @khatt_ihdham_api       = rabt("taarib_khatt_ihdham", "ii", "i")
      @khatt_huwiya_api       = rabt("taarib_khatt_huwiya", "iip", "i")
      # The third slot is a float crossing as its bit pattern; see the
      # header comment on Win32API's limits.
      @khatt_qiyasat_api      = rabt("taarib_khatt_qiyasat", "iiip", "i")
      @khatt_aila_api         = rabt("taarib_khatt_aila", "iipip", "i")
      @silsila_insha_api      = rabt("taarib_silsila_insha", "ipip", "i")
      @silsila_ihdham_api     = rabt("taarib_silsila_ihdham", "ii", "i")
      @takhtit_api            = rabt("taarib_takhtit", "ipp", "i")
      @qiyas_api              = rabt("taarib_qiyas", "ipp", "i")
      @makhzan_ihsaat_api     = rabt("taarib_makhzan_ihsaat", "ip", "i")
      @lawha_insha_api        = rabt("taarib_lawha_insha", "iiiiiip", "i")
      @lawha_ihdham_api       = rabt("taarib_lawha_ihdham", "ii", "i")
      @lawha_ibda_itar_api    = rabt("taarib_lawha_ibda_itar", "ii", "i")
      @lawha_shakl_api        = rabt("taarib_lawha_shakl", "iiipp", "i")
      @lawha_safha_api        = rabt("taarib_lawha_safha", "iiip", "i")
      @lawha_adad_safahat_api = rabt("taarib_lawha_adad_safahat", "iip", "i")
      @lawha_ihsaat_api       = rabt("taarib_lawha_ihsaat", "iip", "i")
      # taarib_iltiqat_shaghghil and taarib_iltiqat_awqif — the runtime
      # string capture pair — are deliberately not bound. Registering
      # capture means handing the DLL a C function pointer to call back
      # through, and Win32API marshals exactly integers, Strings and
      # void: it cannot manufacture native callback code from a Ruby
      # block, so the two exports are unreachable from RGSS3 by
      # construction, not by oversight. Nothing here depends on them —
      # capture is an optional channel and the engine runs whole without
      # it; every other export is bound above.

      # kernel32's memcpy, the standard RGSS way to read memory the DLL
      # merely lends us (atlas pages). Destination is a Ruby String buffer,
      # source is a raw address.
      @nakl_api = Win32API.new("kernel32", "RtlMoveMemory", "pii", "v")

      @kabir, @sagheer = abi_isdar
      if @kabir != ISDAR_KABIR
        # A refusal the caller makes has no stash to read; this is the one
        # place the binding fabricates the sentences itself.
        raise KhataTaarib.new(
          ISDAR_GHAYR_MUTAWAFIQ, "",
          "مكتبة تعريب المحمّلة إصدارها الأكبر #{@kabir} " \
          "وهذا الرابط يتطلب #{ISDAR_KABIR}.",
          "The loaded Taarib library reports major version #{@kabir} " \
          "but this binding requires #{ISDAR_KABIR}.",
          10) # tahdith_taarib: update Taarib itself.
      end
    end

    # Reads +tul+ bytes at a raw address into a fresh BINARY String. Used
    # only for pointers the ABI documents as borrowed; the copy is not an
    # inefficiency, it is the upload-and-forget the borrow contract asks
    # for, done the only way Ruby can do it.
    def nakl(unwan, tul)
      return Taarib.hijz(0) if unwan == 0 || tul <= 0
      hadaf = Taarib.hijz(tul)
      @nakl_api.call(hadaf, unwan, tul)
      hadaf
    end

    # Runs one capacity-negotiated string retrieval to completion. The
    # block receives (buffer, capacity, required-out buffer) and returns
    # the status. Bounded: a library that keeps demanding more than it was
    # just given is broken, and spinning on it inside the game's script
    # thread would hang the game.
    def iqra_nass
      siaa = 128
      4.times do
        hadaf = Taarib.hijz(siaa)
        matlub = Taarib.hijz(4)
        halat = yield(hadaf, siaa, matlub)
        if halat == NAJAH
          nihaya = hadaf.index("\0") || hadaf.bytesize
          return hadaf[0, nihaya].force_encoding("UTF-8")
        end
        if halat == SIAT_QASIRA
          talab = matlub.unpack("V")[0]
          if talab > siaa
            siaa = talab
            next
          end
        end
        return ""
      end
      ""
    end

    # Raises KhataTaarib for a non-zero status, built from the library's
    # thread-local stash. All four parts are read now, before control
    # returns anywhere that could make another call: the next successful
    # entry point on this thread clears the stash, and the retrieval
    # functions themselves are the only ones that never touch it.
    def irfa(halat)
      ramz = iqra_nass { |b, s, m| @khata_ramz_api.call(b, s, m) }
      arabi = iqra_nass { |b, s, m| @khata_nass_api.call(0, b, s, m) }
      injilizi = iqra_nass { |b, s, m| @khata_nass_api.call(1, b, s, m) }
      khutwa = @khata_khutwa_api.call
      raise KhataTaarib.new(halat, ramz, arabi, injilizi, khutwa)
    end

    # Silence for zero, a raised KhataTaarib for anything else.
    def tahaqqaq(halat)
      irfa(halat) if halat != NAJAH
      halat
    end

    # The library's (major, minor) ABI pair.
    def abi_isdar
      kabir = Taarib.hijz(4)
      sagheer = Taarib.hijz(4)
      halat = @abi_isdar_api.call(kabir, sagheer)
      irfa(halat) if halat != NAJAH
      [kabir.unpack("V")[0], sagheer.unpack("V")[0]]
    end

    # The library's human-readable version string, for diagnostics.
    def isdar_nass
      iqra_nass { |b, s, m| @isdar_nass_api.call(b, s, m) }
    end

    private

    def rabt(ism, wasail, kharj)
      Win32API.new(@masar, ism, wasail, kharj)
    rescue
      raise KhataTaarib.new(
        GHAYR_MUHAYYAA, "",
        "تعذّر ربط الدالة #{ism} من #{@masar}.",
        "Could not bind #{ism} from #{@masar}.",
        11) # iadat_tarkib_ittar: reinstall the framework for this game.
    end
  end

  #--------------------------------------------------------------------------
  # Decoded record shapes
  #--------------------------------------------------------------------------

  # One positioned glyph, decoded from the buffer. Fields mirror TaaribHarf;
  # note there is no codepoint field, on purpose, mirroring the ABI: the
  # blit loop is given glyph identifiers only, and cannot fall back to
  # drawing characters even by accident.
  Harf = Struct.new(:muarrif, :anqud, :s, :a, :taqaddum,
                    :nitaq, :khatt, :alam) do
    def alama?
      (alam & HARF_ALAMA) != 0
    end
  end

  # One laid-out line, decoded from the buffer. Fields mirror TaaribSatr.
  Satr = Struct.new(:awwal_harf, :adad_huruf, :bidayat_mantiqi,
                    :nihayat_mantiqi, :asas, :bidaya, :ard, :irtifa,
                    :suud, :hubut, :dabt, :alam) do
    def akhir?
      (alam & SATR_AKHIR) != 0
    end

    def yameen?
      (alam & SATR_YAMEEN) != 0
    end
  end

  # A font's metrics at a pixel size. Fields mirror TaaribQiyasatKhatt.
  QiyasatKhatt = Struct.new(:suud, :hubut, :fajwa, :irtifa_satr,
                            :uluw_kabital, :uluw_saghir, :wahdat)

  # Text measured without positioning. Fields mirror TaaribQiyasNass.
  QiyasNass = Struct.new(:ard, :irtifa, :suud, :hubut, :adad_sutur)

  # Where a glyph lives in the atlas. Fields mirror TaaribMawdiShakl.
  MawdiShakl = Struct.new(:taqaddum, :s, :a, :ard, :irtifa,
                          :izaha_s, :izaha_a, :safha)

  # The atlas's counters. Fields mirror TaaribIhsaatLawha.
  IhsaatLawha = Struct.new(:isabat, :ikhfaqat, :ikhlaat, :ahdath_namu,
                           :bayt, :mizaniya, :ashkal, :safahat)

  # The layout cache's counters. Fields mirror TaaribIhsaatKhazina.
  IhsaatKhazina = Struct.new(:isabat, :ikhfaqat, :ikhlaat, :bayt,
                             :mizaniya, :madkhalat)

  #--------------------------------------------------------------------------
  # A finished layout
  #--------------------------------------------------------------------------

  # One layout result, decoding records out of the context's reusable
  # buffer on demand.
  #
  # The buffer is shared with the context and overwritten by its next
  # khattit call, so a Takhtit is valid only until then — read what the
  # window needs (usually one pass of kul_harf into Bitmap#blt) and let it
  # go. Decoding allocates a Struct per record; that is unavoidable in
  # Ruby, and it is affordable because the expensive work — shaping — was
  # already skipped by the native layout cache for every repeated string.
  class Takhtit
    attr_reader :adad_huruf, :adad_sutur, :ard, :irtifa, :hajm, :alam,
                :tajawuz

    def initialize(huruf_buf, sutur_buf, ras)
      @huruf_buf = huruf_buf
      @sutur_buf = sutur_buf
      @adad_huruf = ras[2]
      @adad_sutur = ras[5]
      @ard = ras[6]
      @irtifa = ras[7]
      @hajm = ras[8]
      @alam = ras[9]
      @tajawuz = nil
      if (@alam & TAKHTIT_TAJAWUZ) != 0
        @tajawuz = {
          :ard => ras[10], :ard_mutah => ras[11],
          :irtifa => ras[12], :irtifa_mutah => ras[13],
          :awwal_satr => ras[14], :adad_sutur => ras[15]
        }
      end
    end

    # The glyph at +fahras+, decoded fresh from the buffer.
    def harf(fahras)
      if fahras < 0 || fahras >= @adad_huruf
        raise IndexError, "harf #{fahras} of #{@adad_huruf}"
      end
      Harf.new(*@huruf_buf[fahras * HARF_HAJM, HARF_HAJM].unpack(HARF_FMT))
    end

    # The line at +fahras+, decoded fresh from the buffer.
    def satr(fahras)
      if fahras < 0 || fahras >= @adad_sutur
        raise IndexError, "satr #{fahras} of #{@adad_sutur}"
      end
      Satr.new(*@sutur_buf[fahras * SATR_HAJM, SATR_HAJM].unpack(SATR_FMT))
    end

    # Yields every glyph in visual order — the blit loop's shape.
    def kul_harf
      @adad_huruf.times { |fahras| yield harf(fahras) }
    end

    # Yields every line.
    def kul_satr
      @adad_sutur.times { |fahras| yield satr(fahras) }
    end

    def yameen?
      (@alam & TAKHTIT_YAMEEN) != 0
    end

    def maqsus?
      (@alam & TAKHTIT_MAQSUS) != 0
    end

    def min_makhzan?
      (@alam & TAKHTIT_MAKHZAN) != 0
    end
  end

  #--------------------------------------------------------------------------
  # Handles
  #--------------------------------------------------------------------------

  # A loaded, validated font. The DLL copied the bytes at load, so the
  # String that fed it — typically read straight out of the patched
  # archive — may be discarded immediately.
  class Khatt
    def initialize(siyaq, maqbad)
      @siyaq = siyaq
      @maqbad = maqbad
    end

    def maqbad
      if @maqbad.nil?
        raise KhataTaarib.new(MAQBAD_BATIL, "",
                              "استُخدم خط بعد تدميره.",
                              "A font was used after it was destroyed.", 0)
      end
      @maqbad
    end

    # The font's stable identity: the hash of its bytes, which is what
    # every atlas glyph key and cache key carries.
    def huwiya
      hadaf = Taarib.hijz(8)
      @siyaq.jisr.tahaqqaq(
        @siyaq.jisr.khatt_huwiya_api.call(@siyaq.maqbad, maqbad, hadaf))
      hadaf.unpack("Q")[0]
    end

    # The font's metrics at +hajm+ pixels, as a QiyasatKhatt. The size
    # crosses as its IEEE-754 bits in an integer slot — the one float this
    # ABI passes by value, and the one place the bit-pattern trick is used.
    def qiyasat(hajm)
      hadaf = Taarib.hijz(QIYASAT_KHATT_HAJM)
      @siyaq.jisr.tahaqqaq(
        @siyaq.jisr.khatt_qiyasat_api.call(
          @siyaq.maqbad, maqbad, Taarib.awama_bit(hajm), hadaf))
      qiyam = hadaf.unpack(QIYASAT_KHATT_FMT)
      QiyasatKhatt.new(qiyam[0], qiyam[1], qiyam[2], qiyam[3],
                       qiyam[4], qiyam[5], qiyam[6])
    end

    # The font's family name.
    def aila
      s = @siyaq
      k = maqbad
      s.jisr.iqra_nass { |b, siaa, m| s.jisr.khatt_aila_api.call(s.maqbad, k, b, siaa, m) }
    end

    # Destroys the handle. Idempotent on this side, because RGSS teardown
    # paths run twice more often than they run once. A chain built from
    # this font keeps its own reference and survives.
    def ihdham
      return if @maqbad.nil?
      maqbad_qadim = @maqbad
      @maqbad = nil
      @siyaq.jisr.tahaqqaq(
        @siyaq.jisr.khatt_ihdham_api.call(@siyaq.maqbad, maqbad_qadim))
    end
  end

  # An ordered chain of fonts, tried in order per character — fallback
  # without a fallback shaper. Every glyph in a layout names its chain
  # index, so the atlas knows which font it came from.
  class Silsila
    def initialize(siyaq, maqbad)
      @siyaq = siyaq
      @maqbad = maqbad
    end

    def maqbad
      if @maqbad.nil?
        raise KhataTaarib.new(MAQBAD_BATIL, "",
                              "استُخدمت سلسلة خطوط بعد تدميرها.",
                              "A font chain was used after it was " \
                              "destroyed.", 0)
      end
      @maqbad
    end

    def ihdham
      return if @maqbad.nil?
      maqbad_qadim = @maqbad
      @maqbad = nil
      @siyaq.jisr.tahaqqaq(
        @siyaq.jisr.silsila_ihdham_api.call(@siyaq.maqbad, maqbad_qadim))
    end
  end

  # A glyph atlas that grows and evicts at runtime. The VX Ace adapter
  # normally blits from an atlas Bitmap shipped inside the patch; this
  # runtime atlas exists for the strings the compiler never saw — player
  # names, debug text — and its pages become Bitmaps via Lawha#safha.
  class Lawha
    def initialize(siyaq, maqbad)
      @siyaq = siyaq
      @maqbad = maqbad
    end

    def maqbad
      if @maqbad.nil?
        raise KhataTaarib.new(MAQBAD_BATIL, "",
                              "استُخدمت لوحة بعد تدميرها.",
                              "An atlas was used after it was destroyed.",
                              0)
      end
      @maqbad
    end

    # Marks a frame boundary, letting the atlas evict rectangles from
    # previous frames while never reclaiming one promised to this frame.
    def ibda_itar
      @siyaq.jisr.tahaqqaq(
        @siyaq.jisr.lawha_ibda_itar_api.call(@siyaq.maqbad, maqbad))
    end

    # Where a glyph lives, rasterizing and packing it on a miss.
    # +hajm_rubi+ is the pixel size in quarter-pixels; +khatt+ indexes the
    # chain; +bakat+ is the subpixel bucket.
    def shakl(silsila, muarrif, hajm_rubi, khatt = 0, bakat = 0)
      miftah = [muarrif, hajm_rubi, khatt, bakat].pack(MIFTAH_FMT)
      hadaf = Taarib.hijz(MAWDI_HAJM)
      @siyaq.jisr.tahaqqaq(
        @siyaq.jisr.lawha_shakl_api.call(
          @siyaq.maqbad, maqbad, silsila.maqbad, miftah, hadaf))
      qiyam = hadaf.unpack(MAWDI_FMT)
      MawdiShakl.new(qiyam[0], qiyam[1], qiyam[2], qiyam[3], qiyam[4],
                     qiyam[5], qiyam[6], qiyam[7])
    end

    # One texture page as [bytes, ard, irtifa, namat], where bytes is a
    # BINARY String of ard*irtifa single-channel texels.
    #
    # The DLL lends a pointer that is valid only until the next call that
    # can change the atlas, and Ruby cannot alias foreign memory — so the
    # texels are copied out through RtlMoveMemory immediately, which is
    # exactly the upload-and-forget the borrow contract requires anyway.
    def safha(fahras)
      hadaf = Taarib.hijz(SAFHA_HAJM)
      @siyaq.jisr.tahaqqaq(
        @siyaq.jisr.lawha_safha_api.call(@siyaq.maqbad, maqbad, fahras,
                                         hadaf))
      unwan, tul, ard, irtifa, namat = hadaf.unpack(SAFHA_FMT)
      [@siyaq.jisr.nakl(unwan, tul), ard, irtifa, namat]
    end

    # How many pages are open.
    def adad_safahat
      hadaf = Taarib.hijz(4)
      @siyaq.jisr.tahaqqaq(
        @siyaq.jisr.lawha_adad_safahat_api.call(@siyaq.maqbad, maqbad,
                                                hadaf))
      hadaf.unpack("V")[0]
    end

    # The atlas's counters. A miss count still climbing after the first
    # minutes of play means the patch compiler missed strings; the
    # diagnostics overlay surfaces exactly that.
    def ihsaat
      hadaf = Taarib.hijz(IHSAAT_HAJM)
      @siyaq.jisr.tahaqqaq(
        @siyaq.jisr.lawha_ihsaat_api.call(@siyaq.maqbad, maqbad, hadaf))
      qiyam = hadaf.unpack(IHSAAT_FMT)
      IhsaatLawha.new(qiyam[0], qiyam[1], qiyam[2], qiyam[3], qiyam[4],
                      qiyam[5], qiyam[6], qiyam[7])
    end

    def ihdham
      return if @maqbad.nil?
      maqbad_qadim = @maqbad
      @maqbad = nil
      @siyaq.jisr.tahaqqaq(
        @siyaq.jisr.lawha_ihdham_api.call(@siyaq.maqbad, maqbad_qadim))
    end
  end

  #--------------------------------------------------------------------------
  # The engine context
  #--------------------------------------------------------------------------

  # The layout decisions, with the ABI's own zero defaults: automatic
  # direction, detected language, no justification, leading-edge alignment,
  # diacritics kept, digits untouched, overflow reported. Zeros on purpose:
  # this binding does not make policy, and a wrapper that quietly defaulted
  # to kashida justification would be deciding how a patch looks.
  KHIYARAT_IFTIRADIYA = {
    :ittijah => ITTIJAH_TILQAI, :lugha => LUGHA_TILQAI,
    :dabt => DABT_BILA, :muhadhaha => MUHADHAHA_BIDAYA,
    :tashkeel => TASHKEEL_IBQA, :arqam => ARQAM_KAMA_HIYA,
    :tajawuz => TAJAWUZ_BALLAGH, :hajm_adna => 0.0, :irtifa_satr => 0.0,
    :tabaud_ahruf => 0.0, :tabaud_kalimat => 0.0,
    :hiwar => false, :satr_wahid => false, :sifat => [],
    :ard_mutah => 0.0, :irtifa_mutah => 0.0, :nitaqat => []
  }.freeze

  # An engine context: the caches, the pooled buffers, and — on this side —
  # the reusable glyph and line buffers every khattit writes into. One
  # context serves one thread at a time; RGSS3 scripts run on one thread,
  # so the adapter holds exactly one for the whole session.
  class Siyaq
    attr_reader :jisr

    # Grown once on SIAT_QASIRA and never shrunk, so after the first few
    # message windows layout stops allocating buffers on this side too.
    SIAA_HURUF = 512
    SIAA_SUTUR = 16

    def initialize(jisr, mizaniyat_makhzan = 4 * 1024 * 1024,
                   adad_makhazin = 2)
      @jisr = jisr
      khiyarat = [mizaniyat_makhzan, adad_makhazin, 0]
                 .pack(SIYAQ_KHIYARAT_FMT)
      khuruj = Taarib.hijz(4)
      @jisr.tahaqqaq(@jisr.siyaq_insha_api.call(khiyarat, khuruj))
      @maqbad = khuruj.unpack("V")[0]
      @siaat_huruf = SIAA_HURUF
      @siaat_sutur = SIAA_SUTUR
      @huruf_buf = Taarib.hijz(HARF_HAJM * @siaat_huruf)
      @sutur_buf = Taarib.hijz(SATR_HAJM * @siaat_sutur)
    end

    def maqbad
      if @maqbad.nil?
        raise KhataTaarib.new(MAQBAD_BATIL, "",
                              "استُخدم سياق بعد تدميره.",
                              "A context was used after it was destroyed.",
                              0)
      end
      @maqbad
    end

    # Destroys the context and everything the DLL holds for it.
    def ihdham
      return if @maqbad.nil?
      maqbad_qadim = @maqbad
      @maqbad = nil
      @jisr.tahaqqaq(@jisr.siyaq_ihdham_api.call(maqbad_qadim))
    end

    # Empties the layout cache, keeping the context — for a scene change
    # that retires a screenful of strings.
    def amsah
      @jisr.tahaqqaq(@jisr.siyaq_amsah_api.call(maqbad))
    end

    # Loads a font from a String of bytes — typically read straight out of
    # the patched archive — and returns a Khatt. The DLL copies; the String
    # may be discarded after this returns. +fahs_arabi+ runs the Arabic
    # table validation; leave it on for any font that will shape Arabic,
    # because a font without joining tables fails silently as isolated
    # letters, not loudly.
    def khatt_min_dhakira(bayt, fahras = 0, fahs_arabi = true)
      nuskha = bayt.dup.force_encoding("BINARY")
      khuruj = Taarib.hijz(4)
      @jisr.tahaqqaq(
        @jisr.khatt_min_dhakira_api.call(
          maqbad, nuskha, nuskha.bytesize, fahras, fahs_arabi ? 1 : 0,
          khuruj))
      Khatt.new(self, khuruj.unpack("V")[0])
    end

    # Builds a fallback chain from Khatt handles, in the order they should
    # be tried. The handle array crosses as a packed String of handles.
    def silsila(khutut)
      if khutut.nil? || khutut.empty?
        raise KhataTaarib.new(QEEMA_BATILA, "",
                              "سلسلة الخطوط تحتاج خطًا واحدًا على الأقل.",
                              "A font chain needs at least one font.", 0)
      end
      saff = khutut.map { |khatt| khatt.maqbad }.pack("V*")
      khuruj = Taarib.hijz(4)
      @jisr.tahaqqaq(
        @jisr.silsila_insha_api.call(maqbad, saff, khutut.size, khuruj))
      Silsila.new(self, khuruj.unpack("V")[0])
    end

    # Lays text out and returns a Takhtit over this context's reusable
    # buffers. +khiyarat+ is an options Hash merged over
    # KHIYARAT_IFTIRADIYA; :ard_mutah, :irtifa_mutah and :nitaqat (an
    # array of span Hashes) ride in the same Hash because Ruby 1.9 has no
    # keyword arguments.
    #
    # The negotiation loop is written out here so no window script writes
    # its own wrong one: on SIAT_QASIRA the required counts are read back
    # from the buffer struct, the buffers grow to exactly those counts,
    # and the call is retried. The result goes stale on the next khattit —
    # one pass of kul_harf into the Bitmap, then let it go.
    def khattit(nass, silsila, hajm, khiyarat = {})
      kh = KHIYARAT_IFTIRADIYA.merge(khiyarat)
      hafiz = []
      talab = itbakh_talab(nass, silsila, hajm, kh, hafiz)
      4.times do
        makhzan = [Taarib.unwan(@huruf_buf), @siaat_huruf, 0,
                   Taarib.unwan(@sutur_buf), @siaat_sutur, 0,
                   0.0, 0.0, 0.0, 0,
                   0.0, 0.0, 0.0, 0.0, 0, 0].pack(MAKHZAN_FMT)
        halat = @jisr.takhtit_api.call(maqbad, talab, makhzan)
        ras = makhzan.unpack(MAKHZAN_FMT)
        if halat == NAJAH
          return Takhtit.new(@huruf_buf, @sutur_buf, ras)
        end
        @jisr.irfa(halat) if halat != SIAT_QASIRA
        if ras[2] > @siaat_huruf
          @siaat_huruf = ras[2]
          @huruf_buf = Taarib.hijz(HARF_HAJM * @siaat_huruf)
        end
        if ras[5] > @siaat_sutur
          @siaat_sutur = ras[5]
          @sutur_buf = Taarib.hijz(SATR_HAJM * @siaat_sutur)
        end
      end
      # Four rounds of growing to the exact demanded size and still short:
      # the demand is not stable. Report instead of spinning.
      @jisr.irfa(SIAT_QASIRA)
    end

    # Measures text without positioning a glyph — same pipeline, same
    # policies, so the number agrees with what khattit would draw.
    def qis(nass, silsila, hajm, khiyarat = {})
      kh = KHIYARAT_IFTIRADIYA.merge(khiyarat)
      hafiz = []
      talab = itbakh_talab(nass, silsila, hajm, kh, hafiz)
      hadaf = Taarib.hijz(QIYAS_HAJM)
      @jisr.tahaqqaq(@jisr.qiyas_api.call(maqbad, talab, hadaf))
      qiyam = hadaf.unpack(QIYAS_FMT)
      QiyasNass.new(qiyam[0], qiyam[1], qiyam[2], qiyam[3], qiyam[4])
    end

    # Creates a runtime glyph atlas. Options: :aqsa_ard, :aqsa_irtifa
    # (page limits, default 2048 — the conservative GPU floor), :hashw
    # (padding texels), :namat (NAMAT_TAGHTIYA or NAMAT_MISAFA),
    # :mizaniya (byte budget).
    def lawha(khiyarat = {})
      aqsa_ard = khiyarat[:aqsa_ard] || 2048
      aqsa_irtifa = khiyarat[:aqsa_irtifa] || 2048
      hashw = khiyarat[:hashw] || 1
      namat = khiyarat[:namat] || NAMAT_TAGHTIYA
      mizaniya = khiyarat[:mizaniya] || 16 * 1024 * 1024
      khuruj = Taarib.hijz(4)
      @jisr.tahaqqaq(
        @jisr.lawha_insha_api.call(maqbad, aqsa_ard, aqsa_irtifa, hashw,
                                   namat, mizaniya, khuruj))
      Lawha.new(self, khuruj.unpack("V")[0])
    end

    private

    # Packs one TaaribTalab, appending every buffer the struct points at to
    # +hafiz+ so a garbage collection between here and the call cannot free
    # what the DLL is about to read. The caller keeps +hafiz+ in scope for
    # the duration of the call — that reference is the entire mechanism.
    def itbakh_talab(nass, silsila, hajm, kh, hafiz)
      nass_bayt = nass.to_s.dup.force_encoding("BINARY")
      hafiz << nass_bayt

      nitaqat = kh[:nitaqat] || []
      if nitaqat.empty?
        nitaqat_unwan = 0
      else
        nitaqat_buf = nitaqat.map { |n| itbakh_nitaq(n) }.join("")
        nitaqat_buf.force_encoding("BINARY")
        hafiz << nitaqat_buf
        nitaqat_unwan = Taarib.unwan(nitaqat_buf)
      end

      sifat = kh[:sifat] || []
      if sifat.empty?
        sifat_unwan = 0
      else
        sifat_buf = sifat.map { |zawj|
          wasm = zawj[0].to_s
          if wasm.bytesize != 4
            raise KhataTaarib.new(QEEMA_BATILA, "",
                                  "وسم الميزة يجب أن يكون أربعة بايتات: " \
                                  "#{wasm}",
                                  "A feature tag must be exactly four " \
                                  "bytes: #{wasm}", 0)
          end
          [wasm, zawj[1]].pack(SIFA_FMT)
        }.join("")
        sifat_buf.force_encoding("BINARY")
        hafiz << sifat_buf
        sifat_unwan = Taarib.unwan(sifat_buf)
      end

      alam = 0
      alam |= KHIYAR_HIWAR if kh[:hiwar]
      alam |= KHIYAR_SATR_WAHID if kh[:satr_wahid]

      [Taarib.unwan(nass_bayt), nass_bayt.bytesize,
       nitaqat_unwan, nitaqat.size, silsila.maqbad,
       hajm.to_f, kh[:ard_mutah].to_f, kh[:irtifa_mutah].to_f,
       sifat_unwan, sifat.size,
       kh[:ittijah], kh[:lugha], kh[:dabt], kh[:muhadhaha],
       kh[:tashkeel], kh[:arqam], kh[:tajawuz],
       kh[:hajm_adna].to_f, kh[:irtifa_satr].to_f,
       kh[:tabaud_ahruf].to_f, kh[:tabaud_kalimat].to_f,
       alam].pack(TALAB_FMT)
    end

    # Packs one style span Hash into TaaribNitaqUslub bytes. Only what the
    # span sets raises a flag, so a span cannot claim a colour it does not
    # carry. Offsets are byte offsets into the UTF-8 text, as the ABI
    # speaks them.
    def itbakh_nitaq(n)
      alam = 0
      lawn = 0
      khatt = 0
      wazn = 0
      hajm = 0.0
      ard_d = 0.0
      irtifa_d = 0.0
      asas_d = 0.0
      marja_d = 0
      alam |= USLUB_MAAIL if n[:mail]
      if n[:lawn]
        alam |= USLUB_LAWN
        lawn = n[:lawn]
      end
      if n[:khatt]
        alam |= USLUB_KHATT
        khatt = n[:khatt]
      end
      if n[:wazn]
        alam |= USLUB_WAZN
        wazn = n[:wazn]
      end
      if n[:hajm]
        alam |= USLUB_HAJM
        hajm = n[:hajm].to_f
      end
      if n[:dharra]
        alam |= USLUB_DHARRA
        ard_d = n[:dharra][0].to_f
        irtifa_d = n[:dharra][1].to_f
        asas_d = n[:dharra][2].to_f
        marja_d = n[:dharra][3] || 0
      end
      [n[:bidaya], n[:tul], lawn, alam, hajm,
       (n[:tabaud] || 0.0).to_f, (n[:izaha] || 0.0).to_f,
       ard_d, irtifa_d, asas_d, marja_d, wazn, n[:id] || 0, khatt,
       0, 0, 0].pack(NITAQ_FMT)
    end
  end

  #--------------------------------------------------------------------------
  # The module surface the injected window code calls
  #--------------------------------------------------------------------------

  @jisr = nil

  # Loads taarib_jisr.dll from the patch's own directory beside Game.exe.
  # Tries <dalil>/i686/taarib_jisr.dll — the layout patches ship, named for
  # the one architecture VX Ace has — then <dalil>/taarib_jisr.dll for a
  # flat layout. Never the system search path: a same-named DLL from
  # somewhere on a player's PATH loading successfully would be far worse
  # than a clean failure naming what was tried.
  def self.hammil(dalil = "Taarib")
    return @jisr if @jisr
    murashshahun = [
      File.join(dalil, "i686", "taarib_jisr.dll"),
      File.join(dalil, "taarib_jisr.dll")
    ]
    ilal = []
    murashshahun.each do |masar|
      unless File.file?(masar)
        ilal << "#{masar}: not present"
        next
      end
      begin
        @jisr = Jisr.new(masar)
        return @jisr
      rescue KhataTaarib => khata
        ilal << "#{masar}: #{khata.injilizi}"
      end
    end
    raise KhataTaarib.new(
      GHAYR_MUHAYYAA, "",
      "تعذّر تحميل مكتبة تعريب من: #{ilal.join('; ')}",
      "Could not load the Taarib library from: #{ilal.join('; ')}",
      11) # iadat_tarkib_ittar: reinstall the framework for this game.
  end

  # The loaded library, or nil before hammil.
  def self.jisr
    @jisr
  end

  # Whether the library is loaded and version-checked.
  def self.mahmul?
    !@jisr.nil?
  end

  # A new engine context over the loaded library. The adapter makes exactly
  # one and keeps it for the session.
  def self.siyaq(mizaniyat_makhzan = 4 * 1024 * 1024, adad_makhazin = 2)
    unless mahmul?
      raise KhataTaarib.new(GHAYR_MUHAYYAA, "",
                            "استدعيت Taarib.siyaq قبل Taarib.hammil.",
                            "Taarib.siyaq was called before " \
                            "Taarib.hammil.", 0)
    end
    Siyaq.new(@jisr, mizaniyat_makhzan, adad_makhazin)
  end

  # The layout cache's counters for +siyaq+, as an IhsaatKhazina.
  # Diagnostics, not control: a hit count that stays flat while a message
  # window redraws the same strings every frame means the cache key is
  # absorbing something that changes per frame, and the diagnostics
  # overlay says so with this number instead of a feeling.
  def self.ihsaat_makhzan(siyaq)
    hadaf = hijz(IHSAAT_KHAZINA_HAJM)
    siyaq.jisr.tahaqqaq(
      siyaq.jisr.makhzan_ihsaat_api.call(siyaq.maqbad, hadaf))
    qiyam = hadaf.unpack(IHSAAT_KHAZINA_FMT)
    IhsaatKhazina.new(qiyam[0], qiyam[1], qiyam[2], qiyam[3], qiyam[4],
                      qiyam[5])
  end
end

#==============================================================================
# The adapter
#
# Everything above this line is the binding: the C ABI, typed, with nothing
# above it. Everything below is the adapter that uses it — the rung branch, the
# settings, the atlas, and the two places RGSS3 actually draws text.
#
# The rung is read, not decided
# -----------------------------
# The Rust side probed which runtime this game loads and wrote the answer into
# Taarib/idad.txt. This file reads it and branches:
#
#   rung 1 or 2  the platform shapes Arabic — an mkxp runtime with HarfBuzz
#                beside it. Correct the direction and the alignment and leave
#                the shaping alone. Bitmap#draw_text still draws the text, and
#                the text stays text.
#
#   rung 3       the official RGSS3 runtime, which draws through the platform's
#                plain text call: characters map to glyphs through the font's
#                cmap and no OpenType layout runs, so Arabic comes out in
#                isolated forms. Taarib lays the run out through the C ABI and
#                blits its own glyphs from an atlas the patch shipped.
#
# Nothing here re-derives the rung from the runtime it finds. A second opinion
# is a second chance to disagree, and the two would disagree on exactly the
# builds where it matters.
#
# Why the atlas comes from the patch and not from the ABI
# ------------------------------------------------------
# The ABI has a runtime atlas and this adapter does not use it for pixels, on
# purpose. RGSS3's only way to put arbitrary pixels into a Bitmap is
# Bitmap#set_pixel, one texel per call; a single 2048-square page is four
# million calls and takes minutes. So the patch rasterizes at build time and
# ships an image, and this side blits rectangles out of it. The ABI is still
# what lays out — glyph identifiers and positions, in visual order, with the
# bidirectional algorithm applied — and that is the part that cannot be
# precomputed, because it depends on the string.
#
# Why there is more than one atlas image
# --------------------------------------
# RGSS3 has no tinting operation at all: Bitmap#blt composites the source as it
# is, and there is no multiply, no colour matrix, and no per-channel blend.
# Colouring a white glyph mask is therefore not possible at run time. So the
# patch bakes one image per colour the game actually uses — its windowskin
# palette is a fixed thirty-two entries and a real game touches a handful — and
# this side picks the nearest by squared distance. A game that asks for a
# colour no layer carries gets the nearest one and a recorded line saying so,
# which is a visible-but-legible failure rather than an invisible one.
#
# No evaluation of patch content
# ------------------------------
# The settings file is key=value and the glyph table is whitespace-separated
# integers, and both are parsed by the code below. RGSS3 embeds Ruby 1.9.2 with
# no JSON parser, which is why the formats are what they are — and `eval` on a
# file the patch shipped is exactly what this product does not do, whatever the
# convenience.
#==============================================================================

module Taarib
  #--------------------------------------------------------------------------
  # The rungs, as the Rust side numbers them.
  #
  # Integers, and compared as integers. They reach the capability report and
  # the registry's per-game records, where a value written by an older build
  # has to keep meaning what it meant.
  #--------------------------------------------------------------------------
  module Rutba
    # The engine shapes correctly: font, direction, translation, stop.
    IDAD = 1
    # The engine shapes and lays right-to-left text out wrongly.
    TASHEEH = 2
    # The engine cannot shape. Taarib lays out and draws.
    ISTILA = 3
  end

  #--------------------------------------------------------------------------
  # The settings file
  #--------------------------------------------------------------------------

  # What the patch decided, read from Taarib/idad.txt.
  #
  # One key=value per line; anything after a '#' at the start of a line is a
  # comment; an unknown key is kept rather than refused, because a settings
  # file written by a newer Taarib is a forward-compatibility problem and not
  # a reason to leave a player with an unpatched game.
  class Idad
    attr_reader :rutba, :thiqa, :khatt, :hajm, :dalil_jisr, :lawha, :jadwal
    attr_reader :athar, :khaam

    def initialize(khaam = {})
      @khaam = khaam
      @rutba = (khaam["rutba"] || Rutba::IDAD).to_i
      @thiqa = (khaam["thiqa"] || 0).to_i
      @khatt = khaam["khatt"].to_s
      @hajm = (khaam["hajm"] || 24).to_i
      @dalil_jisr = khaam["dalil_jisr"].to_s
      @dalil_jisr = "Taarib" if @dalil_jisr.empty?
      @lawha = khaam["lawha"].to_s
      @lawha = "Taarib/lawha" if @lawha.empty?
      @jadwal = khaam["jadwal"].to_s
      @jadwal = "Taarib/lawha.tbl" if @jadwal.empty?
      @athar = []
      khaam.keys.sort.each do |miftah|
        @athar << khaam[miftah] if miftah =~ /\Aathar\d+\z/
      end
    end

    # Whether these settings ask for the glyph takeover.
    def yastawli?
      @rutba >= Rutba::ISTILA
    end

    # Reads the file, or returns defaults when it is absent or unreadable.
    #
    # Defaults mean the configuration rung, which registers the font and
    # changes nothing else. That is the failure a player can still play
    # through, and it is the right one to fall back to.
    def self.iqra(masar)
      khaam = {}
      begin
        File.open(masar, "rb") do |malaf|
          malaf.each_line do |satr|
            satr = satr.strip
            next if satr.empty? || satr[0, 1] == "#"
            fasl = satr.index("=")
            next if fasl.nil? || fasl == 0
            miftah = satr[0, fasl].strip
            qeema = satr[(fasl + 1)..-1].to_s.strip
            khaam[miftah] = qeema
          end
        end
      rescue StandardError
        return new({})
      end
      new(khaam)
    end
  end

  #--------------------------------------------------------------------------
  # The pre-baked atlas
  #--------------------------------------------------------------------------

  # One glyph's rectangle inside an atlas image, and how to place it.
  #
  # +izaha_s+ and +izaha_a+ are FreeType's bitmap_left and bitmap_top: the
  # horizontal offset is to the right of the pen and the vertical one is *up*
  # from the baseline. That sign convention is the single most common thing to
  # get wrong in a blitter and it is stated here rather than left implicit.
  Shakl = Struct.new(:s, :a, :ard, :irtifa, :izaha_s, :izaha_a)

  # The glyph table and the atlas images the patch shipped.
  #
  # The table is whitespace-separated integers, one glyph per line, with a
  # short header naming the images. It is parsed rather than evaluated: a
  # format a hundred lines of Ruby can read is the price of never handing a
  # file the patch shipped to the interpreter.
  #
  #   # taarib-lawha 1
  #   sura <index> <image path, no extension> <r> <g> <b>
  #   shakl <quarter-pixel size> <glyph id> <chain index> \
  #         <s> <a> <width> <height> <left> <top>
  #
  # Glyph rectangles are shared across images: every +sura+ is the same
  # packing in a different colour, so one table addresses all of them.
  class JadwalAshkal
    # [index, path, Color] for every baked colour layer.
    attr_reader :suwar
    # How many glyph rows the table declared.
    attr_reader :adad

    def initialize
      @ashkal = {}
      @suwar = []
      @suwar_muhammala = {}
      @adad = 0
      @ikhfaqat = 0
    end

    # How many lookups found nothing. A count that keeps climbing after the
    # first minutes of play means the patch compiler missed strings, and that
    # is a defect in the patch rather than in the game — so it is counted and
    # surfaced instead of being silently drawn as nothing.
    def ikhfaqat
      @ikhfaqat
    end

    # Reads the table. Returns true when at least one glyph was read.
    def iqra(masar)
      begin
        File.open(masar, "rb") do |malaf|
          malaf.each_line { |satr| satr_wahid(satr) }
        end
      rescue StandardError
        return false
      end
      @adad > 0
    end

    # Parses one line. Unknown leading words are ignored rather than refused,
    # so a table written by a newer Taarib still loads what this build knows.
    def satr_wahid(satr)
      satr = satr.strip
      return if satr.empty? || satr[0, 1] == "#"
      juzuu = satr.split(/\s+/)
      case juzuu[0]
      when "sura"
        return if juzuu.size < 6
        @suwar << [juzuu[1].to_i, juzuu[2].to_s,
                   Color.new(juzuu[3].to_i, juzuu[4].to_i, juzuu[5].to_i, 255)]
      when "shakl"
        return if juzuu.size < 10
        hajm_rubi = juzuu[1].to_i
        muarrif = juzuu[2].to_i
        khatt = juzuu[3].to_i
        khana = @ashkal[hajm_rubi] ||= {}
        khana[(muarrif << 8) | (khatt & 0xFF)] =
          Shakl.new(juzuu[4].to_i, juzuu[5].to_i, juzuu[6].to_i,
                    juzuu[7].to_i, juzuu[8].to_i, juzuu[9].to_i)
        @adad += 1
      end
    end

    # One glyph's rectangle, or nil when the patch did not bake it.
    def shakl(hajm_rubi, muarrif, khatt)
      khana = @ashkal[hajm_rubi]
      if khana.nil?
        @ikhfaqat += 1
        return nil
      end
      natija = khana[(muarrif << 8) | (khatt & 0xFF)]
      @ikhfaqat += 1 if natija.nil?
      natija
    end

    # Every quarter-pixel size the table carries, smallest first. Used to snap
    # a requested size onto one that was actually baked.
    def ahjam
      @ashkal.keys.sort
    end

    # The baked size nearest +hajm_rubi+, or nil when the table is empty.
    #
    # Snapping rather than refusing, because RPG Maker's own windows ask for
    # sizes the patch compiler cannot fully predict — a damage popup shrinks
    # its font, a name box grows it — and a glyph one quarter-pixel off is
    # invisible where a missing glyph is a hole in the sentence.
    def hajm_aqrab(hajm_rubi)
      qaima = ahjam
      return nil if qaima.empty?
      afdal = qaima[0]
      farq = (afdal - hajm_rubi).abs
      qaima.each do |murashah|
        masafa = (murashah - hajm_rubi).abs
        if masafa < farq
          farq = masafa
          afdal = murashah
        end
      end
      afdal
    end

    # The atlas image whose baked colour is nearest +lawn+, loaded on demand.
    #
    # Nearest by squared distance in RGB, which is crude and is the right kind
    # of crude: the layers are a windowskin palette, they are far apart, and a
    # perceptual metric would pick the same layer with more arithmetic per
    # blit.
    def sura(lawn)
      return nil if @suwar.empty?
      afdal = @suwar[0]
      masafa = _masafa(afdal[2], lawn)
      @suwar.each do |murashah|
        hali = _masafa(murashah[2], lawn)
        if hali < masafa
          masafa = hali
          afdal = murashah
        end
      end
      _hammil_sura(afdal)
    end

    # The first baked image, for callers that do not care about colour.
    def sura_asasiya
      return nil if @suwar.empty?
      _hammil_sura(@suwar[0])
    end

    # Releases every loaded image. Called on shutdown and on a reload.
    def ihdham
      @suwar_muhammala.each_value do |sura|
        sura.dispose unless sura.nil? || sura.disposed?
      end
      @suwar_muhammala.clear
    end

    private

    def _masafa(awwal, thani)
      da = awwal.red - thani.red
      dk = awwal.green - thani.green
      dz = awwal.blue - thani.blue
      (da * da) + (dk * dk) + (dz * dz)
    end

    def _hammil_sura(madkhal)
      mahmula = @suwar_muhammala[madkhal[0]]
      return mahmula if !mahmula.nil? && !mahmula.disposed?
      begin
        mahmula = Bitmap.new(madkhal[1])
      rescue StandardError
        return nil
      end
      @suwar_muhammala[madkhal[0]] = mahmula
      mahmula
    end
  end

  #--------------------------------------------------------------------------
  # The drawing side
  #--------------------------------------------------------------------------

  # Lays runs out through the C ABI and blits them from the patch's atlas.
  #
  # A module rather than a class because there is exactly one of it: RGSS3
  # scripts run on one thread, the context is not shareable, and a second
  # instance would mean a second atlas for no benefit.
  module Rassam
    # Unicode ranges whose presence means a run has to be laid out here.
    # Arabic and its extensions, plus Hebrew, Syriac and Thaana, which join or
    # order by the same rules and which the same shaping path handles.
    MADAYAT_YAMEEN = [
      [0x0590, 0x05FF], [0x0600, 0x06FF], [0x0700, 0x074F],
      [0x0750, 0x077F], [0x0780, 0x07BF], [0x08A0, 0x08FF],
      [0xFB50, 0xFDFF], [0xFE70, 0xFEFF]
    ].freeze

    @idad = nil
    @siyaq = nil
    @khatt = nil
    @silsila = nil
    @jadwal = nil
    @jahiz = false
    @athar = []

    class << self
      # The loaded settings, or nil before rakkib.
      attr_reader :idad
      # The glyph table, or nil.
      attr_reader :jadwal
    end

    # Whether the glyph takeover is installed and can actually draw.
    def self.jahiz?
      @jahiz
    end

    # The rung in force. Falls back to the configuration rung when nothing
    # was loaded, which is the rung that changes nothing.
    def self.rutba
      return Rutba::IDAD if @idad.nil?
      @idad.rutba
    end

    # Whether the platform shapes and only direction needs correcting.
    def self.yusahhih?
      !@idad.nil? && !@jahiz && rutba <= Rutba::TASHEEH
    end

    # Every line the adapter recorded, oldest first.
    def self.athar
      @athar.dup
    end

    # Records one line. The only diagnostic channel a player can be asked
    # for, so it records successes as well as failures: "the takeover is
    # installed" is useless on its own and essential next to "the takeover
    # declined because the glyph table was empty".
    #
    # Appended to a file in playtest and kept in memory otherwise.
    # Deliberately not +print+ or +p+: RGSS3 redefines both to open a modal
    # message box, so a dozen log lines during boot would be a dozen dialogs
    # a developer has to dismiss before the game starts.
    def self.sajjil(satr)
      @athar << satr
      return unless $TEST
      begin
        File.open("Taarib/athar.log", "ab") do |malaf|
          malaf.write("#{satr}\n")
        end
      rescue StandardError
        # A read-only game directory must not stop the adapter. The line is
        # still in athar, which is where the diagnostics bundle reads it.
      end
    end

    # Whether a string contains anything this adapter has to lay out.
    def self.fihi_yameen?(nass)
      return false if nass.nil? || nass.empty?
      nass.each_char do |harf|
        raqm = harf.ord
        MADAYAT_YAMEEN.each do |mada|
          return true if raqm >= mada[0] && raqm <= mada[1]
        end
      end
      false
    end

    # Brings the adapter up from the settings file beside Game.exe.
    #
    # Every failure here leaves the game running. The configuration rung is
    # the floor: the font is registered, the direction is corrected, and the
    # reason the takeover declined is recorded. A patch that refuses to load
    # has taken somebody's game away from them.
    def self.rakkib(masar_idad = "Taarib/idad.txt")
      @idad = Idad.iqra(masar_idad)
      @idad.athar.each { |satr| sajjil("probe: #{satr}") }
      sajjil("rung #{@idad.rutba}, probe confidence #{@idad.thiqa}")

      ayyin_khatt(@idad)
      return @idad unless @idad.yastawli?

      begin
        rakkib_istila(@idad)
        @jahiz = true
        sajjil("the glyph takeover is installed; text is drawn by Taarib and is " \
               "not selectable inside the game")
      rescue StandardError => sabab
        # Deliberately broad. A missing DLL, a mismatched ABI, an empty glyph
        # table and a font that will not load all have the same correct
        # response: keep the font, keep the direction, say what happened.
        @jahiz = false
        sajjil("the glyph takeover declined: #{sabab.class}: #{sabab.message}")
      end
      @idad
    end

    # Registers the patch's font with the engine's own default, which is what
    # the correction rung needs and what the takeover falls back to.
    #
    # Font.default_name takes a family name or an array of them, and VX Ace
    # resolves it through the system font list plus whatever is in the game's
    # Fonts/ directory — which is where the patch installs its own file. The
    # name rather than the path is what the engine wants, so the settings
    # carry both and this uses the base name.
    def self.ayyin_khatt(idad_hali)
      return if idad_hali.khatt.empty?
      ism = File.basename(idad_hali.khatt.gsub("\\", "/"), ".*")
      begin
        asli = Font.default_name
        Font.default_name = [ism, asli].flatten.compact
        Font.default_size = idad_hali.hajm if idad_hali.hajm > 0
        sajjil("font default set to #{ism} at size #{Font.default_size}")
      rescue StandardError => sabab
        sajjil("the font could not be registered: #{sabab.message}")
      end
    end

    # The takeover's own bring-up. Raises on any failure; the caller records
    # it and falls back.
    def self.rakkib_istila(idad_hali)
      @jadwal = JadwalAshkal.new
      unless @jadwal.iqra(idad_hali.jadwal)
        raise "the glyph table #{idad_hali.jadwal} is missing or carries no glyphs"
      end
      if @jadwal.suwar.empty?
        raise "the glyph table names no atlas image"
      end
      if @jadwal.sura_asasiya.nil?
        raise "the atlas image #{@jadwal.suwar[0][1]} could not be loaded"
      end
      sajjil("glyph table: #{@jadwal.adad} glyphs across " \
             "#{@jadwal.suwar.size} colour layer(s), sizes " \
             "#{@jadwal.ahjam.join(', ')} quarter-pixels")

      jisr = Taarib.hammil(idad_hali.dalil_jisr)
      sajjil("loaded #{jisr.masar}")
      @siyaq = Taarib.siyaq
      bayt = nil
      File.open(idad_hali.khatt, "rb") { |malaf| bayt = malaf.read }
      @khatt = @siyaq.khatt_min_dhakira(bayt, 0, true)
      @silsila = @siyaq.silsila([@khatt])
      sajjil("font #{@khatt.aila} loaded for layout")
    end

    # Releases everything. Called from a reload and from shutdown; a long
    # session that reloaded scripts without this leaks a context per reload.
    def self.ihdham
      @jahiz = false
      [@silsila, @khatt].each do |maqbad|
        begin
          maqbad.ihdham unless maqbad.nil?
        rescue StandardError
        end
      end
      begin
        @siyaq.ihdham unless @siyaq.nil?
      rescue StandardError
      end
      @jadwal.ihdham unless @jadwal.nil?
      @silsila = nil
      @khatt = nil
      @siyaq = nil
      sajjil("the adapter released its native resources")
    end

    # The layout options every run asks for.
    #
    # Right-to-left and Arabic are forced rather than detected: a run reaches
    # this module *because* it contains Arabic, and automatic detection would
    # lay a line that opens with a name or a number out left to right with the
    # Arabic trailing it.
    KHIYARAT_ARABI = {
      :ittijah => ITTIJAH_YAMEEN,
      :lugha => LUGHA_ARABI,
      :muhadhaha => MUHADHAHA_BIDAYA,
      :tajawuz => TAJAWUZ_BALLAGH,
      :satr_wahid => true
    }.freeze

    # The baked size nearest what the caller asked for, and its pixel value.
    #
    # Layout runs at the *baked* size rather than the requested one, so the
    # advances the ABI reports and the rectangles the atlas holds are metrics
    # of the same size. Laying out at 23.5 and blitting glyphs baked at 24
    # accumulates a fraction of a pixel per glyph, and by the end of a line of
    # dialogue the marks no longer sit on their letters.
    def self.hajm_mabkhi(hajm)
      return nil if @jadwal.nil?
      hajm_rubi = @jadwal.hajm_aqrab((hajm.to_f * 4.0).round)
      return nil if hajm_rubi.nil?
      [hajm_rubi, hajm_rubi / 4.0]
    end

    # A run's size in pixels as [width, height], or nil when it cannot be
    # measured. Measured through the same pipeline that draws it, so the
    # number the window reflows on and the number it draws agree.
    def self.qiyas(nass, hajm)
      return nil unless @jahiz
      mabkhi = hajm_mabkhi(hajm)
      return nil if mabkhi.nil?
      begin
        natija = @siyaq.qis(nass, @silsila, mabkhi[1], KHIYARAT_ARABI)
      rescue StandardError => sabab
        sajjil("a run could not be measured: #{sabab.message}")
        return nil
      end
      [natija.ard.ceil, natija.irtifa.ceil]
    end

    # Lays a run out and blits it into +bitmap+ inside the given rectangle.
    #
    # Returns true when it drew and false when the caller should fall back to
    # the engine's own draw_text — which is what happens for a size the patch
    # did not bake, a colour with no layer, or a layout that raised. Falling
    # back leaves unjoined Arabic on screen, which is bad; drawing nothing
    # leaves a hole, which is worse and is harder to report.
    #
    # Placement convention, stated once because it cannot be read off the
    # ABI headers: a line's pen starts at (satr.bidaya, satr.asas) — the
    # leading edge and the baseline, already in visual order — a glyph's
    # (harf.s, harf.a) is an offset from that pen, and the glyph's bitmap
    # corner is that position plus (izaha_s, -izaha_a).
    def self.arsim(bitmap, x, y, ard, irtifa, nass, muhadhaha = 2)
      return false unless @jahiz
      return false if nass.nil? || nass.empty?
      mabkhi = hajm_mabkhi(bitmap.font.size)
      return false if mabkhi.nil?
      hajm_rubi = mabkhi[0]

      begin
        takhtit = @siyaq.khattit(nass, @silsila, mabkhi[1],
                                 KHIYARAT_ARABI.merge(:ard_mutah => ard.to_f))
      rescue StandardError => sabab
        sajjil("a run could not be laid out: #{sabab.message}")
        return false
      end

      lawn = bitmap.font.color
      sura = @jadwal.sura(lawn)
      return false if sura.nil?
      shaffafiya = lawn.alpha.to_i
      shaffafiya = 255 if shaffafiya <= 0

      # Vertical centring inside the rectangle, which is what Bitmap#draw_text
      # does and therefore what every caller in the engine expects.
      izahat_a = ((irtifa - takhtit.irtifa) / 2.0).round
      izahat_a = 0 if izahat_a < 0
      izahat_s = case muhadhaha
                 when 1 then ((ard - takhtit.ard) / 2.0).round
                 when 2 then (ard - takhtit.ard).round
                 else 0
                 end
      izahat_s = 0 if izahat_s < 0

      rasm = 0
      takhtit.adad_sutur.times do |fahras_satr|
        satr = takhtit.satr(fahras_satr)
        satr.adad_huruf.times do |wahid|
          fahras = satr.awwal_harf + wahid
          break if fahras >= takhtit.adad_huruf
          harf = takhtit.harf(fahras)
          shakl = @jadwal.shakl(hajm_rubi, harf.muarrif, harf.khatt)
          next if shakl.nil? || shakl.ard <= 0 || shakl.irtifa <= 0
          hadaf_s = x + izahat_s + (satr.bidaya + harf.s).round + shakl.izaha_s
          hadaf_a = y + izahat_a + (satr.asas + harf.a).round - shakl.izaha_a
          bitmap.blt(hadaf_s, hadaf_a, sura,
                     Rect.new(shakl.s, shakl.a, shakl.ard, shakl.irtifa),
                     shaffafiya)
          rasm += 1
        end
      end
      rasm > 0 || takhtit.adad_huruf == 0
    end
  end
end

#==============================================================================
# Bitmap — the one call every piece of RGSS3 text goes through
#
# Every window, every sprite and every plugin draws its text with
# Bitmap#draw_text, so replacing it is the whole takeover for anything that is
# not the message window. The original is kept and called for anything this
# adapter does not handle: Latin text, a size the patch did not bake, a run
# that failed to lay out. Falling back is the point — an adapter that owned
# draw_text and could only draw Arabic would blank out every number in the
# game.
#==============================================================================

if defined?(Bitmap) && Bitmap.method_defined?(:draw_text)
  class Bitmap
    unless method_defined?(:taarib_asli_draw_text)
      alias_method :taarib_asli_draw_text, :draw_text
      alias_method :taarib_asli_text_size, :text_size
    end

    # Draws text, taking the run over when it is Arabic and the rung says to.
    #
    # RGSS3 accepts both shapes of this call — five or six positional
    # arguments, or a Rect and a string — and plugins use both, so both are
    # unpacked here rather than one being assumed.
    def draw_text(*muamalat)
      if muamalat.size >= 5
        s = muamalat[0]
        a = muamalat[1]
        ard = muamalat[2]
        irtifa = muamalat[3]
        nass = muamalat[4].to_s
        muhadhaha = muamalat[5] || 0
      elsif muamalat.size >= 2 && muamalat[0].is_a?(Rect)
        mustatil = muamalat[0]
        s = mustatil.x
        a = mustatil.y
        ard = mustatil.width
        irtifa = mustatil.height
        nass = muamalat[1].to_s
        muhadhaha = muamalat[2] || 0
      else
        return taarib_asli_draw_text(*muamalat)
      end

      yameen = Taarib::Rassam.fihi_yameen?(nass)
      # Alignment first, and on every rung: an engine that shapes Arabic
      # correctly still puts the paragraph against the left margin until it is
      # told otherwise, and left-aligned Arabic is wrong even when every letter
      # in it joins.
      muhadhaha = 2 if yameen && muhadhaha == 0
      if yameen && Taarib::Rassam.jahiz?
        return if Taarib::Rassam.arsim(self, s, a, ard, irtifa, nass, muhadhaha)
      end
      taarib_asli_draw_text(s, a, ard, irtifa, nass, muhadhaha)
    end

    # Measures text through whichever pipeline will draw it.
    #
    # This has to agree with draw_text or every window in the game is the
    # wrong width: RGSS3 sizes name boxes, command windows and damage popups
    # from this number.
    def text_size(nass)
      nass = nass.to_s
      if Taarib::Rassam.jahiz? && Taarib::Rassam.fihi_yameen?(nass)
        qiyas = Taarib::Rassam.qiyas(nass, font.size)
        return Rect.new(0, 0, qiyas[0], qiyas[1]) unless qiyas.nil?
      end
      taarib_asli_text_size(nass)
    end
  end
end

#==============================================================================
# Window_Base — the message pipeline
#
# Bitmap#draw_text alone is not enough for the message window, because
# Window_Base does not use it the way everything else does. `draw_text_ex`
# walks the string one character at a time and calls `draw_text` per character
# — which is exactly the shape that cannot join, because a letter's form
# depends on its neighbours and each call sees one letter with no neighbours.
#
# So the run has to be *accumulated* and laid out as a whole. Three things
# follow, and each one is why the code below looks the way it does:
#
#   * Characters are collected instead of drawn, and flushed at an escape
#     code, a newline, a page break, or the end of the string. Those are
#     precisely the boundaries at which a run's context genuinely ends.
#
#   * The accumulated run is **redrawn from scratch on every character**, not
#     appended to. That is what keeps the message window's typewriter effect,
#     and it is also correct in a way appending could never be: adding a letter
#     to an Arabic word changes the *previous* letter's form, so the only right
#     answer is to reshape the whole run. The region is cleared first, and the
#     native layout cache makes the reshape cheap.
#
#   * Runs are placed from a right-hand cursor that moves leftward, so a line
#     broken by escape codes still reads right to left. Markup splits a
#     bidirectional paragraph and this places the pieces by run rather than by
#     the full algorithm — a stated limitation, and one the shaping rung does
#     not have.
#
# The whole interception is skipped when the converted line contains no
# right-to-left text at all, so a Latin line — a number, a file name, an
# untranslated string — goes down the engine's own path untouched.
#==============================================================================

if defined?(Window_Base) && Window_Base.method_defined?(:draw_text_ex)
  class Window_Base
    unless method_defined?(:taarib_asli_draw_text_ex)
      alias_method :taarib_asli_draw_text_ex, :draw_text_ex
      alias_method :taarib_asli_process_character, :process_character
    end

    # Draws formatted text, taking the line over when it is right-to-left.
    def draw_text_ex(s, a, nass)
      unless Taarib::Rassam.jahiz?
        return taarib_asli_draw_text_ex(s, a, nass)
      end
      sabiq_nashit = @taarib_nashit
      sabiq_takdis = @taarib_takdis
      sabiq_yameen = @taarib_yameen
      @taarib_nashit = Taarib::Rassam.fihi_yameen?(nass.to_s)
      @taarib_takdis = ""
      @taarib_ard = 0
      @taarib_yameen = nil
      begin
        natija = taarib_asli_draw_text_ex(s, a, nass)
        taarib_afrigh(@taarib_mawqi) unless @taarib_mawqi.nil?
        natija
      ensure
        # Restored rather than cleared: `draw_text_ex` is re-entrant in a
        # handful of scripts — a choice window drawing an item description
        # inside a message — and a nested call that wiped the outer run's
        # state would lose the sentence the player was reading.
        @taarib_nashit = sabiq_nashit
        @taarib_takdis = sabiq_takdis
        @taarib_yameen = sabiq_yameen
        @taarib_mawqi = nil
      end
    end

    # Collects an ordinary character instead of drawing it, and lets every
    # other kind through to the engine after flushing what was collected.
    def process_character(harf, nass, mawqi)
      unless @taarib_nashit && Taarib::Rassam.jahiz?
        return taarib_asli_process_character(harf, nass, mawqi)
      end
      @taarib_mawqi = mawqi
      if harf != "\n" && harf != "\f" && harf != "\e"
        @taarib_takdis = "" if @taarib_takdis.nil?
        @taarib_takdis += harf
        taarib_arsim_takdis(mawqi)
        return
      end
      taarib_afrigh(mawqi)
      # A newline or a page break ends the line, and with it the right-hand
      # cursor: the next run starts at the right edge again.
      @taarib_yameen = nil if harf == "\n" || harf == "\f"
      taarib_asli_process_character(harf, nass, mawqi)
    end

    # Redraws the run collected so far, in place, reshaped from scratch.
    def taarib_arsim_takdis(mawqi)
      return if @taarib_takdis.nil? || @taarib_takdis.empty?
      irtifa = mawqi[:height] || contents.font.size
      @taarib_yameen = taarib_hafat_yameen(mawqi) if @taarib_yameen.nil?

      qiyas = Taarib::Rassam.qiyas(@taarib_takdis, contents.font.size)
      ard = qiyas.nil? ? contents.text_size(@taarib_takdis).width : qiyas[0]
      ard = 0 if ard < 0

      # Clear the widest the run has ever been on this line, not the width it
      # is now: reshaping can make a run *narrower* — a letter joining to its
      # neighbour is shorter than the two isolated forms — and clearing only
      # the new width would leave the tail of the previous frame on screen.
      masahat = @taarib_ard.to_i
      masahat = ard if ard > masahat
      bidaya = @taarib_yameen - masahat
      bidaya = 0 if bidaya < 0
      arad = @taarib_yameen - bidaya
      contents.clear_rect(bidaya, mawqi[:y], arad, irtifa) if arad > 0

      s = @taarib_yameen - ard
      s = 0 if s < 0
      Taarib::Rassam.arsim(contents, s, mawqi[:y], ard, irtifa,
                           @taarib_takdis, 0)
      @taarib_ard = ard
    end

    # Ends the current run: moves the cursor left past it and forgets it.
    def taarib_afrigh(mawqi)
      return if @taarib_takdis.nil? || @taarib_takdis.empty?
      taarib_arsim_takdis(mawqi) if @taarib_ard.to_i <= 0
      @taarib_yameen = taarib_hafat_yameen(mawqi) if @taarib_yameen.nil?
      @taarib_yameen -= @taarib_ard.to_i
      @taarib_yameen = 0 if @taarib_yameen < 0
      @taarib_takdis = ""
      @taarib_ard = 0
      # The engine's own cursor still has to advance, because everything that
      # is not this adapter — an icon drawn by an escape code, a cursor
      # rectangle, a plugin measuring the line — reads it.
      mawqi[:x] = @taarib_yameen if mawqi.key?(:x)
    end

    # Where a line's right-hand cursor starts.
    #
    # The content width, not the window width: `contents` is the bitmap being
    # drawn into and its width is already the window minus its padding, which
    # is the edge the player sees text end at.
    def taarib_hafat_yameen(mawqi)
      hafa = contents.width
      basita = mawqi[:new_x]
      hafa -= basita if basita.is_a?(Integer) && basita > 0
      hafa = contents.width if hafa <= 0
      hafa
    end
  end
end

#==============================================================================
# Boot
#
# Reading the settings and bringing the adapter up happens at script-load
# time, which in RGSS3 is before the first frame and after Graphics exists.
# Every failure inside is caught and recorded by Rassam itself, so this call
# cannot raise and cannot stop the game from starting — which is the whole
# contract: a patch that refuses to load has taken somebody's game away from
# them.
#==============================================================================

Taarib::Rassam.rakkib("Taarib/idad.txt")
