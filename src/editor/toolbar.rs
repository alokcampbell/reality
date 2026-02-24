use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct ToolbarProps {
    pub on_action: EventHandler<ToolbarAction>,
}

#[derive(Clone, PartialEq)]
pub enum ToolbarAction {
    Bold,
    Italic,
    Code,
    CodeBlock,
    Heading(u8),
    Link,
    BulletList,
    NumberedList,
    Quote,
    HRule,
}

#[component]
pub fn Toolbar(props: ToolbarProps) -> Element {
    rsx! {
        div {
            style: "display:flex;gap:0.25rem;padding:0.4rem 1rem;background:#2a2a3e;border-bottom:1px solid #3a3a5e;flex-wrap:wrap;",

            ToolbarBtn { label: "B", title: "Bold (ctrl+b)", action: ToolbarAction::Bold, on_action: props.on_action.clone(), bold: true }
            ToolbarBtn { label: "I", title: "Italic (ctrl+i)", action: ToolbarAction::Italic, on_action: props.on_action.clone(), italic: true }
            ToolbarBtn { label: "`", title: "Inline code", action: ToolbarAction::Code, on_action: props.on_action.clone(), bold: false, italic: false }
            ToolbarBtn { label: "```", title: "Code block", action: ToolbarAction::CodeBlock, on_action: props.on_action.clone(), bold: false, italic: false }

            div { style: "width:1px;background:#3a3a5e;margin:0 0.25rem;" }

            ToolbarBtn { label: "H1", title: "Heading 1", action: ToolbarAction::Heading(1), on_action: props.on_action.clone(), bold: false, italic: false }
            ToolbarBtn { label: "H2", title: "Heading 2", action: ToolbarAction::Heading(2), on_action: props.on_action.clone(), bold: false, italic: false }
            ToolbarBtn { label: "H3", title: "Heading 3", action: ToolbarAction::Heading(3), on_action: props.on_action.clone(), bold: false, italic: false }

            div { style: "width:1px;background:#3a3a5e;margin:0 0.25rem;" }

            ToolbarBtn { label: "•", title: "Bullet list", action: ToolbarAction::BulletList, on_action: props.on_action.clone(), bold: false, italic: false }
            ToolbarBtn { label: "1.", title: "Numbered list", action: ToolbarAction::NumberedList, on_action: props.on_action.clone(), bold: false, italic: false }
            ToolbarBtn { label: "❝", title: "Blockquote", action: ToolbarAction::Quote, on_action: props.on_action.clone(), bold: false, italic: false }
            ToolbarBtn { label: "—", title: "Horizontal rule", action: ToolbarAction::HRule, on_action: props.on_action.clone(), bold: false, italic: false }

            div { style: "width:1px;background:#3a3a5e;margin:0 0.25rem;" }

            ToolbarBtn { label: "🔗", title: "Link", action: ToolbarAction::Link, on_action: props.on_action.clone(), bold: false, italic: false }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct ToolbarBtnProps {
    label: &'static str,
    title: &'static str,
    action: ToolbarAction,
    on_action: EventHandler<ToolbarAction>,
    #[props(default = false)]
    bold: bool,
    #[props(default = false)]
    italic: bool,
}

#[component]
fn ToolbarBtn(props: ToolbarBtnProps) -> Element {
    let style = format!(
        "padding:0.2rem 0.5rem;background:#3a3a5e;color:white;border:none;border-radius:3px;cursor:pointer;font-size:0.85rem;{}{}",
        if props.bold { "font-weight:bold;" } else { "" },
        if props.italic { "font-style:italic;" } else { "" },
    );
    rsx! {
        button {
            style: "{style}",
            title: "{props.title}",
            onclick: move |_| props.on_action.call(props.action.clone()),
            "{props.label}"
        }
    }
}