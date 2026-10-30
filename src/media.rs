use include_dir::{include_dir, Dir};

static MEDIA: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/assets/media");

pub fn file(name: &str) -> Option<&'static [u8]> {
    MEDIA.get_file(name).map(include_dir::File::contents)
}

pub fn animation(id: i64) -> Option<&'static [u8]> {
    file(&format!("gifs/{id}.webp"))
}
