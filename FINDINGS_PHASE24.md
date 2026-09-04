# Phase 24 findings — frozen

Total: 391 clippy findings
across 10 crates. cargo check, cargo build, cargo deny, tsc and vite build are clean.

## crates/taarib-kashf — 221 findings, 29 files

### crates/taarib-kashf/src/suwar.rs — 31
- warning L1068: multiply and add expressions can be calculated more efficiently and accurately: help: consider using: `0.051_445_995f32.mul_add(azraq, 0.412_221_47 * ahmar + 0.536_332_54 * akhdar)`
- warning L1068: multiply and add expressions can be calculated more efficiently and accurately: help: consider using: `0.412_221_47f32.mul_add(ahmar, 0.536_332_54 * akhdar)`
- warning L1069: multiply and add expressions can be calculated more efficiently and accurately: help: consider using: `0.107_396_96f32.mul_add(azraq, 0.211_903_5 * ahmar + 0.680_699_5 * akhdar)`
- warning L1069: multiply and add expressions can be calculated more efficiently and accurately: help: consider using: `0.211_903_5f32.mul_add(ahmar, 0.680_699_5 * akhdar)`
- warning L1070: multiply and add expressions can be calculated more efficiently and accurately: help: consider using: `0.088_302_46f32.mul_add(ahmar, 0.281_718_85 * akhdar)`
- warning L1070: multiply and add expressions can be calculated more efficiently and accurately: help: consider using: `0.629_978_7f32.mul_add(azraq, 0.088_302_46 * ahmar + 0.281_718_85 * akhdar)`
- warning L1075: multiply and add expressions can be calculated more efficiently and accurately: help: consider using: `0.004_072_047f32.mul_add(-qasir, 0.210_454_26 * tawil + 0.793_617_8 * mutawassit)`
- warning L1075: multiply and add expressions can be calculated more efficiently and accurately: help: consider using: `0.210_454_26f32.mul_add(tawil, 0.793_617_8 * mutawassit)`
- warning L1076: multiply and add expressions can be calculated more efficiently and accurately: help: consider using: `0.450_593_7f32.mul_add(qasir, 1.977_998_5 * tawil - 2.428_592_2 * mutawassit)`
- warning L1076: multiply and add expressions can be calculated more efficiently and accurately: help: consider using: `1.977_998_5f32.mul_add(tawil, -(2.428_592_2 * mutawassit))`
- warning L1077: multiply and add expressions can be calculated more efficiently and accurately: help: consider using: `0.025_904_037f32.mul_add(tawil, 0.782_771_77 * mutawassit)`
- warning L1077: multiply and add expressions can be calculated more efficiently and accurately: help: consider using: `0.808_675_77f32.mul_add(-qasir, 0.025_904_037 * tawil + 0.782_771_77 * mutawassit)`
- warning L1094: multiply and add expressions can be calculated more efficiently and accurately: help: consider using: `0.215_803_76f32.mul_add(self.b, self.l + 0.396_337_78 * self.a)`
- warning L1094: multiply and add expressions can be calculated more efficiently and accurately: help: consider using: `0.396_337_78f32.mul_add(self.a, self.l)`
- warning L1095: multiply and add expressions can be calculated more efficiently and accurately: help: consider using: `0.063_854_17f32.mul_add(-self.b, self.l - 0.105_561_346 * self.a)`
- warning L1095: multiply and add expressions can be calculated more efficiently and accurately: help: consider using: `0.105_561_346f32.mul_add(-self.a, self.l)`
- warning L1096: multiply and add expressions can be calculated more efficiently and accurately: help: consider using: `0.089_484_18f32.mul_add(-self.a, self.l)`
- warning L1096: multiply and add expressions can be calculated more efficiently and accurately: help: consider using: `1.291_485_5f32.mul_add(-self.b, self.l - 0.089_484_18 * self.a)`
- warning L1102: multiply and add expressions can be calculated more efficiently and accurately: help: consider using: `0.230_969_94f32.mul_add(qasir, 4.076_741_7 * tawil - 3.307_711_6 * mutawassit)`
- warning L1102: multiply and add expressions can be calculated more efficiently and accurately: help: consider using: `4.076_741_7f32.mul_add(tawil, -(3.307_711_6 * mutawassit))`
- warning L1103: multiply and add expressions can be calculated more efficiently and accurately: help: consider using: `(-1.268_438f32).mul_add(tawil, 2.609_757_4 * mutawassit)`
- warning L1103: multiply and add expressions can be calculated more efficiently and accurately: help: consider using: `0.341_319_38f32.mul_add(-qasir, -1.268_438 * tawil + 2.609_757_4 * mutawassit)`
- warning L1104: multiply and add expressions can be calculated more efficiently and accurately: help: consider using: `(-0.004_196_086_3f32).mul_add(tawil, -(0.703_418_6 * mutawassit))`
- warning L1104: multiply and add expressions can be calculated more efficiently and accurately: help: consider using: `1.707_614_7f32.mul_add(qasir, -0.004_196_086_3 * tawil - 0.703_418_6 * mutawassit)`
- warning L1126: multiply and add expressions can be calculated more efficiently and accurately: help: consider using: `db.mul_add(db, di * di + da * da)`
- warning L1126: multiply and add expressions can be calculated more efficiently and accurately: help: consider using: `di.mul_add(di, da * da)`
- warning L1141: multiply and add expressions can be calculated more efficiently and accurately: help: consider using: `1.055f32.mul_add(qeema.powf(1.0 / 2.4), -0.055)`
- warning L1204: this could be a `const fn`
- warning L1390: multiply and add expressions can be calculated more efficiently and accurately: help: consider using: `(1.0 - UQUBAT_HALA).mul_add(masafa / NITAQ_ILTIBAS, UQUBAT_HALA)`
- warning L1491: this could be a `const fn`
- warning L1619: consider using `sort_by_key`

