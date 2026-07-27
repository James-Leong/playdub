use getrandom::fill;
use std::{
    collections::HashMap,
    fmt::Write as _,
    fs::File,
    io::{self, BufWriter, Read, Seek, SeekFrom, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

const MAX_REQUEST_HEADER_BYTES: usize = 16 * 1024;
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(15);

pub(crate) struct MediaServer {
    address: SocketAddr,
    resources: Arc<RwLock<HashMap<String, MediaResource>>>,
    stopping: Arc<AtomicBool>,
}

#[derive(Clone)]
enum MediaResource {
    File(PathBuf),
    Directory(PathBuf),
}

impl MediaServer {
    pub(crate) fn start() -> io::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        let address = listener.local_addr()?;
        let resources = Arc::new(RwLock::new(HashMap::new()));
        let stopping = Arc::new(AtomicBool::new(false));
        let thread_resources = Arc::clone(&resources);
        let thread_stopping = Arc::clone(&stopping);
        thread::Builder::new()
            .name("playdub-media-server".to_owned())
            .spawn(move || serve(&listener, &thread_resources, &thread_stopping))?;
        Ok(Self {
            address,
            resources,
            stopping,
        })
    }

    pub(crate) fn authorize(&self, path: &Path) -> io::Result<String> {
        self.authorize_resource(MediaResource::File(path.to_owned()), "")
    }

    pub(crate) fn authorize_directory(&self, path: &Path, entry: &str) -> io::Result<String> {
        if !valid_child_name(entry) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid media directory entry",
            ));
        }
        self.authorize_resource(MediaResource::Directory(path.to_owned()), entry)
    }

    fn authorize_resource(&self, resource: MediaResource, entry: &str) -> io::Result<String> {
        let mut random = [0_u8; 32];
        fill(&mut random)
            .map_err(|error| io::Error::other(format!("OS randomness failed: {error}")))?;
        let token = random
            .iter()
            .fold(String::with_capacity(64), |mut token, byte| {
                write!(token, "{byte:02x}").expect("writing to a String cannot fail");
                token
            });
        self.resources
            .write()
            .map_err(|_| io::Error::other("media authorization lock was poisoned"))?
            .insert(token.clone(), resource);
        let suffix = if entry.is_empty() {
            String::new()
        } else {
            format!("/{entry}")
        };
        Ok(format!("http://{}/media/{token}{suffix}", self.address))
    }
}

impl Drop for MediaServer {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::Release);
        let _ = TcpStream::connect_timeout(&self.address, Duration::from_millis(100));
    }
}

fn serve(
    listener: &TcpListener,
    resources: &Arc<RwLock<HashMap<String, MediaResource>>>,
    stopping: &AtomicBool,
) {
    for connection in listener.incoming() {
        if stopping.load(Ordering::Acquire) {
            break;
        }
        let Ok(stream) = connection else {
            continue;
        };
        let connection_resources = Arc::clone(resources);
        let _ = thread::Builder::new()
            .name("playdub-media-request".to_owned())
            .spawn(move || {
                let _ = handle_connection(stream, &connection_resources);
            });
    }
}

fn handle_connection(
    mut stream: TcpStream,
    resources: &RwLock<HashMap<String, MediaResource>>,
) -> io::Result<()> {
    stream.set_read_timeout(Some(CONNECTION_TIMEOUT))?;
    stream.set_write_timeout(Some(CONNECTION_TIMEOUT))?;
    let request = read_request(&mut stream)?;
    let Some((method, token, child, requested_range)) = parse_request(&request) else {
        return write_error(&mut stream, 400, "Bad Request");
    };
    if method != "GET" && method != "HEAD" {
        return write_error(&mut stream, 405, "Method Not Allowed");
    }

    let resource = resources
        .read()
        .map_err(|_| io::Error::other("media authorization lock was poisoned"))?
        .get(token)
        .cloned();
    let Some(resource) = resource else {
        return write_error(&mut stream, 404, "Not Found");
    };
    let path = match (resource, child) {
        (MediaResource::File(path), None) => path,
        (MediaResource::Directory(directory), Some(child)) if valid_child_name(child) => {
            let path = directory.join(child);
            let metadata = path.symlink_metadata()?;
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return write_error(&mut stream, 404, "Not Found");
            }
            path
        }
        _ => return write_error(&mut stream, 404, "Not Found"),
    };
    let mut file = File::open(&path)?;
    let length = file.metadata()?.len();
    if length == 0 {
        return write_error(&mut stream, 404, "Not Found");
    }

    let range = match requested_range {
        Some(value) => match parse_range(value, length) {
            Some(range) => Some(range),
            None => return write_range_error(&mut stream, length),
        },
        None => None,
    };
    let (start, end, status) = range.map_or((0, length - 1, "200 OK"), |(start, end)| {
        (start, end, "206 Partial Content")
    });
    let response_length = end + 1 - start;
    let mut writer = BufWriter::new(&mut stream);
    write!(
        writer,
        "HTTP/1.1 {status}\r\nContent-Type: {}\r\nContent-Length: {response_length}\r\nAccept-Ranges: bytes\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: Range\r\nAccess-Control-Expose-Headers: Accept-Ranges, Content-Length, Content-Range\r\nCache-Control: no-store\r\nConnection: close\r\n",
        media_mime_type(&path)
    )?;
    if range.is_some() {
        write!(writer, "Content-Range: bytes {start}-{end}/{length}\r\n")?;
    }
    writer.write_all(b"\r\n")?;
    if method == "GET" {
        file.seek(SeekFrom::Start(start))?;
        io::copy(&mut file.take(response_length), &mut writer)?;
    }
    writer.flush()
}

