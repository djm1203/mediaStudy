/// Render markdown text to the terminal using termimad
pub fn render_markdown(text: &str) {
    let skin = termimad::MadSkin::default();
    skin.print_text(text);
}