### crates/taarib-kashf/src/matajir/lutris.rs — 20
- warning L35: item in documentation is missing backticks
- warning L418: use of a disallowed method `std::env::var_os`
- warning L436: item in documentation is missing backticks
- warning L469: item in documentation is missing backticks
- warning L481: item in documentation is missing backticks
- warning L483: item in documentation is missing backticks
- warning L497: `format!(..)` appended to existing `String`
- warning L512: item in documentation is missing backticks
- warning L595: item in documentation is missing backticks
- warning L602: redundant closure: help: replace the closure with the method itself: `str::to_ascii_lowercase`
- warning L986: unnecessary closure used with `bool::then`
- warning L1013: you seem to be trying to use `match` for destructuring a single pattern. Consider using `if let`
- warning L1040: you seem to be trying to use `match` for destructuring a single pattern. Consider using `if let`
- warning L1123: redundant clone: help: remove this
- warning L1360: unnecessary structure name repetition: help: use the applicable keyword: `Self`
- warning L1510: item in documentation is missing backticks
- warning L1764: you seem to be trying to use `match` for destructuring a single pattern. Consider using `if let`
- warning L1772: you seem to be trying to use `match` for destructuring a single pattern. Consider using `if let`
- warning L1835: item in documentation is missing backticks
- warning L1930: item in documentation is missing backticks

### crates/taarib-kashf/src/beea.rs — 16
- warning L335: usage of `contains_key` followed by `insert` on a `BTreeMap`
- warning L464: this `if` statement can be collapsed
- warning L465: this `if` statement can be collapsed
- warning L474: this `if` statement can be collapsed
- warning L539: this `if` statement can be collapsed
- warning L540: this `if` statement can be collapsed
- warning L730: you seem to be trying to use `match` for destructuring a single pattern. Consider using `if let`
- warning L878: unnecessary qualification
- warning L889: you seem to be trying to use `match` for destructuring a single pattern. Consider using `if let`
- warning L1038: this argument is passed by value, but not consumed in the function body: help: consider changing the type to: `&str`
- warning L1125: this `if` statement can be collapsed
- warning L1134: this `if` statement can be collapsed
- warning L1412: item in documentation is missing backticks
- warning L1434: use of a disallowed method `std::env::var_os`
- warning L1577: use of a disallowed method `std::env::var_os`
- warning L1578: use of a disallowed method `std::env::var_os`

### crates/taarib-kashf/src/matajir/playnite.rs — 16
- warning L11: item in documentation is missing backticks
- warning L12: item in documentation is missing backticks
- warning L13: item in documentation is missing backticks
- warning L19: item in documentation is missing backticks
- warning L36: item in documentation is missing backticks
- warning L127: item in documentation is missing backticks
- warning L140: item in documentation is missing backticks
- warning L313: this could be a `const fn`
- warning L318: this could be a `const fn`
- warning L323: this could be a `const fn`
- warning L443: item in documentation is missing backticks
- warning L466: use of a disallowed method `std::env::var_os`
- warning L473: use of a disallowed method `std::env::var_os`
- warning L642: item in documentation is missing backticks
- warning L658: item in documentation is missing backticks
- warning L1432: unnecessary closure used with `bool::then`

