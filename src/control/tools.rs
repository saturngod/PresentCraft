//! The tools an MCP client can call, with their JSON schemas.
//!
//! Slide numbers are 1-based, as the app shows them. Colours are hex strings
//! such as "#1C1C1E". Sizes and paths are strings.

use super::wire::{obj, text};
use crate::json::Json;

/// (name, description, properties, required). A property is
/// (name, type, description); a type is `string`, `integer`, `number`,
/// `boolean`, `string[]`, `string[][]` or `enum:a|b|c`.
type Tool = (
    &'static str,
    &'static str,
    &'static [(&'static str, &'static str, &'static str)],
    &'static [&'static str],
);

const SLIDE: (&str, &str, &str) = ("slide", "integer", "Slide number, starting at 1.");
const CONFIRM: (&str, &str, &str) = (
    "confirm",
    "boolean",
    "Must be true. Destructive changes are refused without it.",
);
const MARKDOWN: (&str, &str, &str) = (
    "markdown",
    "string",
    "Slide text in PresentCraft's Markdown. Lines starting with a tab show on the slide; other lines are speaker notes. A line of three or more dashes starts a new slide.",
);
const LOOK: (&str, &str, &str) = ("look", "enum:light|dark", "Which appearance to change.");

pub const TOOLS: &[Tool] = &[
    ("deck_info", "Summary of the open presentation: name, path, unsaved changes, slide count, theme, appearance, fonts, size, line height, author, audience, aspect, logo, and whether it is presenting.", &[], &[]),
    ("deck_read_markdown", "The whole Markdown script.", &[], &[]),
    ("deck_write_markdown", "Replace the whole Markdown script. Pass confirm: true.", &[MARKDOWN, CONFIRM], &["markdown", "confirm"]),
    ("deck_check", "Check the deck: empty slides, missing pictures, a missing logo, and text contrast below 4.5:1 for each appearance.", &[], &[]),
    ("slide_list", "Every slide with its number, title, and whether it has a picture.", &[], &[]),
    ("slide_get", "The Markdown of one slide.", &[SLIDE], &["slide"]),
    ("slide_add", "Add a slide. Without position it goes at the end.", &[("position", "integer", "Slide number the new slide will have."), MARKDOWN], &["markdown"]),
    ("slide_update", "Replace the Markdown of one slide (text, notes, pictures, tables).", &[SLIDE, MARKDOWN], &["slide", "markdown"]),
    ("slide_delete", "Delete a slide. Pass confirm: true.", &[SLIDE, CONFIRM], &["slide", "confirm"]),
    ("slide_move", "Move a slide to another position.", &[SLIDE, ("to", "integer", "Slide number to move it to.")], &["slide", "to"]),
    ("slide_duplicate", "Copy a slide to the position after it.", &[SLIDE], &["slide"]),
    ("slide_select", "Open a slide in the editor and show it.", &[SLIDE], &["slide"]),
    ("slide_screenshot", "A PNG of the window with this slide shown. Use it to check layout, contrast and overflow.", &[SLIDE], &["slide"]),
    ("insert_picture", "Add a picture or video to a slide. A local file is copied into the presentation; /assets/… and web addresses are used as they are. Video files (.mp4, .mov, .m4v) play in exports.", &[
        SLIDE,
        ("src", "string", "Local file path, /assets/… path, or https:// address."),
        ("caption", "string", "Caption under the picture."),
        ("size", "enum:fill|contain", "fill crops to the area; contain fits the whole picture."),
        ("x", "enum:left|center|right", "Horizontal position."),
        ("y", "enum:top|center|bottom", "Vertical position."),
        ("background", "boolean", "Put the picture behind the slide text, full bleed."),
        ("opacity", "number", "0 to 1."),
        ("filter", "enum:grayscale|sepia|blur|lighten|darken", "Picture filter."),
    ], &["slide", "src"]),
    ("insert_table", "Add a table to a slide, optionally drawn as a chart. The first column holds the labels and the other columns are series; pie, donut and gauge charts use the first number column. The chart options need chart.", &[
        SLIDE,
        ("headers", "string[]", "Column headings."),
        ("rows", "string[][]", "Rows of cell text."),
        ("caption", "string", "Caption under the table or chart."),
        ("chart", "enum:bar|line|spline|area|step|pie|donut|radar|gauge|bubble", "Draw the table as a chart of this type."),
        ("horizontal", "boolean", "Bars run sideways (bar charts)."),
        ("stacked", "boolean", "Stack the series instead of placing them side by side."),
        ("title", "string", "Chart title."),
        ("xlabel", "string", "Label for the horizontal axis."),
        ("ylabel", "string", "Label for the vertical axis."),
        ("colors", "string[]", "Series colours in order: hex such as #E63946, or theme accents such as accent 1 (accent 1 to accent 6). Without it, the theme's chart colours are used."),
        ("color_type", "enum:categorical|sequential|diverging", "categorical gives each series its own colour (default); sequential uses shades of the first colour; diverging runs from the first colour through a neutral middle to the second."),
        ("legend", "boolean", "false hides the legend. It shows by default."),
    ], &["slide", "headers"]),
    ("title_align", "Place the title of a cover slide (one that starts with a # heading) and its subtitle. Writes the x: and y: lines under the heading; the defaults are left and middle. A ## title alone is always centred.", &[
        SLIDE,
        ("x", "enum:left|center|right", "Horizontal position."),
        ("y", "enum:top|middle|bottom", "Vertical position."),
    ], &["slide"]),
    ("undo_ai_change", "Undo the last change made through this server. Changes made by hand in the app are not included.", &[], &[]),
    ("theme_list", "The built-in and saved themes, with their numbers.", &[], &[]),
    ("theme_apply", "Apply a theme by name. This replaces design overrides, as choosing a theme in the app does.", &[("name", "string", "Theme name, as theme_list shows it.")], &["name"]),
    ("theme_import", "Import a theme folder into the saved themes and apply it.", &[("path", "string", "Absolute path to the theme folder.")], &["path"]),
    ("theme_export", "Save the current design as a theme folder.", &[("path", "string", "Absolute path of the new folder inside your home folder.")], &["path"]),
    ("appearance_set", "Switch the appearance.", &[("mode", "enum:light|dark|system", "light, dark, or follow the system.")], &["mode"]),
    ("palette_get", "The background, title and body colours for one appearance, and the six accent colours.", &[LOOK], &["look"]),
    ("palette_set", "Change the background, title or body colour for one appearance.", &[
        LOOK,
        ("background", "string", "Background colour, hex."),
        ("title", "string", "Title colour, hex."),
        ("body", "string", "Body text colour, hex."),
    ], &["look"]),
    ("background_set", "Set the slide background to a solid colour or a gradient for one appearance.", &[
        LOOK,
        ("mode", "enum:solid|gradient", "solid uses the theme's colour; gradient uses the colours below."),
        ("colors", "string[]", "Gradient colours, hex, first slide to last. At least two."),
    ], &["look", "mode"]),
    ("font_set", "Set the title or body font.", &[
        ("role", "enum:title|body", "Which text."),
        ("face", "string", "theme, sans, serif, or the name of an installed font."),
    ], &["role", "face"]),
    ("size_set", "Set the slide text size.", &[("size", "enum:S|M|L|XL", "Text size.")], &["size"]),
    ("line_height_set", "Set the line spacing as a percentage (80 to 200).", &[("percent", "number", "Line spacing, e.g. 115.")], &["percent"]),
    ("document_info_set", "Set the author, audience and slide shape.", &[
        ("author", "string", "Author name."),
        ("audience", "string", "Audience."),
        ("aspect", "enum:16:9|16:10|4:3", "Slide shape."),
    ], &[]),
    ("header_footer_get", "The six header and footer places: leading, middle and trailing, in the header and in the footer.", &[], &[]),
    ("header_footer_set", "Set what one header or footer place shows.", &[
        ("place", "integer", "1 to 6: 1-3 are the header (leading, middle, trailing), 4-6 the footer."),
        ("kind", "enum:empty|number|date|logo|custom", "What to show. number is the slide number; logo shows the deck logo; custom shows text."),
        ("text", "string", "Text for custom."),
    ], &["place", "kind"]),
    ("logo_set", "Set the deck logo from a picture file or a web address. Show it in a header or footer place with header_footer_set kind logo.", &[("src", "string", "Local picture file, /assets/… path, or https:// address.")], &["src"]),
    ("logo_size", "Set the logo height as a fraction of the slide (0.02 to 0.08).", &[("size", "number", "0.02 to 0.08.")], &["size"]),
    ("logo_remove", "Remove the deck logo.", &[], &[]),
    ("media_list", "The pictures and videos in the presentation, with their /assets/ paths.", &[], &[]),
    ("media_add", "Copy a picture or video into the presentation and return its /assets/ path.", &[("path", "string", "Absolute path to the file.")], &["path"]),
    ("media_remove", "Remove an unused picture or video from the presentation. Pass confirm: true.", &[("ref", "string", "/assets/… path."), CONFIRM], &["ref", "confirm"]),
    ("settings_get", "Editor and presentation preferences that the tools may change.", &[], &[]),
    ("settings_set", "Change one preference from settings_get.", &[
        ("key", "enum:editor_font|smart_quotes|smart_dashes|single_return|fit_text|wpm|show_speaking_time|warn_empty", "Preference name."),
        ("value", "string", "true, false, or a number."),
    ], &["key", "value"]),
    ("present_start", "Start the full-screen presentation, optionally from a slide.", &[("slide", "integer", "Slide number to start from.")], &[]),
    ("present_stop", "Stop the presentation.", &[], &[]),
    ("export", "Export the presentation. Pictures and HTML folders are written to the path given.", &[
        ("format", "enum:pdf|pptx|images|html", "File type."),
        ("path", "string", "Absolute path inside your home folder. For images, a folder."),
        ("overwrite", "boolean", "Replace the file if it exists."),
    ], &["format", "path"]),
    ("package_save", "Save the presentation to a .prcraft path.", &[("path", "string", "Absolute path ending in .prcraft, inside your home folder.")], &["path"]),
    ("package_open", "Open a presentation. Unsaved changes are kept unless discard is true.", &[
        ("path", "string", "Absolute path to a .prcraft folder, zip or Markdown file."),
        ("discard", "boolean", "Open even if there are unsaved changes."),
    ], &["path"]),
];

