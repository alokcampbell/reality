#![allow(non_snake_case)]
pub mod crdt;
pub mod toolbar;

use dioxus::prelude::*;
use gloo_net::websocket::{futures::WebSocket, Message};
use futures_util::{SinkExt, StreamExt};
use toolbar::{Toolbar, ToolbarAction};

fn get_ws_url(id: &str) -> String {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let host = location.host().unwrap();
    let protocol = location.protocol().unwrap();
    let ws_protocol = if protocol == "https:" { "wss:" } else { "ws:" };
    format!("{}//{}/ws/{}", ws_protocol, host, id)
}

fn get_textarea() -> Option<web_sys::HtmlTextAreaElement> {
    use wasm_bindgen::JsCast;
    web_sys::window()?
        .document()?
        .get_element_by_id("editor-textarea")?
        .dyn_into::<web_sys::HtmlTextAreaElement>()
        .ok()
}

// keeping the cursor in the same place so it doesnt teleport every new line

fn get_cursor() -> (u32, u32) {
    get_textarea()
        .map(|ta| {
            let s = ta.selection_start().unwrap_or(None).unwrap_or(0);
            let e = ta.selection_end().unwrap_or(None).unwrap_or(0);
            (s, e)
        })
        .unwrap_or((0, 0))
}

fn set_cursor(start: u32, end: u32) {
    if let Some(ta) = get_textarea() {
        let _ = ta.set_selection_start(Some(start));
        let _ = ta.set_selection_end(Some(end));
    }
}

fn apply_remote_patch(old_text: &str, new_text: &str) {
    let ta = match get_textarea() {
        Some(t) => t,
        None    => return,
    };

    let (splice_at, del, ins) = crdt::diff(old_text, new_text);
    let ins_len = ins.chars().count();
    let (cur_start, cur_end) = get_cursor();
    ta.set_value(new_text);
    let new_start = adjust_cursor(cur_start as usize, splice_at, del, ins_len) as u32;
    let new_end   = adjust_cursor(cur_end   as usize, splice_at, del, ins_len) as u32;
    set_cursor(new_start, new_end);
}

fn generate_client_id() -> String {
    let a = (js_sys::Math::random() * 0xffffffff_u32 as f64) as u64;
    let b = (js_sys::Math::random() * 0xffffffff_u32 as f64) as u64;
    format!("{:x}{:x}", a, b)
}