### crates/taarib-kashf/src/matajir/itch.rs — 15
- warning L3: item in documentation is missing backticks
- warning L23: item in documentation is missing backticks
- warning L38: item in documentation is missing backticks
- warning L47: item in documentation is missing backticks
- warning L48: item in documentation is missing backticks
- warning L171: item in documentation is missing backticks
- warning L244: use of a disallowed method `std::env::var_os`
- warning L296: item in documentation is missing backticks
- warning L309: item in documentation is missing backticks
- warning L311: item in documentation is missing backticks
- warning L315: item in documentation is missing backticks
- warning L326: `format!(..)` appended to existing `String`
- warning L341: item in documentation is missing backticks
- warning L456: item in documentation is missing backticks
- warning L462: redundant closure: help: replace the closure with the method itself: `str::to_ascii_lowercase`

### crates/taarib-kashf/src/matajir/steam.rs — 15
- warning L277: this could be a `const fn`
- warning L491: redundant clone: help: remove this
- warning L654: item in documentation is missing backticks
- warning L658: item in documentation is missing backticks
- warning L659: item in documentation is missing backticks
- warning L717: this could be a `const fn`
- warning L779: redundant clone: help: remove this
- warning L810: this `if` statement can be collapsed
- warning L858: item in documentation is missing backticks
- warning L988: redundant clone: help: remove this
- warning L1420: item in documentation is missing backticks
- warning L1566: redundant clone: help: remove this
- warning L1679: redundant clone: help: remove this
- warning L1708: this `if` statement can be collapsed
- warning L1855: case-sensitive file extension comparison

### crates/taarib-kashf/src/matajir/gog.rs — 14
- warning L4: item in documentation is missing backticks
- warning L30: item in documentation is missing backticks
- warning L35: item in documentation is missing backticks
- warning L38: item in documentation is missing backticks
- warning L40: item in documentation is missing backticks
- warning L274: use of a disallowed method `std::env::var_os`
- warning L331: item in documentation is missing backticks
- warning L333: item in documentation is missing backticks
- warning L346: `format!(..)` appended to existing `String`
- warning L676: this argument is passed by value, but not consumed in the function body: help: consider changing the type to: `&str`
- warning L774: field `muarrif` is never read
- warning L799: case-sensitive file extension comparison
- warning L890: this could be a `const fn`
- warning L928: assigning the result of `Clone::clone()` may be inefficient: help: use `clone_from()`: `mawjud.bina_manassa.clone_from(&tathbeet.isdar)`

### crates/taarib-kashf/src/matajir/amazon.rs — 11
- warning L1: item in documentation is missing backticks
- warning L27: item in documentation is missing backticks
- warning L33: item in documentation is missing backticks
- warning L406: use of a disallowed method `std::env::var_os`
- warning L457: item in documentation is missing backticks
- warning L471: item in documentation is missing backticks
- warning L473: item in documentation is missing backticks
- warning L486: `format!(..)` appended to existing `String`
- warning L536: item in documentation is missing backticks
- warning L539: item in documentation is missing backticks
- warning L1085: this argument is passed by value, but not consumed in the function body: help: consider changing the type to: `&str`

### crates/taarib-kashf/src/ayquna.rs — 10
- warning L515: consider using `sort_by_key`
- warning L1032: manual saturating arithmetic: help: consider using `saturating_mul`: `fahras.saturating_mul(HAJM_QAYD_MAJMUA)`
- warning L1056: manual saturating arithmetic: help: consider using `saturating_mul`: `mukhtara.len().saturating_mul(HAJM_QAYD_ICO)`
- warning L1297: manual `!RangeInclusive::contains` implementation: help: use: `!(HAJM_TARWISAT_DIB..=1024).contains(&hajm_tarwisa)`
- warning L2035: redundant clone: help: remove this
- warning L2136: function call inside of `unwrap_or`: help: try: `unwrap_or_else(|| nass.as_ref())`
- warning L2535: use of a disallowed method `std::env::var_os`
- warning L2538: use of a disallowed method `std::env::var_os`
- warning L2544: use of a disallowed method `std::env::var_os`
- warning L2668: these match arms have identical bodies