/// Tools that also work when the app is closed. They take a `package` path
/// to a folder-form .prcraft presentation.
pub const HEADLESS: &[&str] = &[
    "deck_read_markdown",
    "deck_write_markdown",
    "slide_list",
    "slide_get",
    "slide_add",
    "slide_update",
    "slide_delete",
    "slide_move",
    "slide_duplicate",
    "insert_picture",
    "insert_table",
    "title_align",
];

fn schema_for(ty: &str) -> Json {
    if let Some(choices) = ty.strip_prefix("enum:") {
        let values = choices.split('|').map(text).collect();
        return obj(vec![("type", text("string")), ("enum", Json::Arr(values))]);
    }
    match ty {
        "string[]" => obj(vec![
            ("type", text("array")),
            ("items", obj(vec![("type", text("string"))])),
        ]),
        "string[][]" => obj(vec![
            ("type", text("array")),
            (
                "items",
                obj(vec![
                    ("type", text("array")),
                    ("items", obj(vec![("type", text("string"))])),
                ]),
            ),
        ]),
        other => obj(vec![("type", text(other))]),
    }
}

fn input_schema(props: &[(&str, &str, &str)], required: &[&str], headless: bool) -> Json {
    let mut fields: Vec<(String, Json)> = props
        .iter()
        .map(|(name, ty, desc)| {
            let mut s = schema_for(ty);
            if let Json::Obj(items) = &mut s {
                items.push(("description".to_string(), text(desc)));
            }
            (name.to_string(), s)
        })
        .collect();
    if headless {
        fields.push((
            "package".to_string(),
            obj(vec![
                ("type", text("string")),
                (
                    "description",
                    text("Only when PresentCraft is closed: absolute path to a .prcraft folder to edit on disk."),
                ),
            ]),
        ));
    }
    obj(vec![
        ("type", text("object")),
        ("properties", Json::Obj(fields)),
        (
            "required",
            Json::Arr(required.iter().map(|r| text(r)).collect()),
        ),
    ])
}

/// The tool list for `tools/list`.
pub fn list() -> Json {
    let mut out = Vec::new();
    for (name, desc, props, required) in TOOLS {
        out.push(obj(vec![
            ("name", text(name)),
            ("description", text(desc)),
            (
                "inputSchema",
                input_schema(props, required, HEADLESS.contains(name)),
            ),
        ]));
    }
    Json::Arr(out)
}

pub fn exists(name: &str) -> bool {
    TOOLS.iter().any(|(n, ..)| *n == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tool_has_a_schema_and_unique_name() {
        let mut names: Vec<&str> = TOOLS.iter().map(|t| t.0).collect();
        let total = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), total);
        assert!(exists("slide_get"));
        let Json::Arr(items) = list() else { panic!() };
        assert_eq!(items.len(), total);
    }
}
