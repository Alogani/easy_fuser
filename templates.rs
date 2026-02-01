use askama::Template;

#[derive(Template)]
#[template(path = "fuse_driver.jinja")]
pub struct FuseDriverTemplate<'a> {
    pub mode: &'a str,
    pub send_sync: bool,
}