### crates/taarib-kashf/src/matajir/vdf.rs — 9
- warning L3: item in documentation is missing backticks
- warning L178: unnecessary structure name repetition: help: use the applicable keyword: `Self`
- warning L234: this could be a `const fn`
- warning L270: this could be a `const fn`
- warning L270: unnecessary structure name repetition: help: use the applicable keyword: `Self`
- warning L297: unnecessary structure name repetition: help: use the applicable keyword: `Self`
- warning L323: you appear to be counting bytes the naive way: help: consider using the bytecount crate: `bytecount::count(sabiq, b'\n')`
- warning L365: this loop could be written as a `while let` loop: help: try: `while let Some((mawdi, wahda)) = qari.wahda(masar)? { .. }`
- warning L1010: variables can be used directly in the `format!` string

### crates/taarib-kashf/src/matajir/ea.rs — 7
- warning L269: use of a disallowed method `std::env::var_os`
- warning L291: use of a disallowed method `std::env::var_os`
- warning L293: use of a disallowed method `std::env::var_os`
- warning L331: use of deprecated method `quick_xml::events::attributes::Attribute::<'a>::unescape_value`: use `Self::normalized_value()`
- warning L363: redundant closure: help: replace the closure with the method itself: `walkdir::DirEntry::into_path`
- warning L703: use of deprecated method `quick_xml::events::attributes::Attribute::<'a>::unescape_value`: use `Self::normalized_value()`
- warning L782: redundant clone: help: remove this

### crates/taarib-kashf/src/fann_luba.rs — 6
- warning L341: consider using `sort_by_key`
- warning L491: case-sensitive file extension comparison
- warning L501: case-sensitive file extension comparison
- warning L619: case-sensitive file extension comparison
- warning L731: case-sensitive file extension comparison
- warning L873: case-sensitive file extension comparison

### crates/taarib-kashf/src/wujud.rs — 6
- warning L66: constructing a `Duration` using a smaller unit when a larger unit would be more readable
- warning L262: this manual char comparison can be written more succinctly: help: consider using an array of `char`: `['\\', '/']`
- warning L315: this `if` statement can be collapsed
- warning L346: this `let...else` may be rewritten with the `?` operator: help: replace it with: `let qurs = jidhr_al_hajm(jidhr)?;`
- warning L469: this `if` statement can be collapsed
- warning L502: this could be a `const fn`

### crates/taarib-kashf/src/lib.rs — 5
- warning L96: item in documentation is missing backticks
- warning L115: unused import: `MasdarLuba`
- warning L190: this function's return value is unnecessarily wrapped by `Result`
- warning L248: this could be a `const fn`
- warning L248: this function's return value is unnecessarily wrapped by `Result`

### crates/taarib-kashf/src/matajir/rockstar.rs — 5
- warning L190: item in documentation is missing backticks
- warning L382: this could be a `const fn`
- warning L394: this could be a `const fn`
- warning L415: this could be a `const fn`
- warning L691: called `map(<f>).unwrap_or(<a>)` on a `Result` value

### crates/taarib-kashf/src/matajir/ubisoft.rs — 5
- warning L352: this could be a `const fn`
- warning L365: use of a disallowed method `std::env::var_os`
- warning L381: this could be a `const fn`
- warning L423: called `map(<f>).unwrap_or(<a>)` on a `Result` value
- warning L563: case-sensitive file extension comparison

### crates/taarib-kashf/src/matajir/xbox.rs — 4
- warning L54: item in documentation is missing backticks
- warning L259: use of a disallowed method `std::env::var_os`
- warning L323: this could be a `const fn`
- warning L775: use of deprecated method `quick_xml::events::attributes::Attribute::<'a>::unescape_value`: use `Self::normalized_value()`

### crates/taarib-kashf/src/tahdith.rs — 4
- warning L129: this could be a `const fn`
- warning L135: this could be a `const fn`
- warning L399: constructing a `Duration` using a smaller unit when a larger unit would be more readable
- warning L595: this could be a `const fn`

### crates/taarib-kashf/src/matajir/battlenet.rs — 3
- warning L308: use of a disallowed method `std::env::var_os`
- warning L616: redundant closure: help: replace the closure with the method itself: `char::is_control`
- warning L697: you seem to be trying to use `match` for destructuring a single pattern. Consider using `if let`

### crates/taarib-kashf/src/matajir/legendary.rs — 3
- warning L133: item in documentation is missing backticks
- warning L404: use of a disallowed method `std::env::var_os`
- warning L477: assigning the result of `ToOwned::to_owned()` may be inefficient: help: use `clone_into()`: `ism.trim().clone_into(&mut hali)`

