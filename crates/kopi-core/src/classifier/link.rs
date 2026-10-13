//! Web addresses.

/// Schemes that make a link. `mailto:`, `tel:` and app schemes stay text.
const SCHEMES: &[&str] = &["http", "https", "ftp", "ftps"];

/// Longer text isn't one address that someone copied.
const MAX_BYTES: usize = 8 * 1024;

/// The host, if the whole of `text` is one web address: a URL with a scheme from [`SCHEMES`]
/// and a host, or a bare `www.` address. Lowercase, without credentials or a port.
pub(super) fn host(text: &str) -> Option<String> {
    if text.is_empty() || text.len() > MAX_BYTES || text.chars().any(char::is_whitespace) {
        return None;
    }
    let (rest, bare) = match text.split_once("://") {
        Some((scheme, rest)) => {
            if !SCHEMES.iter().any(|s| scheme.eq_ignore_ascii_case(s)) {
                return None;
            }
            (rest, false)
        }
        None if text.get(..4)?.eq_ignore_ascii_case("www.") => (text, true),
        None => return None,
    };
    let authority = rest.split(['/', '?', '#']).next()?;
    let host_port = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);

    if let Some(ipv6) = host_port.strip_prefix('[') {
        let (address, after) = ipv6.split_once(']')?;
        let ok = !address.is_empty()
            && address
                .chars()
                .all(|c| c.is_ascii_hexdigit() || c == ':' || c == '.');
        let port_ok = after.is_empty() || after.strip_prefix(':').is_some_and(is_port);
        return (ok && port_ok).then(|| format!("[{}]", address.to_ascii_lowercase()));
    }
    let host = match host_port.rsplit_once(':') {
        Some((host, port)) if is_port(port) => host,
        Some(_) => return None,
        None => host_port,
    };

    let labels: Vec<&str> = host.strip_suffix('.').unwrap_or(host).split('.').collect();
    let valid = labels.iter().all(|label| {
        !label.is_empty()
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label.chars().all(|c| c.is_alphanumeric() || c == '-')
    });
    // A bare address needs more than "www." to stand on: "www.example.com", not "www.notes".
    let bare_ok = !bare
        || (labels.len() >= 3
            && labels
                .last()
                .is_some_and(|tld| tld.len() >= 2 && tld.chars().all(char::is_alphabetic)));
    (valid && bare_ok).then(|| host.to_lowercase())
}

/// An empty port is allowed, as browsers allow it.
fn is_port(port: &str) -> bool {
    port.len() <= 5 && port.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosts() {
        let cases: &[(&str, Option<&str>)] = &[
            ("https://example.com", Some("example.com")),
            ("http://example.com/", Some("example.com")),
            ("HTTPS://Example.COM/Path?q=1#top", Some("example.com")),
            ("https://sub.example.co.uk/a/b", Some("sub.example.co.uk")),
            ("https://user:secret@example.com/", Some("example.com")),
            ("http://localhost:3000/api", Some("localhost")),
            ("http://127.0.0.1:8080", Some("127.0.0.1")),
            ("http://[::1]:8080/", Some("[::1]")),
            ("http://[2001:DB8::1]", Some("[2001:db8::1]")),
            ("https://example.com:/x", Some("example.com")),
            ("https://example.com?q=1", Some("example.com")),
            ("https://bücher.de/", Some("bücher.de")),
            ("ftp://files.example.com/pub", Some("files.example.com")),
            ("www.example.com", Some("www.example.com")),
            ("WWW.Example.com/docs", Some("www.example.com")),
            ("https://example.com/a)b", Some("example.com")),
            // Not links.
            ("", None),
            ("example.com", None),
            ("www.notes", None),
            ("www.example.123", None),
            ("https://", None),
            ("https:///path", None),
            ("https://exa mple.com", None),
            ("see https://example.com", None),
            ("https://example.com and more", None),
            ("https://example.com\nhttps://example.org", None),
            ("mailto:me@example.com", None),
            ("tel:+15551234567", None),
            ("file:///Users/me/a.txt", None),
            ("javascript:alert(1)", None),
            ("slack://open", None),
            ("https://-bad.com", None),
            ("https://a..b", None),
            ("https://example.com:80a", None),
            ("http://[::1", None),
            ("http://[zz]", None),
        ];
        for (text, want) in cases {
            assert_eq!(host(text).as_deref(), *want, "{text:?}");
        }
    }

    #[test]
    fn very_long_text_is_not_a_link() {
        let long = format!("https://example.com/{}", "a".repeat(MAX_BYTES));
        assert_eq!(host(&long), None);
    }
}