#[component]
pub fn Editor(id: String) -> Element {
    let mut content   = use_signal(|| String::new());
    let mut preview   = use_signal(|| false);
    let mut last_text = use_signal(|| String::new());
    let client_id     = use_signal(|| generate_client_id());
    let mut doc = use_signal(|| crdt::Doc::new());

    let ws_tx: Signal<Option<futures_channel::mpsc::UnboundedSender<String>>> =
        use_signal(|| None);

    use_effect({
        let id        = id.clone();
        let mut ws_tx = ws_tx.clone();

        move || {
            let id        = id.clone();
            let client_id = client_id.read().clone();

            wasm_bindgen_futures::spawn_local(async move {
                let ws = match WebSocket::open(&get_ws_url(&id)) {
                    Ok(ws) => ws,
                    Err(e) => { eprintln!("WS error: {:?}", e); return; }
                };

                let (mut write, mut read) = ws.split();
                let (tx, mut rx) = futures_channel::mpsc::unbounded::<String>();
                ws_tx.set(Some(tx));

                wasm_bindgen_futures::spawn_local(async move {
                    while let Some(msg) = rx.next().await {
                        let _ = write.send(Message::Text(msg)).await;
                    }
                });

                wasm_bindgen_futures::spawn_local(async move {
                    while let Some(Ok(msg)) = read.next().await {
                        let json = match msg {
                            Message::Text(t)  => t,
                            Message::Bytes(b) => match String::from_utf8(b) {
                                Ok(s)  => s,
                                Err(_) => continue,
                            },
                        };
                    
                        let (text, sender_id, full_doc_bytes) = match decode_payload(json.as_bytes()) {
                            Some(v) => v,
                            None    => continue,
                        };
                        
                        if sender_id == "server" {
                            if let Some(loaded) = crdt::Doc::load_from_bytes(&full_doc_bytes) {
                                *doc.write() = loaded;
                            }
                            let current_text = doc.read().get_text();
                            if let Some(ta) = get_textarea() {
                                ta.set_value(&current_text);
                            }
                            last_text.set(current_text.clone());
                            content.set(current_text);
                            continue;
                        }
                        if sender_id != client_id {
                            let old_text = last_text.read().clone();
                            let merged_text = doc.write().merge_from_bytes(&full_doc_bytes);
                            let new_text = merged_text.unwrap_or(text);
                            apply_remote_patch(&old_text, &new_text);
                            last_text.set(new_text.clone());
                            content.set(new_text);
                        }
                    }
                });
            });
        }
    });

    let mut send_patch = move |old: &str, new: &str| {
        let (insert_at, delete_count, inserted_text) = crdt::diff(old, new);
        if delete_count == 0 && inserted_text.is_empty() { return; }
    
        let change_bytes = {
            let mut d = doc.write();
            d.splice_text(insert_at, delete_count, &inserted_text);
            d.save_changes()
        };
    
        let msg = serde_json::json!({
            "client_id": &*client_id.read(),
            "changes": change_bytes,
        });
    
        if let Some(tx) = ws_tx.read().as_ref() {
            let _ = tx.unbounded_send(msg.to_string());
        }
    };

    let handle_input = move |e: Event<FormData>| {
        let new_text = e.value();
        let old_text = last_text.read().clone();
        send_patch(&old_text, &new_text);
        last_text.set(new_text.clone());
        content.set(new_text);
    };

    let handle_toolbar = move |action: ToolbarAction| {
        let old_text = content.read().clone();
        let (sel_start, sel_end) = get_cursor();
        let (new_text, cursor_after) = apply_toolbar_action_at_cursor(
            &old_text, action, sel_start as usize, sel_end as usize,
        );

        if let Some(ta) = get_textarea() {
            ta.set_value(&new_text);
        }
        set_cursor(cursor_after as u32, cursor_after as u32);
        send_patch(&old_text, &new_text);
        last_text.set(new_text.clone());
        content.set(new_text);
    };

    let id_display = id.clone();

    rsx! {
        // this is the site / code for the actual note taking app, not the landing page like landing.rs
        div { style: "display:flex;flex-direction:column;height:100vh;font-family:monospace;",

            div { style: "display:flex;align-items:center;padding:0.5rem 1rem;background:#1a1a2e;color:white;gap:1rem;flex-shrink:0;",
                span { style: "font-weight:bold;font-family:sans-serif;font-size:1.1rem;", "Reality" }
                span { style: "font-size:0.75rem;opacity:0.6;flex:1;font-family:sans-serif;", "/{id_display}" }
                button {
                    style: "padding:0.3rem 0.7rem;background:#3a3a5e;color:white;border:none;border-radius:4px;cursor:pointer;",
                    onclick: move |_| {
                        let id = id_display.clone();
                        wasm_bindgen_futures::spawn_local(async move {
                            let window = web_sys::window().unwrap();
                            let origin = window.location().origin().unwrap();
                            let url = format!("{}/doc/{}", origin, id);
                            let cb = window.navigator().clipboard();
                            let _ = wasm_bindgen_futures::JsFuture::from(cb.write_text(&url)).await;
                        });
                    },
                    "Copy Link"
                }
                button {
                    style: "padding:0.3rem 0.7rem;background:#3a3a5e;color:white;border:none;border-radius:4px;cursor:pointer;",
                    onclick: move |_| preview.set(!preview()),
                    if preview() { "✏ Edit" } else { "👁 Preview" }
                }
                button {
                    style: "padding:0.3rem 0.7rem;background:#3a3a5e;color:white;border:none;border-radius:4px;cursor:pointer;",
                    onclick: move |_| download_md(&content.read()),
                    "⬇ Download"
                }
            }

            if !preview() {
                Toolbar { on_action: handle_toolbar }
            }

            div { style: "flex:1;overflow:hidden;display:flex;",
                if preview() {
                    div {
                        style: "flex:1;padding:2rem;overflow:auto;max-width:800px;margin:0 auto;font-family:sans-serif;line-height:1.6;",
                        dangerous_inner_html: render_markdown(&content.read())
                    }
                } else {
                    // holding already typed data for the preview
                    textarea {
                        id: "editor-textarea",
                        style: "flex:1;padding:1rem;font-family:'Fira Code',monospace;font-size:14px;line-height:1.6;border:none;resize:none;outline:none;background:#fafafa;width:100%;box-sizing:border-box;",
                        value: "{content}",
                        oninput: handle_input,
                    }
                }
            }
        }
    }
}