### crates/taarib-kashf/src/matajir/mahmul.rs — 3
- warning L834: adding items after statements is confusing, since items exist from the start of the scope
- warning L925: adding items after statements is confusing, since items exist from the start of the scope
- warning L1148: called `map(<f>).unwrap_or(<a>)` on a `Result` value

### crates/taarib-kashf/src/tawheed.rs — 3
- warning L253: this could be a `const fn`
- warning L259: this could be a `const fn`
- warning L337: you seem to be trying to use `match` for destructuring a single pattern. Consider using `if let`

### crates/taarib-kashf/src/fahs.rs — 2
- warning L113: you are deriving `PartialEq` and can implement `Eq`: help: consider deriving `Eq` as well: `PartialEq, Eq`
- warning L294: unused `self` argument

### crates/taarib-kashf/src/matajir/heroic.rs — 2
- warning L242: use of a disallowed method `std::env::var_os`
- warning L293: this could be a `const fn`

### crates/taarib-kashf/src/matajir/yadawi.rs — 2
- warning L468: function call inside of `unwrap_or`: help: try: `unwrap_or_else(|| nass.as_ref())`
- warning L803: adding items after statements is confusing, since items exist from the start of the scope

### crates/taarib-kashf/src/matajir/bottles.rs — 1
- warning L322: use of a disallowed method `std::env::var_os`

### crates/taarib-kashf/src/matajir/epic.rs — 1
- warning L308: use of a disallowed method `std::env::var_os`

### crates/taarib-kashf/src/matajir/mod.rs — 1
- warning L16: item in documentation is missing backticks

### crates/taarib-kashf/src/matajir/riot.rs — 1
- warning L435: use of a disallowed method `std::env::var_os`

## crates/taarib-tarjama — 60 findings, 6 files

### crates/taarib-tarjama/src/dhakira.rs — 21
- warning L188: first doc comment paragraph is too long
- warning L210: these match arms have identical bodies
- warning L338: unexpected `cfg` condition value: `wajiha`: help: remove the condition
- warning L339: unexpected `cfg` condition value: `mukhattatat`: help: remove the condition
- warning L573: unexpected `cfg` condition value: `wajiha`: help: remove the condition
- warning L574: unexpected `cfg` condition value: `mukhattatat`: help: remove the condition
- warning L608: unexpected `cfg` condition value: `wajiha`: help: remove the condition
- warning L609: unexpected `cfg` condition value: `mukhattatat`: help: remove the condition
- warning L629: unexpected `cfg` condition value: `wajiha`: help: remove the condition
- warning L630: unexpected `cfg` condition value: `mukhattatat`: help: remove the condition
- warning L747: unexpected `cfg` condition value: `wajiha`: help: remove the condition
- warning L748: unexpected `cfg` condition value: `mukhattatat`: help: remove the condition
- warning L770: more than 1 bools in function parameters
- warning L789: unexpected `cfg` condition value: `wajiha`: help: remove the condition
- warning L790: unexpected `cfg` condition value: `mukhattatat`: help: remove the condition
- warning L809: unused `self` argument
- warning L835: unexpected `cfg` condition value: `wajiha`: help: remove the condition
- warning L836: unexpected `cfg` condition value: `mukhattatat`: help: remove the condition
- warning L1015: first doc comment paragraph is too long
- warning L1021: first doc comment paragraph is too long
- warning L1559: this parameter is a mutable reference but is not used mutably

### crates/taarib-tarjama/src/dufaat.rs — 16
- warning L74: use of a disallowed type `std::sync::Mutex`
- warning L628: use of a disallowed type `std::sync::Mutex`
- warning L635: use of a disallowed type `std::sync::Mutex`
- warning L637: use of a disallowed type `std::sync::Mutex`
- warning L639: use of a disallowed type `std::sync::Mutex`
- warning L641: use of a disallowed type `std::sync::Mutex`
- warning L759: temporary with significant `Drop` in `if let` scrutinee will live until the end of the `if let` expression
- warning L853: this lint expectation is unfulfilled
- warning L877: assigning the result of `Clone::clone()` may be inefficient: help: use `clone_from()`: `muswadda.nasq_hadaf.clone_from(&mustaad.nasq)`
- warning L905: temporary with significant `Drop` in `if let` scrutinee will live until the end of the `if let` expression
- warning L933: temporary with significant `Drop` in `if let` scrutinee will live until the end of the `if let` expression
- warning L1297: use of a disallowed type `std::sync::Mutex`
- warning L1298: use of a disallowed type `std::sync::Mutex`
- warning L1299: use of a disallowed type `std::sync::Mutex`
- warning L1307: use of a disallowed type `std::sync::Mutex`
- warning L1400: assigning the result of `Clone::clone()` may be inefficient: help: use `clone_from()`: `mudkhal.nasq_hadaf.clone_from(nasq)`

