//! General entity references, resolved the one way for every XML reader here.

use std::borrow::Cow;

use quick_xml::events::BytesRef;

/// The replacement text for a general entity reference, when one is defined.
///
/// quick-xml stopped folding entity references into the surrounding text event
/// and now delivers each one as its own [`quick_xml::events::Event::GeneralRef`].
/// A reader that matches only on text therefore drops every `&amp;` in the
/// document without saying so, which is why this lives in one place: four
/// crates read XML, and four private copies of an entity table is four chances
/// to disagree about what `&apos;` means.
///
/// Resolved: the five entities XML predefines, and numeric character
/// references in either base. Everything else — an entity a DTD declared, or
/// one nothing declared — is [`None`], because this crate has no document to
/// look it up in. The caller decides whether that is a refusal or a passthrough.
#[must_use]
pub fn hall_marja(marja: &BytesRef<'_>) -> Option<Cow<'static, str>> {
    if marja.is_char_ref() {
        return match marja.resolve_char_ref() {
            Ok(Some(harf)) => Some(Cow::Owned(harf.to_string())),
            Ok(None) | Err(_) => None,
        };
    }
    quick_xml::escape::resolve_xml_entity(marja.xml10_content().as_ref()).map(Cow::Borrowed)
}

/// The reference as it was written, `&` and `;` included.
///
/// What a reader puts back when it is preserving markup verbatim rather than
/// interpreting it, and what an error message quotes when it refuses one.
#[must_use]
pub fn nass_marja(marja: &BytesRef<'_>) -> String {
    format!("&{};", marja.xml10_content())
}
