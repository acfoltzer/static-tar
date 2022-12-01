use std::path::Path;

use fastly::http::header::{CONTENT_LENGTH, LOCATION};
use fastly::http::{HeaderValue, Method, StatusCode};
use fastly::{Error, Request, Response};
use flate2::bufread::GzDecoder;
use tar::Archive;

const SITE_TAR_GZ: &'static [u8] = include_bytes!("site.tar.gz");
const SITE_ROOT: HeaderValue = HeaderValue::from_static("/doc/fastly/index.html");

fn main() -> Result<(), Error> {
    let req = Request::from_client();
    if req.get_method() != Method::GET {
        Response::from_status(StatusCode::METHOD_NOT_ALLOWED).send_to_client();
        return Ok(());
    }
    match req.get_path() {
        "/" => {
            Response::from_status(StatusCode::PERMANENT_REDIRECT)
                .with_header(LOCATION, SITE_ROOT)
                .send_to_client();
            Ok(())
        }
        other if other.starts_with('/') => try_from_site_tar(other.split_at(1).1),
        huh => panic!("unexpected path format: {huh}"),
    }
}

fn try_from_site_tar(path: &str) -> Result<(), Error> {
    let path = Path::new(path);
    for entry in Archive::new(GzDecoder::new(SITE_TAR_GZ)).entries()? {
        let mut entry = entry?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        if entry.path()? == path {
            let mime_type = mime_guess::from_path(path).first_or_octet_stream();
            let mut resp_body = Response::from_status(200)
                .with_content_type(mime_type)
                .with_header(CONTENT_LENGTH, entry.header().size()?.to_string())
                .with_framing_headers_mode(fastly::http::FramingHeadersMode::ManuallyFromHeaders)
                .stream_to_client();
            std::io::copy(&mut entry, &mut resp_body)?;
            return Ok(());
        }
    }
    Response::from_status(404).send_to_client();
    Ok(())
}