### crates/taarib-tarjama/src/masrad.rs — 12
- warning L105: unexpected `cfg` condition value: `wajiha`: help: remove the condition
- warning L106: unexpected `cfg` condition value: `mukhattatat`: help: remove the condition
- warning L147: unexpected `cfg` condition value: `wajiha`: help: remove the condition
- warning L148: unexpected `cfg` condition value: `mukhattatat`: help: remove the condition
- warning L617: assigning the result of `ToOwned::to_owned()` may be inefficient: help: use `clone_into()`: `mudkhal.masdar.trim().clone_into(&mut tarshih.masdar_khaam)`
- warning L627: assigning the result of `ToOwned::to_owned()` may be inefficient: help: use `clone_into()`: `hadaf.trim().clone_into(&mut sura.khaam)`
- warning L966: unexpected `cfg` condition value: `wajiha`: help: remove the condition
- warning L967: unexpected `cfg` condition value: `mukhattatat`: help: remove the condition
- warning L979: unexpected `cfg` condition value: `wajiha`: help: remove the condition
- warning L980: unexpected `cfg` condition value: `mukhattatat`: help: remove the condition
- warning L1002: unexpected `cfg` condition value: `wajiha`: help: remove the condition
- warning L1003: unexpected `cfg` condition value: `mukhattatat`: help: remove the condition

### crates/taarib-tarjama/src/siyaq.rs — 6
- warning L374: `format!(..)` appended to existing `String`
- warning L393: `format!(..)` appended to existing `String`
- warning L400: `format!(..)` appended to existing `String`
- warning L467: `format!(..)` appended to existing `String`
- warning L472: `format!(..)` appended to existing `String`
- warning L525: `format!(..)` appended to existing `String`

### crates/taarib-tarjama/src/muzawwidun.rs — 3
- warning L488: this argument is passed by value, but not consumed in the function body
- warning L640: empty line after doc comment
- warning L742: this argument is passed by value, but not consumed in the function body

### crates/taarib-tarjama/src/khata.rs — 2
- warning L308: these match arms have identical bodies: the wildcard arm
- warning L376: these match arms have identical bodies

## crates/taarib-tabaqa — 35 findings, 6 files

### crates/taarib-tabaqa/src/vulkan.rs — 26
- warning L963: unnecessary `unsafe` block: unnecessary `unsafe` block
- warning L1097: unnecessary `unsafe` block: unnecessary `unsafe` block
- warning L1559: trivial cast: `extern "system" fn(*const ..., ..., ...) -> ... {insha_mithal}` as `unsafe extern "system" fn(*const ..., ..., ...) -> ...`
- warning L1559: trivial cast: `extern "system" fn(Device, *const ...) {itlaf_jihaz}` as `for<'a> unsafe extern "system" fn(ash::vk::Device, *const ash::vk::AllocationCallbacks<'a>)`
- warning L1559: trivial cast: `extern "system" fn(Device, *const i8) -> ... {vkGetDeviceProcAddr}` as `unsafe extern "system" fn(Device, *const i8) -> Option<...>`
- warning L1559: trivial cast: `extern "system" fn(Device, ..., ..., ...) -> ... {insha_silsila}` as `unsafe extern "system" fn(Device, *const ..., ..., ...) -> ...`
- warning L1559: trivial cast: `extern "system" fn(Device, SwapchainKHR, *const ...) {itlaf_silsila}` as `unsafe extern "system" fn(Device, SwapchainKHR, *const ...)`
- warning L1559: trivial cast: `extern "system" fn(Instance, *const ...) {itlaf_mithal}` as `for<'a> unsafe extern "system" fn(ash::vk::Instance, *const ash::vk::AllocationCallbacks<'a>)`
- warning L1559: trivial cast: `extern "system" fn(Instance, *const i8) -> ... {vkGetInstanceProcAddr}` as `unsafe extern "system" fn(Instance, *const i8) -> Option<...>`
- warning L1559: trivial cast: `extern "system" fn(PhysicalDevice, ..., ..., ...) -> ... {insha_jihaz}` as `unsafe extern "system" fn(PhysicalDevice, *const ..., ..., ...) -> ...`
- warning L1559: trivial cast: `extern "system" fn(Queue, *const PresentInfoKHR<'a>) -> ... {taqdeem}` as `unsafe extern "system" fn(Queue, *const PresentInfoKHR<'a>) -> Result`
- warning L1559: trivial cast: `extern "system" fn(ash::vk::Device, u32, u32, *mut ash::vk::Queue) {vulkan::jib_tabur}` as `unsafe extern "system" fn(ash::vk::Device, u32, u32, *mut ash::vk::Queue)`
- warning L1622: this `let...else` may be rewritten with the `?` operator: help: replace it with: `let jadwal = (unsafe { mithal_min_maqbad(mithal.as_raw()) })?;`
- warning L1645: this `let...else` may be rewritten with the `?` operator: help: replace it with: `let jadwal = (unsafe { jihaz_min_maqbad(jihaz.as_raw()) })?;`
- warning L1664: this lint expectation is unfulfilled
- warning L1681: this lint expectation is unfulfilled
- warning L1701: this lint expectation is unfulfilled
- warning L1714: this lint expectation is unfulfilled
- warning L1744: this lint expectation is unfulfilled
- warning L1756: this lint expectation is unfulfilled
- warning L1789: trivial cast: `extern "system" fn(Instance, *const i8) -> ... {vkGetInstanceProcAddr}` as `unsafe extern "system" fn(Instance, *const i8) -> Option<...>`
- warning L1790: trivial cast: `extern "system" fn(Device, *const i8) -> ... {vkGetDeviceProcAddr}` as `unsafe extern "system" fn(Device, *const i8) -> Option<...>`
- warning L2700: function call inside of `ok_or`
- warning L2898: function call inside of `ok_or`
- warning L3826: useless conversion to the same type: `u32`
- warning L3827: useless conversion to the same type: `u32`