fn adjust_cursor(cursor: usize, splice_at: usize, del: usize, ins_len: usize) -> usize {
    if cursor <= splice_at {
        cursor
    } else if cursor < splice_at + del {
        splice_at
    } else {
        (cursor + ins_len).saturating_sub(del)
    }
}

fn apply_toolbar_action_at_cursor(
    content: &str,
    action: ToolbarAction,
    sel_start: usize,
    sel_end: usize,
) -> (String, usize) {
    let chars: Vec<char> = content.chars().collect();
    let sel_start = sel_start.min(chars.len());
    let sel_end   = sel_end.min(chars.len());
    let selected: String = chars[sel_start..sel_end].iter().collect();
    let before:   String = chars[..sel_start].iter().collect();
    let after:    String = chars[sel_end..].iter().collect();
    match action {
        ToolbarAction::Bold => {
            let inner = if selected.is_empty() { "bold text" } else { &selected };
            let cursor = sel_start + 2 + inner.chars().count() + 2;
            (format!("{}**{}**{}", before, inner, after), cursor)
        }
        ToolbarAction::Italic => {
            let inner = if selected.is_empty() { "italic text" } else { &selected };
            let cursor = sel_start + 1 + inner.chars().count() + 1;
            (format!("{}_{}_{}", before, inner, after), cursor)
        }
        ToolbarAction::Code => {
            let inner = if selected.is_empty() { "code" } else { &selected };
            let cursor = sel_start + 1 + inner.chars().count() + 1;
            (format!("{}`{}`{}", before, inner, after), cursor)
        }
        ToolbarAction::Link => {
            let inner = if selected.is_empty() { "link text" } else { &selected };
            let url = "https://url.com";
            let cursor = sel_start + 1 + inner.chars().count() + 2 + url.chars().count() + 1;
            (format!("{}[{}]({}){}", before, inner, url, after), cursor)
        }
        _ => {
            // toolbar actions lol
            let snippet: &str = match action {
                ToolbarAction::CodeBlock    => "```\ncode here\n```",
                ToolbarAction::Heading(1)   => "# Heading 1",
                ToolbarAction::Heading(2)   => "## Heading 2",
                ToolbarAction::Heading(3)   => "### Heading 3",
                ToolbarAction::BulletList   => "- list item",
                ToolbarAction::NumberedList => "1. list item",
                ToolbarAction::Quote        => "> blockquote",
                ToolbarAction::HRule        => "---",
                _                           => return (content.to_string(), sel_start),
            };
            let leading  = if !before.is_empty() && !before.ends_with('\n') { "\n" } else { "" };
            let trailing = if !after.is_empty()  && !after.starts_with('\n') { "\n" } else { "" };
            let cursor   = sel_start + leading.len() + snippet.chars().count() + trailing.len();
            (format!("{}{}{}{}{}", before, leading, snippet, trailing, after), cursor)
        }
    }
}

fn decode_payload(bytes: &[u8]) -> Option<(String, String, Vec<u8>)> {
    let v: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let text      = v.get("text")?.as_str()?.to_string();
    let sender_id = v.get("sender_id")?.as_str()?.to_string();
    let full_doc  = v.get("full_doc")?
        .as_array()?
        .iter()
        .filter_map(|b| b.as_u64().map(|n| n as u8))
        .collect();
    Some((text, sender_id, full_doc))
}

