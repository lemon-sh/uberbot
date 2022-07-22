use ellipse::Ellipse;
use serde::Deserialize;
use std::fmt::Write;

#[derive(Deserialize)]
struct WebhookData {
    content: Option<String>,
    username: Option<String>,
    embeds: Vec<Embed>
}

#[derive(Deserialize)]
struct Embed {
    title: Option<String>,
    description: Option<String>,
    url: Option<String>,
    timestamp: Option<String>,
    footer: Option<EmbedFooter>,
    image: Option<UrlObject>,
    thumbnail: Option<UrlObject>,
    video: Option<UrlObject>,
    author: Option<EmbedAuthor>,
    fields: Vec<EmbedField>,
}

#[derive(Deserialize)]
struct UrlObject {
    url: String
}

#[derive(Deserialize)]
struct EmbedAuthor {
    name: String
}

#[derive(Deserialize)]
struct EmbedFooter {
    text: String
}

#[derive(Deserialize)]
struct EmbedField {
    name: String,
    value: String
}

pub fn textify(json: &str, webhook_name: &str) -> anyhow::Result<String> {
    let wh: WebhookData = serde_json::from_str(json)?;
    let mut buf = format!("-- [Webhook: {}]\r\n", wh.username.as_deref().unwrap_or(webhook_name));

    if let Some(content) = wh.content {
        let content = content.trim().truncate_ellipse(450);
        for line in content.lines() {
            write!(&mut buf, " {}\r\n", line)?;
        }
    }
    for embed in wh.embeds {
        write!(&mut buf, "-> {}\r\n", embed.title.as_deref().unwrap_or("Embed"))?;
        if let Some(description) = embed.description {
            let description = description.trim().truncate_ellipse(450);
            for line in description.lines() {
                write!(&mut buf, "  {}\r\n", line)?;
            }
        }
        for field in embed.fields {
            write!(&mut buf, "  + {}\r\n", field.name)?;
            let value = field.value.trim().truncate_ellipse(450);
            for line in value.lines() {
                write!(&mut buf, "   {}\r\n", line)?;
            }
        }
        if let Some(url) = embed.url {
            write!(&mut buf, "  url: {}\r\n", url)?;
        }
        if let Some(image) = embed.image {
            write!(&mut buf, "  img: {}\r\n", image.url)?;
        }
        if let Some(thumbnail) = embed.thumbnail {
            write!(&mut buf, "  thumb: {}\r\n", thumbnail.url)?;
        }
        if let Some(video) = embed.video {
            write!(&mut buf, "  vid: {}\r\n", video.url)?;
        }
        if let Some(author) = embed.author {
            write!(&mut buf, "  by: {}\r\n", author.name)?;
        }
        if let Some(footer) = embed.footer {
            write!(&mut buf, "  - {}\r\n", footer.text)?;
        }
        if let Some(timestamp) = embed.timestamp {
            write!(&mut buf, "  - {}\r\n", timestamp)?;
        }
    }

    buf.push_str("-- end of webhook");
    Ok(buf)
}