fn read_request(stream: &mut TcpStream) -> io::Result<Vec<u8>> {
    let mut request = Vec::with_capacity(1024);
    let mut buffer = [0_u8; 1024];
    while request.len() < MAX_REQUEST_HEADER_BYTES {
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        request.extend_from_slice(&buffer[..read]);
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            return Ok(request);
        }
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "invalid or oversized HTTP request",
    ))
}

fn parse_request(request: &[u8]) -> Option<(&str, &str, Option<&str>, Option<&str>)> {
    let request = std::str::from_utf8(request).ok()?;
    let mut lines = request.split("\r\n");
    let mut request_line = lines.next()?.split_ascii_whitespace();
    let method = request_line.next()?;
    let target = request_line.next()?;
    if request_line.next()? != "HTTP/1.1" || request_line.next().is_some() {
        return None;
    }
    let resource = target.strip_prefix("/media/")?;
    let (token, child) = resource
        .split_once('/')
        .map_or((resource, None), |(token, child)| (token, Some(child)));
    if token.len() != 64 || !token.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    if child.is_some_and(|child| !valid_child_name(child)) {
        return None;
    }
    let range = lines.find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case("range").then_some(value.trim())
    });
    Some((method, token, child, range))
}

fn valid_child_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        && value != "."
        && value != ".."
}

fn parse_range(value: &str, length: u64) -> Option<(u64, u64)> {
    let value = value.strip_prefix("bytes=")?;
    if value.contains(',') {
        return None;
    }
    let (start, end) = value.split_once('-')?;
    if start.is_empty() {
        let suffix = end.parse::<u64>().ok()?.min(length);
        return (suffix > 0).then_some((length - suffix, length - 1));
    }
    let start = start.parse::<u64>().ok()?;
    if start >= length {
        return None;
    }
    let end = if end.is_empty() {
        length - 1
    } else {
        end.parse::<u64>().ok()?.min(length - 1)
    };
    (end >= start).then_some((start, end))
}

fn media_mime_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "mp4" | "m4v" => "video/mp4",
        "mov" => "video/quicktime",
        "webm" => "video/webm",
        "mkv" => "video/x-matroska",
        "avi" => "video/x-msvideo",
        "m3u8" => "application/vnd.apple.mpegurl",
        "ts" => "video/mp2t",
        _ => "application/octet-stream",
    }
}

fn write_error(stream: &mut TcpStream, code: u16, reason: &str) -> io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {code} {reason}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    )
}

fn write_range_error(stream: &mut TcpStream, length: u64) -> io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{length}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_bounded_open_and_suffix_ranges() {
        assert_eq!(parse_range("bytes=10-19", 100), Some((10, 19)));
        assert_eq!(parse_range("bytes=90-", 100), Some((90, 99)));
        assert_eq!(parse_range("bytes=-10", 100), Some((90, 99)));
        assert_eq!(parse_range("bytes=90-200", 100), Some((90, 99)));
    }

    #[test]
    fn rejects_multiple_reversed_and_out_of_bounds_ranges() {
        assert_eq!(parse_range("bytes=0-1,5-6", 100), None);
        assert_eq!(parse_range("bytes=20-10", 100), None);
        assert_eq!(parse_range("bytes=100-", 100), None);
        assert_eq!(parse_range("items=0-1", 100), None);
    }

    #[test]
    fn accepts_only_opaque_media_urls_and_single_ranges() {
        let token = "ab".repeat(32);
        let request =
            format!("GET /media/{token} HTTP/1.1\r\nRange: bytes=0-99\r\nHost: local\r\n\r\n");
        assert_eq!(
            parse_request(request.as_bytes()),
            Some(("GET", token.as_str(), None, Some("bytes=0-99")))
        );
        assert_eq!(
            parse_request(b"GET /media/../../secret HTTP/1.1\r\n\r\n"),
            None
        );
    }

    #[test]
    fn serves_only_the_requested_file_range() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "playdub-media-server-{}-{nonce}.mp4",
            std::process::id()
        ));
        std::fs::write(&path, b"0123456789").unwrap();
        let server = MediaServer::start().unwrap();
        let url = server.authorize(&path).unwrap();
        let authority_and_path = url.strip_prefix("http://").unwrap();
        let (authority, target) = authority_and_path.split_once('/').unwrap();
        let mut stream = TcpStream::connect(authority).unwrap();
        write!(
            stream,
            "GET /{target} HTTP/1.1\r\nHost: {authority}\r\nRange: bytes=3-6\r\n\r\n"
        )
        .unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).unwrap();
        assert!(response.starts_with(b"HTTP/1.1 206 Partial Content\r\n"));
        assert!(response.ends_with(b"\r\n\r\n3456"));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn serves_authorized_hls_entries_without_allowing_traversal() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "playdub-media-directory-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&directory).unwrap();
        std::fs::write(directory.join("stream.m3u8"), b"#EXTM3U\n").unwrap();
        let server = MediaServer::start().unwrap();
        let url = server
            .authorize_directory(&directory, "stream.m3u8")
            .unwrap();
        assert!(url.ends_with("/stream.m3u8"));
        assert!(server.authorize_directory(&directory, "../secret").is_err());
        std::fs::remove_dir_all(directory).unwrap();
    }
}
