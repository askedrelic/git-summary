fn main() {
    println!("cargo:rerun-if-changed=tailwind.config.js");
    println!("cargo:rerun-if-changed=src/templates/input.css");

    minijinja_embed::embed_templates!("src/templates",  &[".html", ".css"]);
}