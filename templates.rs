use askama::Template;

#[derive(Template)]
#[template(path = "fuse_driver.jinja")]
pub struct FuseDriverTemplate<'a> {
    pub mode: &'a str,
}

#[derive(Template)]
#[template(path = "fuse_handler.jinja")]
pub struct FuseHandlerTemplate<'a> {
    pub mode: &'a str,
}