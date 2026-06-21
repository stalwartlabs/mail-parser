use mail_parser::{MessageParser, PartType};
use std::fs;

fn main() {
    let dir = "/tmp/tbtest";
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().map_or(true, |e| e != "eml") { continue; }
        let raw = fs::read(&path).unwrap();
        let msg = MessageParser::default().parse(&raw).unwrap();
        println!("\n=== {} ===", path.file_name().unwrap().to_string_lossy());
        println!("text_body={:?} html_body={:?} attachments={:?}", msg.text_body, msg.html_body, msg.attachments);
        for (i, p) in msg.parts.iter().enumerate() {
            let kind = match &p.body {
                PartType::Text(t) => format!("Text({} bytes, starts {:?})", t.len(), &t.chars().take(40).collect::<String>()),
                PartType::Html(t) => format!("Html({} bytes)", t.len()),
                PartType::Binary(_) => "Binary".into(),
                PartType::InlineBinary(_) => "InlineBinary".into(),
                PartType::Message(_) => "Message".into(),
                PartType::Multipart(ids) => format!("Multipart{:?}", ids),
            };
            let ct = p.headers.iter().find(|h| matches!(h.name, mail_parser::HeaderName::ContentType))
                .and_then(|h| h.value.as_content_type())
                .map(|c| format!("{}/{}", c.ctype(), c.subtype().unwrap_or("")));
            println!("  [{}] ct={:?} -> {}", i, ct, kind);
        }
    }
}
