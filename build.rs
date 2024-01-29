fn main() {
    minijinja_embed::embed_templates!("src/templates",  &[".html", ".css"]);
}