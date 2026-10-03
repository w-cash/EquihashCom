//! Social and community links (data/curated/links.json), verified entries only.
use crate::data::{link_kind_label, SocialLink};
use crate::fmt;
use maud::{html, Markup};

pub const ICONS: &str = "/static/icons.svg?v=3";

/// A row of links with small monochrome icons and text labels. Renders nothing when there are no
/// verified links, so a missing kind never leaves an empty icon behind. `links` must already be
/// filtered by `data::verified_links` (the loader does this).
pub fn render(links: &[SocialLink], aria: &str, class: &str) -> Markup {
    if links.is_empty() {
        return html! {};
    }
    // One link per kind up front; further links of a kind already shown sit behind "+N more", so
    // a coin with two websites and two GitHub orgs doesn't wrap the title block onto four lines.
    let mut seen: Vec<&str> = Vec::new();
    let (mut first, mut more): (Vec<&SocialLink>, Vec<&SocialLink>) = (Vec::new(), Vec::new());
    for l in links {
        if seen.contains(&l.kind.as_str()) {
            more.push(l);
        } else {
            seen.push(&l.kind);
            first.push(l);
        }
    }
    html! {
        ul class={"links " (class)} aria-label=(aria) {
            @for l in &first { li { (one(l, false)) } }
            @if !more.is_empty() {
                li class="more" {
                    details {
                        summary { "+" (more.len()) " more" }
                        ul class="links-more" { @for l in &more { li { (one(l, true)) } } }
                    }
                }
            }
        }
    }
}

fn one(l: &SocialLink, with_host: bool) -> Markup {
    let kind = link_kind_label(&l.kind).unwrap_or("Link");
    let host = fmt::host(Some(&l.url));
    let host = host.split('/').next().unwrap_or("");
    html! {
        a href=(l.url) target="_blank" rel="noopener noreferrer" title={(l.label) " · " (host)} data-kind=(l.kind) {
            svg class="ic" aria-hidden="true" focusable="false" { use href={(ICONS) "#i-" (l.kind)} {} }
            span { (kind) @if with_host { " · " (host) } }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::verified_links;

    fn l(kind: &str, url: &str, status: &str) -> SocialLink {
        SocialLink { kind: kind.into(), url: url.into(), label: format!("{kind} label"), status: status.into(), ..Default::default() }
    }

    #[test]
    fn only_verified_known_http_links_render() {
        let raw = vec![
            l("website", "https://z.cash/", "verified"),
            l("discord", "https://discord.gg/abc", "unverified"),
            l("telegram", "https://t.me/dead", "dead"),
            l("myspace", "https://myspace.com/x", "verified"),
            l("x", "javascript:alert(1)", "verified"),
            l("github", "https://github.com/zcash", "verified"),
            l("github", "https://github.com/zcash", "verified"),
            l("forum", "https://forum.zcashcommunity.com/ x", "verified"),
        ];
        let v = verified_links(&raw);
        assert_eq!(v.iter().map(|x| x.kind.as_str()).collect::<Vec<_>>(), vec!["website", "github"]);
        let html = render(&v, "Zcash links", "coin-links").into_string();
        assert!(html.contains("https://z.cash/") && html.contains("https://github.com/zcash"));
        for bad in ["discord.gg", "t.me", "myspace", "javascript:", "forum.zcash"] {
            assert!(!html.contains(bad), "{bad} rendered");
        }
        assert_eq!(html.matches("rel=\"noopener noreferrer\"").count(), 2);
        assert_eq!(html.matches("target=\"_blank\"").count(), 2);
        assert!(html.contains("#i-website") && html.contains("#i-github") && !html.contains("#i-discord"));
    }

    #[test]
    fn duplicate_kinds_fold_behind_more() {
        let v = verified_links(&[
            l("website", "https://z.cash/", "verified"),
            l("website", "https://zfnd.org/", "verified"),
            l("x", "https://x.com/zcash", "verified"),
        ]);
        let html = render(&v, "Zcash links", "coin-links").into_string();
        assert!(html.contains("+1 more"));
        let more = &html[html.find("links-more").unwrap()..];
        assert!(more.contains("zfnd.org") && !more.contains("x.com"));
        assert_eq!(html.matches("rel=\"noopener noreferrer\"").count(), 3, "every link, folded or not, is safe");
    }

    #[test]
    fn no_links_renders_nothing() {
        assert_eq!(render(&[], "x", "y").into_string(), "");
        assert_eq!(render(&verified_links(&[l("x", "https://x.com/a", "unverified")]), "x", "y").into_string(), "");
    }

    #[test]
    fn link_text_is_escaped() {
        let mut a = l("website", "https://example.com/?a=1&b=\"2\"", "verified");
        a.label = "<script>".into();
        let html = render(&verified_links(&[a]), "x", "y").into_string();
        assert!(!html.contains("<script>") && html.contains("&lt;script&gt;"));
    }
}
