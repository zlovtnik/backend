use quick_xml::events::Event;
use quick_xml::Reader;
use validator::ValidationError;

/// Validates that a string contains well-formed XML.
///
/// This is a pragmatic validation step to ensure `xml_content` payloads are parseable.
/// Full XSD validation is handled separately (if/when an XSD validation engine is introduced).
pub fn validate_xml(xml: &str) -> Result<(), ValidationError> {
    if xml.trim().is_empty() {
        return Err(ValidationError::new("empty_xml"));
    }

    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(e) => {
                let mut err = ValidationError::new("invalid_xml");
                err.message = Some(format!("XML parse error: {}", e).into());
                return Err(err);
            }
        }
        buf.clear();
    }

    Ok(())
}