fn download_md(content: &str) {
    use wasm_bindgen::JsCast;
    let window   = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let arr      = js_sys::Array::new();
    arr.push(&wasm_bindgen::JsValue::from_str(content));
    let blob = web_sys::Blob::new_with_str_sequence(&arr).unwrap();
    let url  = web_sys::Url::create_object_url_with_blob(&blob).unwrap();
    let a    = document.create_element("a").unwrap()
        .dyn_into::<web_sys::HtmlAnchorElement>().unwrap();
    a.set_href(&url);
    a.set_download("document.md");
    a.click();
    web_sys::Url::revoke_object_url(&url).unwrap();
}

// visual effects, maybe one day i can go back and make it one function with a list or something, but did it the hard and long way for now

fn render_markdown(md: &str) -> String {
    let mut output         = String::new();
    let mut in_code_block  = false;
    let mut in_list        = false;
    for line in md.lines() {
        if line.starts_with("```") {
            if in_code_block {
                output.push_str("</code></pre>\n");
                in_code_block = false;
            } else {
                if in_list { output.push_str("</ul>\n"); in_list = false; }
                output.push_str("<pre><code>");
                in_code_block = true;
            }
            continue;
        }
        if in_code_block { output.push_str(&html_escape(line)); output.push('\n'); continue; }
        if in_list && !line.starts_with("- ") { output.push_str("</ul>\n"); in_list = false; }
        let rendered = if let Some(h) = line.strip_prefix("### ") {
            format!("<h3>{}</h3>\n", inline_md(h))
        } else if let Some(h) = line.strip_prefix("## ") {
            format!("<h2>{}</h2>\n", inline_md(h))
        } else if let Some(h) = line.strip_prefix("# ") {
            format!("<h1>{}</h1>\n", inline_md(h))
        } else if let Some(item) = line.strip_prefix("- ") {
            if !in_list { output.push_str("<ul>\n"); in_list = true; }
            format!("<li>{}</li>\n", inline_md(item))
        } else if let Some(item) = line.strip_prefix("> ") {
            format!("<blockquote>{}</blockquote>\n", inline_md(item))
        } else if line == "---" {
            "<hr>\n".to_string()
        } else if line.is_empty() {
            "<br>\n".to_string()
        } else {
            format!("<p>{}</p>\n", inline_md(line))
        };
        output.push_str(&rendered);
    }
    if in_list { output.push_str("</ul>\n"); }
    if in_code_block { output.push_str("</code></pre>\n"); }
    output
}

fn inline_md(s: &str) -> String {
    let s = html_escape(s);
    let s = replace_inline(&s, '`', '`', "<code>", "</code>");
    let s = replace_bold(&s);
    let s = replace_inline(&s, '_', '_', "<em>", "</em>");
    replace_links(&s)
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn replace_bold(s: &str) -> String {
    let mut result = String::new();
    let mut chars  = s.chars().peekable();
    let mut open   = false;
    while let Some(c) = chars.next() {
        if c == '*' && chars.peek() == Some(&'*') {
            chars.next();
            if open { result.push_str("</strong>"); } else { result.push_str("<strong>"); }
            open = !open;
        } else { result.push(c); }
    }
    result
}

fn replace_inline(s: &str, open_char: char, close_char: char, open_tag: &str, close_tag: &str) -> String {
    let mut result = String::new();
    let mut inside = false;
    for c in s.chars() {
        if c == open_char && !inside { result.push_str(open_tag); inside = true; }
        else if c == close_char && inside { result.push_str(close_tag); inside = false; }
        else { result.push(c); }
    }
    result
}

fn replace_links(s: &str) -> String {
    let mut result = s.to_string();
    while let Some(start) = result.find('[') {
        if let Some(mid) = result[start..].find("](") {
            let mid = start + mid;
            if let Some(end) = result[mid..].find(')') {
                let end  = mid + end;
                let text = result[start+1..mid].to_string();
                let url  = result[mid+2..end].to_string();
                let link = format!("<a href=\"{}\" target=\"_blank\">{}</a>", url, text);
                result   = format!("{}{}{}", &result[..start], link, &result[end+1..]);
                continue;
            }
        }
        break;
    }
    result
}