### crates/taarib-tabaqa/src/gl.rs — 3
- warning L495: unused doc comment
- warning L794: manual `Debug` impl does not include all fields
- warning L1174: more than 3 bools in a struct

### crates/taarib-tabaqa/src/khata.rs — 2
- warning L473: these match arms have identical bodies
- warning L484: these match arms have identical bodies

### crates/taarib-tabaqa/src/manatiq.rs — 2
- warning L249: this argument (16 byte) is passed by reference, but would be more efficient if passed by value (limit: 16 byte): help: consider passing by value instead: `MustatilNisbi`
- warning L1007: assigning the result of `ToOwned::to_owned()` may be inefficient: help: use `clone_into()`: `mintaqa.ism.trim().clone_into(&mut mintaqa.ism)`

### crates/taarib-tabaqa/src/lawhat_tahakkum.rs — 1
- warning L1570: assigning the result of `ToOwned::to_owned()` may be inefficient: help: use `clone_into()`: `majmua.ism_luba().clone_into(&mut self.ism_luba)`

### crates/taarib-tabaqa/src/qira.rs — 1
- warning L298: `if _ { .. } else { .. }` is an expression

## crates/taarib-jisr — 21 findings, 6 files

### crates/taarib-jisr/src/awamir.rs — 6
- warning L210: redundant closure: help: replace the closure with the method itself: `std::option::Option::cloned`
- warning L266: first doc comment paragraph is too long
- warning L312: first doc comment paragraph is too long
- warning L344: first doc comment paragraph is too long
- warning L376: first doc comment paragraph is too long
- warning L702: first doc comment paragraph is too long

### crates/taarib-jisr/src/anwa.rs — 5
- warning L50: trailing zero-sized array in a struct which is not marked with a `repr` attribute
- warning L59: trailing zero-sized array in a struct which is not marked with a `repr` attribute
- warning L68: trailing zero-sized array in a struct which is not marked with a `repr` attribute
- warning L77: trailing zero-sized array in a struct which is not marked with a `repr` attribute
- warning L115: first doc comment paragraph is too long

### crates/taarib-jisr/src/khata_c.rs — 5
- warning L104: this function could have a `#[must_use]` attribute
- warning L115: this function could have a `#[must_use]` attribute
- warning L169: this could be a `const fn`
- warning L195: this could be a `const fn`
- warning L263: this could be a `const fn`

### crates/taarib-jisr/src/khazina.rs — 3
- warning L471: first doc comment paragraph is too long
- warning L854: this could be a `const fn`
- warning L872: this could be a `const fn`

### crates/taarib-jisr/src/dhakira.rs — 1
- warning L466: manual implementation of `.is_multiple_of()`: help: replace with: `!muashir.addr().is_multiple_of(align_of::<T>())`

### crates/taarib-jisr/src/lib.rs — 1
- warning L10: item in documentation is missing backticks

## crates/taarib-ruqaa — 16 findings, 5 files

### crates/taarib-ruqaa/src/katib.rs — 7
- warning L423: function call inside of `ok_or`
- warning L641: function call inside of `ok_or`
- warning L681: function call inside of `ok_or`
- warning L792: function call inside of `ok_or`
- warning L833: function call inside of `ok_or`
- warning L881: function call inside of `ok_or`
- warning L904: function call inside of `ok_or`

### crates/taarib-ruqaa/src/qari.rs — 4
- warning L199: function call inside of `ok_or`: help: try: `ok_or_else(|| KhataRuqaa::QismMafqud { ism: naw.ism() })`
- warning L811: function call inside of `ok_or`
- warning L955: function call inside of `ok_or`
- warning L960: function call inside of `ok_or`

### crates/taarib-ruqaa/src/jadawil.rs — 2
- warning L490: first doc comment paragraph is too long
- warning L929: first doc comment paragraph is too long

### crates/taarib-ruqaa/src/tarwisa.rs — 2
- warning L443: function call inside of `ok_or`
- warning L466: function call inside of `ok_or`

### crates/taarib-ruqaa/src/muhadhah.rs — 1
- warning L82: manual `Debug` impl does not include all fields

## crates/taarib-wasm — 15 findings, 5 files

### crates/taarib-wasm/src/khatt_js.rs — 4
- warning L106: unnecessary structure name repetition: help: use the applicable keyword: `Self`
- warning L122: unnecessary structure name repetition: help: use the applicable keyword: `Self`
- warning L180: unnecessary structure name repetition: help: use the applicable keyword: `Self`
- warning L215: unnecessary structure name repetition: help: use the applicable keyword: `Self`

### crates/taarib-wasm/src/saff_js.rs — 4
- warning L14: item in documentation is missing backticks
- warning L56: item in documentation is missing backticks
- warning L56: item in documentation is missing backticks
- warning L87: unnecessary structure name repetition: help: use the applicable keyword: `Self`

### crates/taarib-wasm/src/talab_js.rs — 3
- warning L184: unnecessary structure name repetition: help: use the applicable keyword: `Self`
- warning L256: unnecessary structure name repetition: help: use the applicable keyword: `Self`
- warning L400: unnecessary structure name repetition: help: use the applicable keyword: `Self`

### crates/taarib-wasm/src/lawha_js.rs — 2
- warning L93: unnecessary structure name repetition: help: use the applicable keyword: `Self`
- warning L370: unnecessary structure name repetition: help: use the applicable keyword: `Self`

### crates/taarib-wasm/src/natija_js.rs — 2
- warning L278: code link adjacent to code text
- warning L287: code link adjacent to code text

## crates/taarib-mustalahat — 10 findings, 5 files

### crates/taarib-mustalahat/src/luba.rs — 4
- warning L165: these match arms have identical bodies
- warning L189: these match arms have identical bodies
- warning L216: these match arms have identical bodies
- warning L247: these match arms have identical bodies

### crates/taarib-mustalahat/src/lawha_badila.rs — 2
- warning L261: 6 bindings with single-character names in scope
- warning L386: all if blocks contain the same code at the end

### crates/taarib-mustalahat/src/muraja.rs — 2
- warning L259: this argument is passed by value, but not consumed in the function body
- warning L270: this argument is passed by value, but not consumed in the function body

### crates/taarib-mustalahat/src/nass.rs — 1
- warning L371: empty line after doc comment

### crates/taarib-mustalahat/src/sawt.rs — 1
- warning L236: unused `self` argument

## crates/taarib-khatm — 6 findings, 1 files

### crates/taarib-khatm/src/malik.rs — 6
- error L55: `panic` should not be present in production code
- error L65: indexing may panic
- error L65: indexing may panic
- error L65: indexing may panic
- error L74: indexing may panic
- error L74: indexing may panic

## crates/taarib-mudkhal — 6 findings, 1 files

### crates/taarib-mudkhal/src/hamula.rs — 6
- warning L10: pub(crate) constant inside private module
- warning L18: pub(crate) constant inside private module
- warning L21: pub(crate) constant inside private module
- warning L32: pub(crate) function inside private module
- warning L48: pub(crate) function inside private module
- warning L61: pub(crate) function inside private module

## crates/taarib-usus — 1 findings, 1 files

### crates/taarib-usus/src/masarat.rs — 1
- warning L866: these match arms have identical bodies
